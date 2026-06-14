use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use lofty::prelude::*;
use lofty::probe::Probe;
use walkdir::WalkDir;

use super::models::Track;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "opus", "wav", "aac", "m4a", "wma", "aiff",
];

const COVER_FILENAMES: &[&str] = &[
    "cover.jpg", "Cover.jpg", "cover.png", "Cover.png",
    "cover.webp", "Cover.webp", "folder.jpg", "Folder.jpg",
    "folder.png", "Folder.png",
];

/// Escaneia `dir` recursivamente e retorna faixas ordenadas por disco/álbum/número/título.
/// `cover_data` é sempre `None` — carregado sob demanda via `load_cover`.
pub fn scan_folder(dir: &Path) -> Vec<Track> {
    let mut pairs: Vec<(PathBuf, TrackInfo)> = WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            let ext = path.extension()?.to_str()?.to_lowercase();
            if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                return None;
            }
            read_tags(&path).ok().map(|info| (path, info))
        })
        .collect();

    pairs.sort_by(|(_, a), (_, b)| {
        a.album
            .cmp(&b.album)
            .then(a.disc_number.cmp(&b.disc_number))
            .then(a.track_number.cmp(&b.track_number))
            .then(a.title.cmp(&b.title))
    });

    pairs
        .into_iter()
        .enumerate()
        .map(|(i, (path, info))| {
            let (play_count, liked) = crate::persist::get(|db| {
                let pc = db.play_counts.get(&path).copied().unwrap_or(0);
                let l = db.favorites.contains(&path);
                (pc, l)
            });
            Track {
                id: (i + 1) as i64,
                path,
                title: info.title,
                artist: info.artist,
                album: info.album,
                track_number: info.track_number,
                disc_number: info.disc_number,
                duration: Duration::from_millis(info.duration_ms),
                cover_data: None,
                genre: info.genre,
                year: info.year,
                play_count,
                liked,
                date_played: None,
            }
        })
        .collect()
}

/// Carrega a capa: tag embutida primeiro, depois arquivo de capa na pasta.
pub fn load_cover(path: &Path) -> Option<Vec<u8>> {
    let tagged = Probe::open(path).ok()?.read().ok()?;
    let embedded = tagged.primary_tag().and_then(|t| {
        t.pictures()
            .iter()
            .find(|p| {
                matches!(
                    p.pic_type(),
                    lofty::picture::PictureType::CoverFront | lofty::picture::PictureType::Other
                )
            })
            .map(|p| p.data().to_vec())
    });
    embedded.or_else(|| cover_from_folder(path))
}

// ── Internos ───────────────────────────────────────────────────────────────────

struct TrackInfo {
    title: String,
    artist: String,
    album: String,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    duration_ms: u64,
    genre: String,
    year: Option<u32>,
}

fn read_tags(path: &Path) -> Result<TrackInfo> {
    let tagged = Probe::open(path)?.read()?;
    let duration_ms = tagged.properties().duration().as_millis() as u64;
    let tags = tagged.primary_tag();

    let unknown = crate::locale::get().unknown;

    let title = tags
        .and_then(|t| t.title())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(unknown)
                .to_string()
        });

    let folder_artist = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(unknown)
        .to_string();

    let folder_album = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(unknown)
        .to_string();

    let artist = tags
        .and_then(|t| t.artist())
        .map(|s| s.to_string())
        .unwrap_or(folder_artist);

    let album = tags
        .and_then(|t| t.album())
        .map(|s| s.to_string())
        .unwrap_or(folder_album);

    let track_number = tags.and_then(|t| t.track());
    let disc_number = tags.and_then(|t| t.disk());

    let genre = tags
        .and_then(|t| t.genre())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let year = tags.and_then(|t| t.year());

    Ok(TrackInfo {
        title,
        artist,
        album,
        track_number,
        disc_number,
        duration_ms,
        genre,
        year,
    })
}

/// Escreve metadados no arquivo de áudio.
pub fn write_tags(
    path: &Path,
    title: &str,
    artist: &str,
    album: &str,
    genre: &str,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    cover_path: Option<&str>,
    year: Option<u32>,
) -> Result<()> {
    use lofty::config::WriteOptions;

    let mut tagged_file = Probe::open(path)?.read()?;

    let has_primary = tagged_file.primary_tag().is_some();
    let has_any = has_primary || tagged_file.first_tag().is_some();
    anyhow::ensure!(has_any, "no writable tag found in file");

    let tag = if has_primary {
        tagged_file.primary_tag_mut().unwrap()
    } else {
        tagged_file.first_tag_mut().unwrap()
    };

    tag.set_title(title.to_owned());
    tag.set_artist(artist.to_owned());
    tag.set_album(album.to_owned());
    tag.set_genre(genre.to_owned());

    if let Some(n) = track_number {
        tag.set_track(n);
    } else {
        tag.remove_track();
    }

    if let Some(n) = disc_number {
        tag.set_disk(n);
    } else {
        tag.remove_disk();
    }

    if let Some(y) = year {
        tag.set_year(y);
    }

    if let Some(cp) = cover_path {
        let cp = cp.trim();
        if !cp.is_empty() {
            if let Ok(data) = std::fs::read(cp) {
                let mime = if cp.ends_with(".png") { "image/png" } else { "image/jpeg" };
                let pic = lofty::picture::Picture::new_unchecked(
                    lofty::picture::PictureType::CoverFront,
                    Some(lofty::picture::MimeType::from_str(mime)),
                    None,
                    data,
                );
                tag.set_picture(0, pic);
            }
        }
    }

    tag.save_to_path(path, WriteOptions::default())?;
    Ok(())
}

fn cover_from_folder(path: &Path) -> Option<Vec<u8>> {
    let dir = path.parent()?;
    for name in COVER_FILENAMES {
        if let Ok(data) = std::fs::read(dir.join(name)) {
            return Some(data);
        }
    }
    None
}
