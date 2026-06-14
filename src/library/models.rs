use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: i64,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration: Duration,
    pub cover_data: Option<Vec<u8>>,
    pub genre: String,
    pub year: Option<u32>,
    pub play_count: u32,
    pub liked: bool,
    pub date_played: Option<String>,
}

impl Track {
    pub fn duration_str(&self) -> String {
        let secs = self.duration.as_secs();
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}:{s:02}")
    }
}
