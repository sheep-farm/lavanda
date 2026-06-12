use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct SavedState {
    pub volume: Option<f32>,
    pub last_folder: Option<PathBuf>,
}

fn state_path() -> PathBuf {
    let xdg = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!(
            "{}/.config",
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
        )
    });
    PathBuf::from(xdg).join("lavanda").join("state.toml")
}

pub fn load() -> SavedState {
    let Ok(content) = std::fs::read_to_string(state_path()) else {
        return SavedState::default();
    };
    toml::from_str(&content).unwrap_or_default()
}

pub fn save(s: &SavedState) {
    let path = state_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    if let Ok(content) = toml::to_string(s) {
        std::fs::write(path, content).ok();
    }
}
