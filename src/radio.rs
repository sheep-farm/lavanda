//! Cliente do diretório comunitário [radio-browser.info](https://www.radio-browser.info/).
//!
//! Apenas busca/listagem de estações. O streaming de áudio (Icecast/Shoutcast
//! + metadata ICY) fica em [`crate::audio`].

use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Mirrors da API. `all.api...` faz round-robin via DNS; os demais são fallback.
const MIRRORS: &[&str] = &[
    "https://all.api.radio-browser.info",
    "https://de1.api.radio-browser.info",
    "https://nl1.api.radio-browser.info",
];

const UA: &str = concat!("lavanda/", env!("CARGO_PKG_VERSION"));

/// Uma estação retornada pela API (campos relevantes; o resto é ignorado).
/// Também é o que persistimos como favorito em `db.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RadioStation {
    #[serde(default)]
    pub stationuuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub url_resolved: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub favicon: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub countrycode: String,
    #[serde(default)]
    pub codec: String,
    #[serde(default)]
    pub bitrate: u32,
    #[serde(default)]
    pub votes: u64,
    #[serde(default)]
    pub clickcount: u64,
    /// 1 = stream HLS (segmentado) — o symphonia não decodifica; filtramos fora.
    #[serde(default)]
    pub hls: u8,
}

impl RadioStation {
    /// URL a ser tocada: prefere a já resolvida pelo servidor.
    pub fn stream_url(&self) -> &str {
        if !self.url_resolved.is_empty() {
            &self.url_resolved
        } else {
            &self.url
        }
    }
}

fn call(path: &str, queries: &[(&str, &str)]) -> Result<Vec<RadioStation>> {
    let mut last_err = String::from("nenhum mirror tentado");
    for base in MIRRORS {
        let url = format!("{base}{path}");
        let mut req = ureq::get(&url)
            .set("User-Agent", UA)
            .timeout(Duration::from_secs(12));
        for (k, v) in queries {
            req = req.query(k, v);
        }
        match req.call() {
            Ok(resp) => {
                let mut stations = resp
                    .into_json::<Vec<RadioStation>>()
                    .map_err(|e| anyhow!("resposta inválida: {e}"))?;
                // HLS é segmentado e não decodificável pelo symphonia: esconde.
                stations.retain(|s| s.hls == 0 && !s.stream_url().is_empty());
                return Ok(stations);
            }
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(anyhow!("radio-browser indisponível: {last_err}"))
}

/// Busca estações por nome, ordenadas por popularidade (clickcount).
pub fn search(query: &str) -> Result<Vec<RadioStation>> {
    call(
        "/json/stations/search",
        &[
            ("name", query),
            ("limit", "100"),
            ("hidebroken", "true"),
            ("order", "clickcount"),
            ("reverse", "true"),
        ],
    )
}

/// Estações mais clicadas no momento (tela inicial da aba Radios).
pub fn top(limit: usize) -> Result<Vec<RadioStation>> {
    let limit = limit.to_string();
    call(
        "/json/stations/search",
        &[
            ("limit", &limit),
            ("hidebroken", "true"),
            ("order", "clickcount"),
            ("reverse", "true"),
        ],
    )
}

/// Conectividade: tenta resolver e conectar ao diretório (sinal mais relevante
/// para a aba Radios). Bloqueante — rodar fora da thread de UI.
pub fn is_online() -> bool {
    use std::net::ToSocketAddrs;
    let Ok(addrs) = ("all.api.radio-browser.info", 443).to_socket_addrs() else {
        return false;
    };
    for addr in addrs {
        if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(4)).is_ok() {
            return true;
        }
    }
    false
}

/// Registra um "click" de reprodução (educado com o diretório; falha em silêncio).
/// Estações do SomaFM (`stationuuid` começa com `somafm:`) não existem no
/// radio-browser, então não há o que registrar.
pub fn register_click(uuid: &str) {
    if uuid.is_empty() || uuid.starts_with("somafm:") {
        return;
    }
    for base in MIRRORS {
        let url = format!("{base}/json/url/{uuid}");
        if ureq::get(&url)
            .set("User-Agent", UA)
            .timeout(Duration::from_secs(5))
            .call()
            .is_ok()
        {
            return;
        }
    }
}

// ── SomaFM (diretório curado, complementar ao radio-browser) ─────────────────
//
// Lista enxuta e bem mantida de estações Icecast com `StreamTitle` ICY. Mapeada
// para `RadioStation` para reaproveitar todo o pipeline (streaming, favoritos,
// quarentena, UI). Preferimos sempre o stream MP3 de maior qualidade: os
// formatos `aac`/`aacp` do SomaFM são HE-AAC, que o symphonia não decodifica.

const SOMAFM_URL: &str = "https://somafm.com/channels.json";

#[derive(Deserialize)]
struct SomaChannels {
    channels: Vec<SomaChannel>,
}

#[derive(Deserialize)]
struct SomaChannel {
    id: String,
    title: String,
    #[serde(default)]
    genre: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    playlists: Vec<SomaPlaylist>,
}

#[derive(Deserialize)]
struct SomaPlaylist {
    url: String,
    format: String,
    quality: String,
}

fn quality_rank(q: &str) -> u8 {
    match q {
        "highest" => 3,
        "high" => 2,
        "low" => 1,
        _ => 0,
    }
}

impl SomaChannel {
    fn into_station(self) -> Option<RadioStation> {
        // Melhor stream MP3; ignora aac/aacp (HE-AAC não decodificável).
        let pls = self
            .playlists
            .iter()
            .filter(|p| p.format == "mp3")
            .max_by_key(|p| quality_rank(&p.quality))?;
        Some(RadioStation {
            // Chave sintética: não é UUID do radio-browser (ver `register_click`).
            stationuuid: format!("somafm:{}", self.id),
            name: format!("SomaFM · {}", self.title),
            url: pls.url.clone(), // .pls — resolvido por crate::audio::stream
            homepage: format!("https://somafm.com/{}/", self.id),
            favicon: self.image,
            tags: self.genre.replace('|', ","),
            codec: "MP3".into(),
            ..Default::default()
        })
    }
}

/// Lista curada do SomaFM. Mesmo tipo de retorno que [`top`]/[`search`].
pub fn somafm() -> Result<Vec<RadioStation>> {
    let resp = ureq::get(SOMAFM_URL)
        .set("User-Agent", UA)
        .timeout(Duration::from_secs(12))
        .call()
        .map_err(|e| anyhow!("SomaFM indisponível: {e}"))?;
    let data: SomaChannels = resp
        .into_json()
        .map_err(|e| anyhow!("SomaFM: resposta inválida: {e}"))?;
    Ok(data
        .channels
        .into_iter()
        .filter_map(SomaChannel::into_station)
        .collect())
}
