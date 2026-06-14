//! Streaming HTTP de rádio (Icecast/Shoutcast) com extração de metadata ICY.
//!
//! Uma thread de rede lê o corpo da resposta, deixa o [`icy-metadata`] separar
//! os blocos ICY (now playing) e empurra só o áudio para um ring buffer de
//! bytes. O [`RingReader`] — que é `Send + Sync` — alimenta o symphonia como
//! qualquer `MediaSource`, desacoplando rede de decodificação e resolvendo o
//! `Sync` que o reader do HTTP não garante.

use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use icy_metadata::IcyMetadataReader;
use symphonia::core::io::MediaSource;
use tokio::sync::mpsc;

use super::AudioEvent;

const UA: &str = concat!("lavanda/", env!("CARGO_PKG_VERSION"));
/// Limite do buffer de áudio em bytes (~1 MiB) — aplica backpressure na rede.
const RING_CAP: usize = 1 << 20;
const CHUNK: usize = 8192;

struct Ring {
    buf: Mutex<VecDeque<u8>>,
    cond: Condvar,
    done: AtomicBool,
}

/// Fonte de mídia para o symphonia, alimentada pela thread de rede.
pub struct RingReader {
    ring: Arc<Ring>,
    cancel: Arc<AtomicBool>,
}

impl Read for RingReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let mut buf = self.ring.buf.lock().unwrap();
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                return Ok(0); // EOF → encerra o loop de decode
            }
            if !buf.is_empty() {
                let n = buf.len().min(out.len());
                for (slot, b) in out.iter_mut().zip(buf.drain(..n)) {
                    *slot = b;
                }
                self.ring.cond.notify_all(); // libera espaço para a rede
                return Ok(n);
            }
            if self.ring.done.load(Ordering::SeqCst) {
                return Ok(0); // rede terminou e buffer vazio
            }
            let (g, _) = self
                .ring
                .cond
                .wait_timeout(buf, Duration::from_millis(200))
                .unwrap();
            buf = g;
        }
    }
}

impl Seek for RingReader {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "stream de rádio não é buscável",
        ))
    }
}

impl MediaSource for RingReader {
    fn is_seekable(&self) -> bool {
        false
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// Abre a conexão, dispara a thread de rede e devolve o reader + o codec
/// Diagnóstico do servidor, útil para classificar falhas de decode.
pub struct StreamInfo {
    pub content_type: String,
    pub final_url: String,
}

/// Abre a conexão, dispara a thread de rede e devolve o reader + diagnóstico.
pub fn open(
    url: &str,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
    cancel: Arc<AtomicBool>,
) -> Result<(RingReader, StreamInfo)> {
    // Stream infinito: NÃO usar timeout geral (ele mataria a conexão e o áudio
    // pararia ao esvaziar o buffer). Só timeout de conexão e de leitura por
    // chamada — este não dispara enquanto a estação envia dados.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build();

    // Muitas estações do diretório apontam para um arquivo de playlist
    // (.pls/.m3u) em vez do stream direto; resolvemos antes de decodificar.
    let resp = connect_resolving(&agent, url, 0)?;

    let info = StreamInfo {
        content_type: resp.header("Content-Type").unwrap_or("").to_string(),
        final_url: resp.get_url().to_string(),
    };

    let metaint = resp
        .header("icy-metaint")
        .and_then(|s| s.parse::<usize>().ok())
        .and_then(NonZeroUsize::new);

    // Wrap do corpo HTTP: FillReader garante leituras completas (o
    // IcyMetadataReader 0.6.0 panica com leituras parciais de rede); o
    // icy-metadata separa os blocos ICY e dispara o callback com o título.
    let title_tx = event_tx.clone();
    let last_title: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let reader = IcyMetadataReader::new(FillReader(resp.into_reader()), metaint, move |meta| {
        if let Ok(meta) = meta {
            if let Some(title) = meta.stream_title() {
                let title = title.trim().to_string();
                let mut last = last_title.lock().unwrap();
                if !title.is_empty() && last.as_deref() != Some(title.as_str()) {
                    *last = Some(title.clone());
                    let _ = title_tx.send(AudioEvent::StreamTitle(title));
                }
            }
        }
    });

    let ring = Arc::new(Ring {
        buf: Mutex::new(VecDeque::new()),
        cond: Condvar::new(),
        done: AtomicBool::new(false),
    });

    let ring_net = ring.clone();
    let cancel_net = cancel.clone();
    std::thread::spawn(move || {
        pump(reader, &ring_net, &cancel_net);
        ring_net.done.store(true, Ordering::SeqCst);
        ring_net.cond.notify_all();
    });

    Ok((RingReader { ring, cancel }, info))
}

/// Adaptador que preenche o buffer inteiro (até EOF). O `IcyMetadataReader`
/// 0.6.0 assume leituras completas: com leitura parcial de rede ele acaba
/// chamando `buf[..metaint]` num buffer menor e panica. Isto garante o contrato.
struct FillReader<R>(R);

impl<R: Read> Read for FillReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.0.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(filled)
    }
}

/// Faz GET com `Icy-MetaData`; se a resposta for uma playlist (.pls/.m3u),
/// extrai a primeira URL de stream e segue (até 2 níveis). HLS (.m3u8) não é
/// suportado e gera erro claro.
fn connect_resolving(
    agent: &ureq::Agent,
    url: &str,
    depth: u8,
) -> Result<ureq::Response> {
    let resp = agent
        .get(url)
        .set("Icy-MetaData", "1")
        .set("User-Agent", UA)
        .call()
        .map_err(|e| anyhow!("conexão falhou: {e}"))?;

    let ct = resp.header("Content-Type").unwrap_or("").to_string();
    if depth < 2 && looks_like_playlist(&ct, url) {
        let body = resp
            .into_string()
            .map_err(|e| anyhow!("ler playlist: {e}"))?;
        return match parse_playlist(&body) {
            Some(next) => connect_resolving(agent, &next, depth + 1),
            None => Err(anyhow!("playlist sem stream tocável (HLS não é suportado)")),
        };
    }

    Ok(resp)
}

fn looks_like_playlist(content_type: &str, url: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("mpegurl") || ct.contains("scpls") || ct.contains("pls+xml") || ct.contains("uri-list") {
        return true;
    }
    // Áudio de verdade nunca é playlist.
    if ct.starts_with("audio/") || ct.starts_with("application/ogg") {
        return false;
    }
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    path.ends_with(".m3u") || path.ends_with(".m3u8") || path.ends_with(".pls")
}

/// Extrai a primeira URL http(s) de uma playlist .pls (`FileN=URL`) ou .m3u
/// (URLs em linhas). Retorna `None` para HLS (`#EXT-X-`).
fn parse_playlist(body: &str) -> Option<String> {
    if body.contains("#EXT-X-") {
        return None; // HLS — segmentado, fora do alcance do symphonia
    }
    for line in body.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with(';') || l.starts_with('[') {
            continue;
        }
        // .pls: Chave=URL
        if let Some((_, v)) = l.split_once('=') {
            let v = v.trim();
            if v.starts_with("http://") || v.starts_with("https://") {
                return Some(v.to_string());
            }
        }
        // .m3u: URL nua
        if l.starts_with("http://") || l.starts_with("https://") {
            return Some(l.to_string());
        }
    }
    None
}

/// Lê áudio já limpo (sem ICY) e alimenta o ring com backpressure.
fn pump(mut reader: impl Read, ring: &Arc<Ring>, cancel: &Arc<AtomicBool>) {
    let mut tmp = [0u8; CHUNK];
    loop {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        match reader.read(&mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => push(ring, cancel, &tmp[..n]),
        }
    }
}

/// Empurra bytes de áudio para o ring, esperando se estiver cheio (backpressure).
fn push(ring: &Arc<Ring>, cancel: &Arc<AtomicBool>, data: &[u8]) {
    let mut buf = ring.buf.lock().unwrap();
    loop {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        if buf.len() < RING_CAP {
            break;
        }
        let (g, _) = ring
            .cond
            .wait_timeout(buf, Duration::from_millis(200))
            .unwrap();
        buf = g;
    }
    buf.extend(data.iter().copied());
    ring.cond.notify_all();
}
