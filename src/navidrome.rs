use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Result};

const UA: &str = concat!("lavanda/", env!("CARGO_PKG_VERSION"));
const API_VERSION: &str = "1.16.1";
const CLIENT: &str = "lavanda";

fn hex_encode(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

fn auth(user: &str, password: &str) -> String {
    format!(
        "u={}&p=enc:{}&v={}&c={}&f=json",
        user,
        hex_encode(password),
        API_VERSION,
        CLIENT,
    )
}

fn get(url: &str) -> Result<serde_json::Value> {
    let resp = ureq::get(url)
        .set("User-Agent", UA)
        .call()
        .map_err(|e| anyhow!("{e}"))?;
    let json: serde_json::Value = resp.into_json()?;
    let inner = &json["subsonic-response"];
    if inner["status"].as_str() != Some("ok") {
        let msg = inner["error"]["message"]
            .as_str()
            .unwrap_or("unknown error");
        return Err(anyhow!("{msg}"));
    }
    Ok(inner.clone())
}

#[derive(Debug, Clone)]
pub struct NavidromeItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct NavidromeTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration: Duration,
    pub genre: String,
    pub year: Option<u32>,
}

impl NavidromeTrack {
    pub fn to_track(&self) -> crate::library::models::Track {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.id.hash(&mut h);
        crate::library::models::Track {
            id: h.finish() as i64,
            path: std::path::PathBuf::from(format!("navidrome://{}", self.id)),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            track_number: self.track_number,
            disc_number: self.disc_number,
            duration: self.duration,
            cover_data: None,
            genre: self.genre.clone(),
            year: self.year,
            play_count: 0,
            liked: false,
            date_played: None,
        }
    }
}

pub fn is_navidrome_path(path: &Path) -> bool {
    path.to_str().map_or(false, |s| s.starts_with("navidrome://"))
}


pub fn stream_url(base_url: &str, song_id: &str, user: &str, password: &str) -> String {
    format!(
        "{}/rest/stream.view?id={}&{}",
        base_url.trim_end_matches('/'),
        song_id,
        auth(user, password),
    )
}

pub fn fetch_artists(base_url: &str, user: &str, password: &str) -> Result<Vec<NavidromeItem>> {
    let url = format!(
        "{}/rest/getArtists.view?{}",
        base_url.trim_end_matches('/'),
        auth(user, password),
    );
    let resp = get(&url)?;
    // Artistas agrupados por letra de índice; achatamos a lista.
    let indexes = resp["artists"]["index"]
        .as_array()
        .ok_or_else(|| anyhow!("resposta sem artists.index"))?;
    let mut result = Vec::new();
    for index in indexes {
        if let Some(artists) = index["artist"].as_array() {
            for artist in artists {
                if let (Some(id), Some(name)) =
                    (artist["id"].as_str(), artist["name"].as_str())
                {
                    result.push(NavidromeItem {
                        id: id.to_string(),
                        name: name.to_string(),
                    });
                }
            }
        }
    }
    Ok(result)
}

pub fn fetch_albums(
    base_url: &str,
    user: &str,
    password: &str,
    artist_id: &str,
) -> Result<Vec<NavidromeItem>> {
    let url = format!(
        "{}/rest/getArtist.view?id={}&{}",
        base_url.trim_end_matches('/'),
        artist_id,
        auth(user, password),
    );
    let resp = get(&url)?;
    let albums = resp["artist"]["album"]
        .as_array()
        .ok_or_else(|| anyhow!("resposta sem artist.album"))?;
    Ok(albums
        .iter()
        .filter_map(|album| {
            let id = album["id"].as_str()?.to_string();
            let name = album["name"].as_str()?.to_string();
            Some(NavidromeItem { id, name })
        })
        .collect())
}

pub fn fetch_tracks(
    base_url: &str,
    user: &str,
    password: &str,
    album_id: &str,
) -> Result<Vec<NavidromeTrack>> {
    let url = format!(
        "{}/rest/getAlbum.view?id={}&{}",
        base_url.trim_end_matches('/'),
        album_id,
        auth(user, password),
    );
    let resp = get(&url)?;
    let songs = resp["album"]["song"]
        .as_array()
        .ok_or_else(|| anyhow!("resposta sem album.song"))?;
    Ok(songs
        .iter()
        .filter_map(|song| {
            let id = song["id"].as_str()?.to_string();
            let title = song["title"].as_str().unwrap_or("Unknown").to_string();
            let artist = song["artist"].as_str().unwrap_or("Unknown").to_string();
            let album = song["album"].as_str().unwrap_or("Unknown").to_string();
            let track_number = song["track"].as_u64().map(|n| n as u32);
            let disc_number = song["discNumber"].as_u64().map(|n| n as u32);
            let year = song["year"].as_u64().map(|n| n as u32);
            let genre = song["genre"].as_str().unwrap_or("").to_string();
            // Subsonic: duração em segundos
            let secs = song["duration"].as_u64().unwrap_or(0);
            let duration = Duration::from_secs(secs);
            Some(NavidromeTrack {
                id,
                title,
                artist,
                album,
                track_number,
                disc_number,
                duration,
                genre,
                year,
            })
        })
        .collect())
}
