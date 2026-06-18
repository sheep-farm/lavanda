use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Deserialize;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub music_dir: String, // String para suportar "~" antes de expandir
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: bool,
    pub language: String,
    pub seek_step: u64,
    pub volume_step: f32,
    pub jellyfin_url: String,
    pub jellyfin_token: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            music_dir: "~/Music".into(),
            volume: 0.8,
            shuffle: false,
            repeat: false,
            language: "auto".into(),
            seek_step: 5,
            volume_step: 0.05,
            jellyfin_url: String::new(),
            jellyfin_token: String::new(),
        }
    }
}

impl Config {
    /// Retorna `music_dir` com `~` expandido para `$HOME`.
    pub fn music_path(&self) -> PathBuf {
        expand_tilde(&self.music_dir)
    }
}

// ── Inicialização ─────────────────────────────────────────────────────────────

pub fn load() {
    CONFIG.get_or_init(|| read_or_default());
}

pub fn get() -> &'static Config {
    CONFIG.get_or_init(|| read_or_default())
}

fn read_or_default() -> Config {
    let path = config_path();

    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        std::fs::write(&path, DEFAULT_CONFIG).ok();
        eprintln!("lavanda: configuração criada em {}", path.display());
        return Config::default();
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lavanda: erro ao ler config: {e}");
            return Config::default();
        }
    };

    match toml::from_str::<Config>(&content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("lavanda: config inválida ({e}), usando padrões");
            Config::default()
        }
    }
}

fn config_path() -> PathBuf {
    expand_tilde("~/.config/lavanda/config.toml")
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home).join(&path[2..])
    } else {
        PathBuf::from(path)
    }
}

// ── Config padrão gerada na primeira execução ─────────────────────────────────

const DEFAULT_CONFIG: &str = r#"# lavanda — configuration file
# ~/.config/lavanda/config.toml
#
# All fields are optional. Missing fields use the defaults shown here.

# Path to your music library. Scanned on launch and browsed by artist/album/genre.
music_dir = "~/Music"

# Initial volume (0.0 = mute, 1.0 = 100%)
volume = 0.8

# Start the session with shuffle enabled
shuffle = false

# Start the session with repeat enabled
repeat = false

# Interface language. Options: "auto", "en", "pt_BR", "es"
# "auto" detects from $LANG
language = "auto"

# Seek step in seconds for the ← → arrow keys
seek_step = 5

# Volume delta per + / - keypress
volume_step = 0.05

# Jellyfin integration (optional)
# jellyfin_url   = "http://your-jellyfin-server:8096"
# jellyfin_token = "your-api-key"
"#;
