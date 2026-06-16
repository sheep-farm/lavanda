use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time as SymphoniaTime;
use tokio::sync::mpsc;

const OUTPUT_RATE: u32 = 48000;
const OUTPUT_CHANNELS: u16 = 2;

#[derive(Debug, Clone)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug)]
pub enum AudioCommand {
    Play(PathBuf),
    /// Toca um stream de rádio (URL + dica de codec do diretório).
    PlayStream {
        url: String,
        codec: String,
    },
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    /// Próxima faixa a tocar quando a atual terminar naturalmente (gapless).
    /// `None` = parar ao fim. A UI mantém isto atualizado conforme a fila/modo.
    SetNext(Option<PathBuf>),
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    Playing,
    Paused,
    Stopped,
    Progress {
        position: Duration,
        duration: Duration,
    },
    /// Título "now playing" de um stream de rádio (metadata ICY).
    StreamTitle(String),
    Error(String),
    /// Fim da fila (sem próxima): a reprodução parou.
    TrackEnded,
    /// Encadeou para a próxima faixa sem cortar o áudio (gapless). A UI atualiza
    /// a faixa atual sem reenviar `Play`.
    TrackAdvanced(PathBuf),
}

pub struct AudioPlayer {
    pub cmd_tx: mpsc::UnboundedSender<AudioCommand>,
    pub viz_buf: Arc<Mutex<VecDeque<f32>>>,
    event_rx: Option<mpsc::UnboundedReceiver<AudioEvent>>,
}

impl AudioPlayer {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let viz_buf: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));

        let viz = viz_buf.clone();
        std::thread::spawn(move || audio_thread(cmd_rx, event_tx, viz));

        AudioPlayer {
            cmd_tx,
            viz_buf,
            event_rx: Some(event_rx),
        }
    }

    pub fn send(&self, cmd: AudioCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// Entrega o receptor de eventos uma única vez, para ser consumido pela
    /// subscription dirigida por canal.
    pub fn take_events(&mut self) -> mpsc::UnboundedReceiver<AudioEvent> {
        self.event_rx.take().expect("event receiver already taken")
    }
}

// ── Thread de áudio ──────────────────────────────────────────────────────────

fn audio_thread(
    mut cmd_rx: mpsc::UnboundedReceiver<AudioCommand>,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
    viz_buf: Arc<Mutex<VecDeque<f32>>>,
) {
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            let _ = event_tx.send(AudioEvent::Error("Nenhum dispositivo de áudio".into()));
            return;
        }
    };

    let sample_format = device
        .default_output_config()
        .map(|c| c.sample_format())
        .unwrap_or(SampleFormat::F32);

    let stream_config = StreamConfig {
        channels: OUTPUT_CHANNELS,
        sample_rate: SampleRate(OUTPUT_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let pcm: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::with_capacity(
        OUTPUT_RATE as usize * 2,
    )));

    // Compartilhados entre fill_output e o loop de comandos
    let paused: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let shared_vol: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.8));

    let pcm_cb = pcm.clone();
    let paused_cb = paused.clone();
    let err_fn = |e| eprintln!("Erro stream: {e}");

    let stream = match sample_format {
        SampleFormat::I16 => {
            let pcm2 = pcm.clone();
            let paused2 = paused.clone();
            device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _| {
                    let mut tmp = vec![0f32; data.len()];
                    fill_output(&mut tmp, &pcm2, &paused2);
                    for (d, s) in data.iter_mut().zip(tmp.iter()) {
                        *d = cpal::Sample::from_sample(*s);
                    }
                },
                err_fn,
                None,
            )
        }
        _ => device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _| fill_output(data, &pcm_cb, &paused_cb),
            err_fn,
            None,
        ),
    };

    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            let _ = event_tx.send(AudioEvent::Error(format!("Build stream: {e}")));
            return;
        }
    };

    if let Err(e) = stream.play() {
        let _ = event_tx.send(AudioEvent::Error(format!("Stream play: {e}")));
        return;
    }

    let mut cancel: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    // Compartilhado: a cadeia gapless (em outra task) atualiza a faixa atual,
    // e Seek/Resume precisam ler a que está realmente tocando.
    let current_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    // Próxima faixa pré-carregada para o encadeamento sem cortes.
    let next_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    let mut current_stream: Option<String> = None;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        loop {
            match cmd_rx.recv().await {
                None => break,

                Some(AudioCommand::Stop) => {
                    cancel.store(true, Ordering::SeqCst);
                    paused.store(false, Ordering::SeqCst);
                    pcm.lock().unwrap().clear();
                    *current_path.lock().unwrap() = None;
                    *next_path.lock().unwrap() = None;
                    current_stream = None;
                    let _ = event_tx.send(AudioEvent::Stopped);
                }

                Some(AudioCommand::SetNext(p)) => {
                    *next_path.lock().unwrap() = p;
                }

                Some(AudioCommand::Pause) => {
                    paused.store(true, Ordering::SeqCst);
                    let _ = event_tx.send(AudioEvent::Paused);
                }

                Some(AudioCommand::Resume) => {
                    paused.store(false, Ordering::SeqCst);
                    if current_path.lock().unwrap().is_some() || current_stream.is_some() {
                        let _ = event_tx.send(AudioEvent::Playing);
                    }
                }

                Some(AudioCommand::SetVolume(v)) => {
                    *shared_vol.lock().unwrap() = v.clamp(0.0, 1.0);
                }

                Some(AudioCommand::Seek(pos)) => {
                    let path = current_path.lock().unwrap().clone();
                    if let Some(path) = path {
                        cancel.store(true, Ordering::SeqCst);
                        paused.store(false, Ordering::SeqCst);
                        pcm.lock().unwrap().clear();

                        let new_cancel = Arc::new(AtomicBool::new(false));
                        cancel = new_cancel.clone();

                        // Mantém next_path: ao terminar, o encadeamento segue normal.
                        let np = next_path.clone();
                        let cp = current_path.clone();
                        let pcm2 = pcm.clone();
                        let tx = event_tx.clone();
                        let vol = shared_vol.clone();
                        let flag = new_cancel;
                        let viz = viz_buf.clone();

                        tokio::task::spawn_blocking(move || {
                            play_chain(path, Some(pos), cp, np, pcm2, tx, vol, flag, viz);
                        });
                    }
                }

                Some(AudioCommand::Play(path)) => {
                    cancel.store(true, Ordering::SeqCst);
                    paused.store(false, Ordering::SeqCst);
                    pcm.lock().unwrap().clear();
                    // Salto explícito do usuário: descarta a próxima pré-carregada
                    // (a UI reenvia SetNext em seguida).
                    *next_path.lock().unwrap() = None;

                    let new_cancel = Arc::new(AtomicBool::new(false));
                    cancel = new_cancel.clone();
                    *current_path.lock().unwrap() = Some(path.clone());
                    current_stream = None;

                    let _ = event_tx.send(AudioEvent::Playing);

                    let np = next_path.clone();
                    let cp = current_path.clone();
                    let pcm2 = pcm.clone();
                    let tx = event_tx.clone();
                    let vol = shared_vol.clone();
                    let flag = new_cancel;
                    let viz = viz_buf.clone();

                    tokio::task::spawn_blocking(move || {
                        play_chain(path, None, cp, np, pcm2, tx, vol, flag, viz);
                    });
                }

                Some(AudioCommand::PlayStream { url, codec }) => {
                    cancel.store(true, Ordering::SeqCst);
                    paused.store(false, Ordering::SeqCst);
                    pcm.lock().unwrap().clear();
                    *next_path.lock().unwrap() = None;

                    let new_cancel = Arc::new(AtomicBool::new(false));
                    cancel = new_cancel.clone();
                    *current_path.lock().unwrap() = None;
                    current_stream = Some(url.clone());

                    let _ = event_tx.send(AudioEvent::Playing);

                    let pcm2 = pcm.clone();
                    let tx = event_tx.clone();
                    let vol = shared_vol.clone();
                    let flag = new_cancel;
                    let viz = viz_buf.clone();

                    tokio::task::spawn_blocking(move || {
                        match decode_stream(&url, &codec, pcm2, tx.clone(), vol, flag, viz) {
                            Ok(_) => {
                                let _ = tx.send(AudioEvent::Stopped);
                            }
                            Err(e) => {
                                let _ = tx.send(AudioEvent::Error(e.to_string()));
                            }
                        }
                    });
                }
            }
        }
    });
}

/// Toca `initial` e, ao terminar naturalmente, encadeia a próxima faixa
/// (`next_path`) **no mesmo buffer, sem `clear()`** — eis o gapless. Roda numa
/// task bloqueante dedicada; `cancel` interrompe (novo Play/Seek/Stop).
#[allow(clippy::too_many_arguments)]
fn play_chain(
    initial: PathBuf,
    seek_to: Option<Duration>,
    current_path: Arc<Mutex<Option<PathBuf>>>,
    next_path: Arc<Mutex<Option<PathBuf>>>,
    pcm: Arc<Mutex<VecDeque<f32>>>,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
    volume: Arc<Mutex<f32>>,
    cancel: Arc<AtomicBool>,
    viz_buf: Arc<Mutex<VecDeque<f32>>>,
) {
    let mut path = initial;
    let mut seek = seek_to;
    loop {
        match decode_file(
            &path,
            pcm.clone(),
            event_tx.clone(),
            volume.clone(),
            cancel.clone(),
            viz_buf.clone(),
            seek,
        ) {
            // Fim natural: tenta a próxima pré-carregada sem cortar o áudio.
            Ok(true) => {
                let nxt = next_path.lock().unwrap().take();
                match nxt {
                    Some(p) => {
                        *current_path.lock().unwrap() = Some(p.clone());
                        let _ = event_tx.send(AudioEvent::TrackAdvanced(p.clone()));
                        path = p;
                        seek = None;
                        continue;
                    }
                    None => {
                        let _ = event_tx.send(AudioEvent::TrackEnded);
                        break;
                    }
                }
            }
            // Cancelado por novo comando: encerra em silêncio.
            Ok(false) => break,
            Err(e) => {
                let _ = event_tx.send(AudioEvent::Error(e.to_string()));
                break;
            }
        }
    }
}

fn fill_output(output: &mut [f32], pcm: &Arc<Mutex<VecDeque<f32>>>, paused: &Arc<AtomicBool>) {
    if paused.load(Ordering::SeqCst) {
        for s in output.iter_mut() {
            *s = 0.0;
        }
        return;
    }
    let mut buf = pcm.lock().unwrap();
    for sample in output.iter_mut() {
        *sample = buf.pop_front().unwrap_or(0.0);
    }
}

// ── Decode ───────────────────────────────────────────────────────────────────

const VIZ_BUF_CAP: usize = 8192;

/// Retorna Ok(true) se a faixa terminou normalmente, Ok(false) se foi cancelada.
fn decode_file(
    path: &PathBuf,
    pcm: Arc<Mutex<VecDeque<f32>>>,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
    volume: Arc<Mutex<f32>>,
    cancel: Arc<AtomicBool>,
    viz_buf: Arc<Mutex<VecDeque<f32>>>,
    seek_to: Option<Duration>,
) -> Result<bool> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let fmt_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &MetadataOptions::default())
        .map_err(|e| anyhow!("Formato não suportado: {e}"))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("Nenhuma faixa de áudio"))?;

    let track_id = track.id;
    let time_base = track.codec_params.time_base;
    let n_frames = track.codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| anyhow!("Decoder: {e}"))?;

    // Emite Progress no máximo a cada 250 ms: o suficiente para a barra e o
    // relógio, sem inundar a UI com um evento por packet (~38/s em MP3).
    let mut next_emit = Duration::ZERO;

    let mut sample_count = if let Some(pos) = seek_to {
        let seek_time = SymphoniaTime {
            seconds: pos.as_secs(),
            frac: pos.subsec_nanos() as f64 / 1_000_000_000.0,
        };
        format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: seek_time,
                    track_id: None,
                },
            )
            .ok();
        decoder.reset();
        (pos.as_secs_f64() * OUTPUT_RATE as f64) as u64
    } else {
        0u64
    };

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) => break,
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(anyhow!("Packet: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("Decode: {e}")),
        };

        let file_rate = decoded.spec().rate;
        let vol = *volume.lock().unwrap();
        let samples = decoded_to_output(decoded, file_rate, vol, &viz_buf);

        sample_count += samples.len() as u64 / 2;

        if let (Some(tb), Some(nf)) = (time_base, n_frames) {
            let position = Duration::from_secs_f64(sample_count as f64 / OUTPUT_RATE as f64);
            if position >= next_emit {
                let duration =
                    Duration::from_secs_f64(nf as f64 * tb.numer as f64 / tb.denom as f64);
                let _ = event_tx.send(AudioEvent::Progress { position, duration });
                next_emit = position + Duration::from_millis(250);
            }
        }

        if !push_pcm(&pcm, &cancel, samples) {
            return Ok(false);
        }
    }

    Ok(true)
}

// ── Decode de stream de rádio ──────────────────────────────────────────────────

/// Decodifica um stream HTTP infinito. Sem seek e sem duração; emite Progress
/// com `duration = ZERO` (a UI trata isso como "ao vivo").
fn decode_stream(
    url: &str,
    codec: &str,
    pcm: Arc<Mutex<VecDeque<f32>>>,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
    volume: Arc<Mutex<f32>>,
    cancel: Arc<AtomicBool>,
    viz_buf: Arc<Mutex<VecDeque<f32>>>,
) -> Result<()> {
    let (source, info) = super::stream::open(url, event_tx.clone(), cancel.clone())?;
    let mss = MediaSourceStream::new(Box::new(source), Default::default());

    // Dica de formato: prefere o Content-Type do servidor, cai para o codec do diretório.
    let mut hint = Hint::new();
    let ct = info.content_type.to_ascii_lowercase();
    let ext = if ct.contains("mpeg") || ct.contains("mp3") {
        Some("mp3")
    } else if ct.contains("aac") {
        Some("aac")
    } else if ct.contains("ogg") || ct.contains("opus") {
        Some("ogg")
    } else if ct.contains("flac") {
        Some("flac")
    } else {
        match codec.to_ascii_lowercase().as_str() {
            "mp3" => Some("mp3"),
            "aac" | "aac+" | "aacp" => Some("aac"),
            "ogg" | "vorbis" | "opus" => Some("ogg"),
            "flac" => Some("flac"),
            _ => None,
        }
    };
    if let Some(e) = ext {
        hint.with_extension(e);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| {
            anyhow!(
                "formato não reconhecido (codec={codec}, content-type=\"{}\", stream={}): {e}",
                info.content_type,
                info.final_url
            )
        })?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("Nenhuma faixa de áudio"))?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| anyhow!("Decoder: {e}"))?;

    let mut sample_count = 0u64;
    let mut next_emit = Duration::ZERO;

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Ok(());
        }

        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) => return Ok(()),
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(anyhow!("Packet: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("Decode: {e}")),
        };

        let file_rate = decoded.spec().rate;
        let vol = *volume.lock().unwrap();
        let samples = decoded_to_output(decoded, file_rate, vol, &viz_buf);

        sample_count += samples.len() as u64 / 2;
        let position = Duration::from_secs_f64(sample_count as f64 / OUTPUT_RATE as f64);
        if position >= next_emit {
            let _ = event_tx.send(AudioEvent::Progress {
                position,
                duration: Duration::ZERO,
            });
            next_emit = position + Duration::from_millis(500);
        }

        if !push_pcm(&pcm, &cancel, samples) {
            return Ok(());
        }
    }
}

// ── Helpers de decode (compartilhados entre arquivo e stream) ───────────────────

/// Converte um buffer decodificado em PCM estéreo a `OUTPUT_RATE`, aplica volume
/// e alimenta o buffer do visualizador.
fn decoded_to_output(
    decoded: symphonia::core::audio::AudioBufferRef,
    file_rate: u32,
    vol: f32,
    viz_buf: &Arc<Mutex<VecDeque<f32>>>,
) -> Vec<f32> {
    let spec = *decoded.spec();
    let n_channels = spec.channels.count();

    let mut conv = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
    conv.copy_interleaved_ref(decoded);
    let raw = conv.samples();

    let stereo: Vec<f32> = match n_channels {
        1 => raw.iter().flat_map(|&s| [s * vol, s * vol]).collect(),
        2 => raw.iter().map(|&s| s * vol).collect(),
        n => raw
            .chunks(n)
            .flat_map(|ch| {
                let l = ch.first().copied().unwrap_or(0.0) * vol;
                let r = ch.get(1).copied().unwrap_or(0.0) * vol;
                [l, r]
            })
            .collect(),
    };

    let samples = if file_rate != OUTPUT_RATE {
        resample_stereo(&stereo, file_rate, OUTPUT_RATE)
    } else {
        stereo
    };

    {
        let mut vb = viz_buf.lock().unwrap();
        for ch in samples.chunks(2) {
            let mono = (ch[0] + ch.get(1).copied().unwrap_or(ch[0])) * 0.5;
            vb.push_back(mono);
        }
        while vb.len() > VIZ_BUF_CAP {
            vb.pop_front();
        }
    }

    samples
}

/// Empurra PCM para o buffer de saída com backpressure. Retorna `false` se
/// cancelado durante a espera.
fn push_pcm(pcm: &Arc<Mutex<VecDeque<f32>>>, cancel: &Arc<AtomicBool>, samples: Vec<f32>) -> bool {
    loop {
        if cancel.load(Ordering::SeqCst) {
            return false;
        }
        if pcm.lock().unwrap().len() < OUTPUT_RATE as usize * 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    pcm.lock().unwrap().extend(samples);
    true
}

fn resample_stereo(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    let ratio = in_rate as f64 / out_rate as f64;
    let in_frames = input.len() / 2;
    let out_frames = (in_frames as f64 / ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_frames * 2);

    for i in 0..out_frames {
        let src = i as f64 * ratio;
        let idx = src as usize;
        let frac = (src - idx as f64) as f32;

        let l0 = input.get(idx * 2).copied().unwrap_or(0.0);
        let l1 = input.get(idx * 2 + 2).copied().unwrap_or(l0);
        let r0 = input.get(idx * 2 + 1).copied().unwrap_or(0.0);
        let r1 = input.get(idx * 2 + 3).copied().unwrap_or(r0);

        out.push(l0 + (l1 - l0) * frac);
        out.push(r0 + (r1 - r0) * frac);
    }

    out
}
