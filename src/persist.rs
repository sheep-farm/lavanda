use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

static DB: std::sync::OnceLock<Mutex<Db>> = std::sync::OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableColumn {
    TrackNumber,
    Title,
    Artist,
    Album,
    Genre,
    Year,
    DiscNumber,
    Duration,
    Plays,
    DatePlayed,
}

impl TableColumn {
    pub fn all() -> &'static [TableColumn] {
        &[
            TableColumn::TrackNumber,
            TableColumn::Title,
            TableColumn::Artist,
            TableColumn::Album,
            TableColumn::Genre,
            TableColumn::Year,
            TableColumn::DiscNumber,
            TableColumn::Duration,
            TableColumn::Plays,
            TableColumn::DatePlayed,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            TableColumn::TrackNumber => "#",
            TableColumn::Title => "Title",
            TableColumn::Artist => "Artist",
            TableColumn::Album => "Album",
            TableColumn::Genre => "Genre",
            TableColumn::Year => "Year",
            TableColumn::DiscNumber => "Disc",
            TableColumn::Duration => "Duration",
            TableColumn::Plays => "Plays",
            TableColumn::DatePlayed => "Date Played",
        }
    }
}

fn default_columns() -> Vec<TableColumn> {
    vec![
        TableColumn::TrackNumber,
        TableColumn::Title,
        TableColumn::Artist,
        TableColumn::Album,
        TableColumn::Duration,
        TableColumn::Plays,
    ]
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Db {
    #[serde(default)]
    pub favorites: HashSet<PathBuf>,
    #[serde(default)]
    pub play_counts: HashMap<PathBuf, u32>,
    #[serde(default)]
    pub playlists: HashMap<String, Vec<PathBuf>>,
    #[serde(default)]
    pub recently_played: Vec<(PathBuf, String)>,
    #[serde(default)]
    pub hidden_artists_albums: Vec<(String, bool)>,
    #[serde(default = "default_columns")]
    pub table_columns: Vec<TableColumn>,
    #[serde(default)]
    pub radio_favorites: Vec<crate::radio::RadioStation>,
    #[serde(default)]
    pub radio_quarantine: Vec<String>,
}

impl Default for Db {
    fn default() -> Self {
        Db {
            favorites: HashSet::new(),
            play_counts: HashMap::new(),
            playlists: HashMap::new(),
            recently_played: Vec::new(),
            hidden_artists_albums: Vec::new(),
            table_columns: default_columns(),
            radio_favorites: Vec::new(),
            radio_quarantine: Vec::new(),
        }
    }
}

fn db_path() -> PathBuf {
    let xdg = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!("{}/.config", std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
    });
    PathBuf::from(xdg).join("lavanda/db.json")
}

impl Db {
    fn load() -> Self {
        let path = db_path();
        if !path.exists() {
            return Self::default();
        }
        let mut db: Db = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // Repara dbs antigos que persistiram a lista de colunas vazia.
        if db.table_columns.is_empty() {
            db.table_columns = default_columns();
        }
        db
    }

    fn save(&self) {
        let path = db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            std::fs::write(path, json).ok();
        }
    }
}

fn db() -> &'static Mutex<Db> {
    DB.get_or_init(|| Mutex::new(Db::load()))
}

pub fn init() {
    let _ = db();
}

pub fn get<F, R>(f: F) -> R
where
    F: FnOnce(&Db) -> R,
{
    f(&db().lock().unwrap())
}

pub fn write<F, R>(f: F) -> R
where
    F: FnOnce(&mut Db) -> R,
{
    let mut guard = db().lock().unwrap();
    let res = f(&mut guard);
    guard.save();
    res
}

pub fn toggle_favorite(path: PathBuf) -> bool {
    write(|db| {
        if db.favorites.contains(&path) {
            db.favorites.remove(&path);
            false
        } else {
            db.favorites.insert(path);
            true
        }
    })
}

pub fn increment_play_count(path: PathBuf) -> u32 {
    write(|db| {
        let count = db.play_counts.entry(path).or_insert(0);
        *count += 1;
        *count
    })
}

pub fn add_to_recently_played(path: PathBuf) {
    write(|db| {
        db.recently_played.retain(|(p, _)| p != &path);
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        db.recently_played.insert(0, (path, now));
        if db.recently_played.len() > 100 {
            db.recently_played.truncate(100);
        }
    });
}

pub fn create_playlist(name: String) {
    write(|db| {
        db.playlists.entry(name).or_default();
    });
}

pub fn delete_playlist(name: String) {
    write(|db| {
        db.playlists.remove(&name);
    });
}

pub fn rename_playlist(old: String, new: String) {
    write(|db| {
        if let Some(list) = db.playlists.remove(&old) {
            db.playlists.insert(new, list);
        }
    });
}

pub fn add_to_playlist(name: String, path: PathBuf) {
    write(|db| {
        let list = db.playlists.entry(name).or_default();
        if !list.contains(&path) {
            list.push(path);
        }
    });
}

// ── Rádio ──────────────────────────────────────────────────────────────────────

/// Identidade de uma estação: uuid quando houver, senão a URL.
fn station_key(s: &crate::radio::RadioStation) -> String {
    if !s.stationuuid.is_empty() {
        s.stationuuid.clone()
    } else {
        s.url.clone()
    }
}

pub fn is_radio_favorite(station: &crate::radio::RadioStation) -> bool {
    let key = station_key(station);
    get(|db| db.radio_favorites.iter().any(|s| station_key(s) == key))
}

/// Alterna o favorito; retorna `true` se passou a ser favorito.
pub fn toggle_radio_favorite(station: &crate::radio::RadioStation) -> bool {
    let key = station_key(station);
    write(|db| {
        if let Some(pos) = db.radio_favorites.iter().position(|s| station_key(s) == key) {
            db.radio_favorites.remove(pos);
            false
        } else {
            db.radio_favorites.push(station.clone());
            true
        }
    })
}

pub fn radio_favorites() -> Vec<crate::radio::RadioStation> {
    get(|db| db.radio_favorites.clone())
}

pub fn is_quarantined(station: &crate::radio::RadioStation) -> bool {
    let key = station_key(station);
    get(|db| db.radio_quarantine.iter().any(|k| k == &key))
}

/// Coloca a estação em quarentena (não aparece mais nas listas).
pub fn quarantine_station(station: &crate::radio::RadioStation) {
    let key = station_key(station);
    write(|db| {
        if !db.radio_quarantine.contains(&key) {
            db.radio_quarantine.push(key);
        }
        // Quarentena também remove dos favoritos, se estiver lá.
        db.radio_favorites
            .retain(|s| station_key(s) != station_key(station));
    });
}
