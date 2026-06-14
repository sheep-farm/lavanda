# lavanda

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-1.0.1-blue.svg)](Cargo.toml)
[![Wayland](https://img.shields.io/badge/Wayland-native-cyan.svg)](https://wayland.freedesktop.org)

A native Wayland music player written in Rust, built for [Omarchy](https://omarchy.org/) / Hyprland rices. Follows the active Omarchy theme automatically — colors update live when you switch themes.

![lavanda](https://raw.githubusercontent.com/sheep-farm/lavanda/master/assets/screenshot.png)

---

## Features

- **Audio formats** — MP3, FLAC, OGG, Opus, WAV, AAC, M4A, AIFF and more via [Symphonia](https://github.com/pdeljanov/Symphonia)
- **Indexed library** — `~/Music` is scanned on launch and browsed by **Artists**, **Albums** or **Genres** from the sidebar
- **Sortable, configurable columns** — title, artist, album, genre, year, disc, duration, play count, date played; click a header to sort, right-click to show/hide columns
- **Playlists** — create, rename (double-click), delete custom playlists; add tracks or whole albums via right-click
- **Auto-playlists** — Liked Songs, Recently Played and Most Played, maintained automatically
- **Liked songs & play counts** — toggle a favorite with `l`/`f`; plays are counted and persisted to `~/.config/lavanda/db.json`
- **Multi-selection** — `Ctrl+Click` to toggle, `Shift+Click` to range-select; context-menu actions apply to the whole selection
- **Album art** — embedded cover tag displayed; falls back to `cover.jpg`, `Cover.jpg`, `folder.jpg` (and `.png`, `.webp` variants) in the same directory
- **Real seek** — click anywhere on the progress bar to jump
- **Dynamic volume** — slider takes effect immediately, mid-playback
- **Shuffle & repeat** — per-session
- **Metadata editor** — press `e`/`m` on any track to edit title, artist, album, genre, year, disc and track number; optionally apply to the whole album; tags written back to the file via lofty
- **Spectrum visualizer** — FFT bars in the player panel, auto-normalized to the signal level; toggle with `v`
- **Focus layout** — large cover art centered, library hidden, spectrum at the bottom; toggle with `t`
- **Resizable panels** — drag the sidebar divider and the playlist-panel divider; sidebar width is persisted across sessions
- **Session state** — volume is restored on next launch
- **MPRIS2** — full D-Bus integration; works with `playerctl`, Waybar `mpris` module, AGS, EWW, etc.
- **Nerd Font icons** — Font Awesome tier-1 codepoints (universal across any Nerd Font)
- **Live Omarchy theming** — reads `~/.config/omarchy/current/theme.name` and updates the palette within 3 seconds of a theme switch; no restart required
- **i18n** — English, Portuguese (pt_BR) and Spanish; auto-detected from `$LANG`

---

## Requirements

| Requirement | Notes |
|---|---|
| Rust ≥ 1.75 | `rustup` recommended |
| A Nerd Font | `JetBrainsMono Nerd Font Mono` by default; any Nerd Font works |
| PipeWire or PulseAudio | Audio output via cpal |
| D-Bus session bus | For MPRIS2 (`DBUS_SESSION_BUS_ADDRESS` must be set) |
| Wayland compositor | Tested on Hyprland; works on any wlroots compositor |

---

## Installation

### Prebuilt binary (no Rust needed)

```bash
curl -LO https://github.com/sheep-farm/lavanda/releases/download/v1.0.1/lavanda-1.0.1-x86_64-linux.tar.gz
tar xzf lavanda-1.0.1-x86_64-linux.tar.gz
install -Dm755 lavanda ~/.local/bin/lavanda
```

### From source

```bash
git clone https://github.com/sheep-farm/lavanda
cd lavanda
cargo build --release
install -Dm755 target/release/lavanda ~/.local/bin/lavanda
```

### Desktop entry

```bash
cp assets/lavanda.desktop ~/.local/share/applications/
```

### With cargo install

```bash
cargo install --git https://github.com/sheep-farm/lavanda
```

---

## Configuration

lavanda generates `~/.config/lavanda/config.toml` on first run with all options set to their defaults.

```toml
# ~/.config/lavanda/config.toml

# Path to your music library. Scanned on launch and browsed by artist/album/genre.
music_dir = "~/Music"

# Initial volume (0.0 = mute, 1.0 = 100%)
volume = 0.8

# Start with shuffle enabled
shuffle = false

# Start with repeat enabled
repeat = false

# Interface language: "auto", "en", "pt_BR", "es"
language = "auto"

# Seek step in seconds for the ← / → arrow keys
seek_step = 5

# Volume delta per + / - keypress
volume_step = 0.05
```

Volume is restored on next launch via `~/.config/lavanda/state.toml`. Library data — liked songs, play counts, playlists, recently played and column layout — is persisted to `~/.config/lavanda/db.json`.

---

## Music library

lavanda scans `~/Music` recursively on launch and builds an in-memory index. The sidebar browses it by **Artists**, **Albums** or **Genres** (switch with the tabs at the top), with a filter box to narrow the list. Press `F5` to rescan after adding files.

**Tag fallback hierarchy:**
- No artist tag → parent folder name
- No album tag → immediate parent folder name
- No title tag → filename stem

**Playlists** are managed inside the app: create one with `c` or the `+` button, double-click to rename, and add tracks or whole albums through the right-click menu. Liked Songs, Recently Played and Most Played are maintained automatically. Custom playlists, likes, play counts and recently-played history are stored in `~/.config/lavanda/db.json`.

**Cover art lookup order:**
1. Embedded tag (CoverFront or Other picture type)
2. `cover.jpg`, `Cover.jpg`, `cover.png`, `Cover.png`, `cover.webp`, `Cover.webp`
3. `folder.jpg`, `Folder.jpg`, `folder.png`, `Folder.png`

---

## Omarchy theming

lavanda reads the active Omarchy theme from `~/.config/omarchy/current/theme.name` and maps its `colors.toml` to the UI palette:

| `colors.toml` key | lavanda role |
|---|---|
| `background` | window background |
| `foreground` | primary text |
| `accent` | accent color (highlights, active elements) |
| `color8` | muted/overlay color; also used to derive surface shades |
| `color1` | red (error status bar) |
| `color2` | green |
| `color15` | subtext |

Works with all built-in Omarchy themes (Catppuccin, Nord, Gruvbox, Tokyo Night, Rosé Pinè, etc.) and custom user themes in `~/.config/omarchy/themes/`.

### Waybar integration

```jsonc
"mpris": {
    "format": "{player_icon}  {title} — {artist}",
    "format-paused": "{player_icon}  {title} — {artist}",
    "format-stopped": "",
    "player-icons": { "lavanda": "󰝚", "default": "󰝚" },
    "status-icons": { "paused": "󰏤", "playing": "󰐊", "stopped": "󰓛" },
    "max-length": 45,
    "on-click": "playerctl play-pause",
    "on-click-right": "playerctl next",
    "on-scroll-up": "playerctl next",
    "on-scroll-down": "playerctl previous",
    "tooltip-format": "{title}\n{artist} — {album}"
}
```

---

## Keybindings

These work when the lavanda window is focused.

### Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | move cursor within the track list |
| `Enter` | play the selected track |

### Playback

| Key | Action |
|---|---|
| `Space` | play / pause |
| `→` / `←` | seek forward / backward |
| `n` / `p` | next / previous track |
| `s` | toggle shuffle |
| `r` | toggle repeat |
| `+` or `=` | volume up |
| `-` | volume down |

### Library

| Key | Action |
|---|---|
| `/` | search / filter tracks |
| `l` / `f` | like / unlike the selected track |
| `e` / `m` | edit metadata of the selected track |
| `c` | create a playlist |
| `a` | add the selected track to a playlist |
| `t` | toggle focus layout (large cover, library hidden) |
| `v` | toggle spectrum visualizer |
| `F5` | rescan the library |
| `?` or `Ctrl+K` | keybinding reference |
| `Escape` | close any overlay |

Mouse: double-click a track to play it; right-click a track, artist, album or column header for context actions; `Ctrl+Click` / `Shift+Click` to multi-select.

For system-wide controls (lavanda running in background), wire `playerctl` to your compositor:

```ini
# hyprland.conf
bind = SUPER, F5, exec, playerctl play-pause
bind = SUPER, F6, exec, playerctl previous
bind = SUPER, F7, exec, playerctl next
```

---

## playerctl

```bash
playerctl -p lavanda play-pause
playerctl -p lavanda next
playerctl -p lavanda previous
playerctl -p lavanda metadata
```

---

## Architecture

```
src/
├── main.rs
├── app.rs              # iced Application — state, messages, subscriptions
├── config.rs           # config.toml parsing and defaults
├── state.rs            # session state persistence (state.toml)
├── persist.rs          # library data store (db.json): likes, play counts, playlists, columns
├── locale.rs           # i18n strings (en, pt_BR, es)
├── audio/
│   ├── player.rs       # symphonia decode + cpal output (dedicated thread)
│   ├── spectrum.rs     # FFT spectrum analysis
│   └── mpris.rs        # MPRIS2 D-Bus server via mpris-server
├── library/
│   ├── scanner.rs      # library scan (walkdir + lofty); cover loader
│   └── models.rs       # Track
└── ui/
    ├── theme.rs        # Omarchy theme reader, live palette, container styles
    ├── icons.rs        # Nerd Font codepoints and UI font constants
    ├── views/          # library view, player panel, focus layout
    └── components/     # progress bar, controls, spectrum, tag editor, playlist dialog
```

Track metadata lives in memory for the session; library state (likes, play counts, playlists, recently played, column layout) is persisted as JSON in `~/.config/lavanda/db.json`.

---

## Status

**1.0.1** — functional for daily use.

Known limitations:
- Seek accuracy depends on the container format (Symphonia limitation)
- No gapless playback between tracks

---

## Lineage

Version **1.0.1** is the product of **convergent evolution with [OmaTunes](https://github.com/Balthazzahr/omatunes)**, a sibling project by the same author that forked from an early lavanda and explored playlists, an indexed library, play counts and a richer tag editor. lavanda 1.0.1 folds those ideas back into the original codebase. The two projects share a common git ancestry and the same MIT license; parts of the library-state store (`persist.rs`) and the `Track` model originate from OmaTunes.

The development model behind this — fork, diverge, and selectively reintegrate — is described in [DEVELOPMENT.md](DEVELOPMENT.md).

---

## License

MIT
