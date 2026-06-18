use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Result};

const UA: &str = concat!("lavanda/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct JellyfinItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct JellyfinTrack {
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

impl JellyfinTrack {
    pub fn to_track(&self) -> crate::library::models::Track {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.id.hash(&mut h);
        crate::library::models::Track {
            id: h.finish() as i64,
            path: std::path::PathBuf::from(format!("jellyfin://{}", self.id)),
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

pub fn is_jellyfin_path(path: &Path) -> bool {
    path.to_str().map_or(false, |s| s.starts_with("jellyfin://"))
}

pub fn item_id_from_path(path: &Path) -> Option<String> {
    path.to_str()?.strip_prefix("jellyfin://").map(|s| s.to_string())
}

pub fn stream_url(base_url: &str, item_id: &str, token: &str) -> String {
    format!(
        "{}/Audio/{}/stream?api_key={}&Static=true",
        base_url.trim_end_matches('/'),
        item_id,
        token,
    )
}

fn get(url: &str) -> Result<serde_json::Value> {
    let resp = ureq::get(url)
        .set("User-Agent", UA)
        .call()
        .map_err(|e| anyhow!("{e}"))?;
    Ok(resp.into_json()?)
}

pub fn fetch_artists(base_url: &str, token: &str) -> Result<Vec<JellyfinItem>> {
    let url = format!(
        "{}/Artists?api_key={}&Recursive=true&IncludeItemTypes=Audio&SortBy=Name&SortOrder=Ascending&Limit=500",
        base_url.trim_end_matches('/'),
        token,
    );
    let resp = get(&url)?;
    parse_items(&resp)
}

pub fn fetch_albums(base_url: &str, token: &str, artist_id: &str) -> Result<Vec<JellyfinItem>> {
    let url = format!(
        "{}/Items?api_key={}&AlbumArtistIds={}&IncludeItemTypes=MusicAlbum&Recursive=true&SortBy=ProductionYear,SortName&SortOrder=Ascending&Limit=500",
        base_url.trim_end_matches('/'),
        token,
        artist_id,
    );
    let resp = get(&url)?;
    parse_items(&resp)
}

pub fn fetch_tracks(base_url: &str, token: &str, album_id: &str) -> Result<Vec<JellyfinTrack>> {
    let url = format!(
        "{}/Items?api_key={}&ParentId={}&IncludeItemTypes=Audio&SortBy=ParentIndexNumber,IndexNumber&SortOrder=Ascending",
        base_url.trim_end_matches('/'),
        token,
        album_id,
    );
    let resp = get(&url)?;
    let items = resp["Items"]
        .as_array()
        .ok_or_else(|| anyhow!("resposta sem campo Items"))?;

    Ok(items
        .iter()
        .filter_map(|item| {
            let id = item["Id"].as_str()?.to_string();
            let title = item["Name"].as_str().unwrap_or("Unknown").to_string();
            let artist = item["AlbumArtist"]
                .as_str()
                .or_else(|| {
                    item["Artists"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("Unknown")
                .to_string();
            let album = item["Album"].as_str().unwrap_or("Unknown").to_string();
            let track_number = item["IndexNumber"].as_u64().map(|n| n as u32);
            let disc_number = item["ParentIndexNumber"].as_u64().map(|n| n as u32);
            let year = item["ProductionYear"].as_u64().map(|n| n as u32);
            let genre = item["Genres"]
                .as_array()
                .and_then(|g| g.first())
                .and_then(|g| g.as_str())
                .unwrap_or("")
                .to_string();
            // Jellyfin duration em "ticks" (unidades de 100 ns)
            let ticks = item["RunTimeTicks"].as_u64().unwrap_or(0);
            let duration = Duration::from_nanos(ticks * 100);
            Some(JellyfinTrack {
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

fn parse_items(resp: &serde_json::Value) -> Result<Vec<JellyfinItem>> {
    let items = resp["Items"]
        .as_array()
        .ok_or_else(|| anyhow!("resposta sem campo Items"))?;
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = item["Id"].as_str()?.to_string();
            let name = item["Name"].as_str()?.to_string();
            Some(JellyfinItem { id, name })
        })
        .collect())
}
