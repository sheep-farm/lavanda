# lavanda

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.1.1--dev-blue.svg)](Cargo.toml)
[![Wayland](https://img.shields.io/badge/Wayland-native-cyan.svg)](https://wayland.freedesktop.org)

A native Wayland music player written in Rust, built for [Omarchy](https://omarchy.org/) / Hyprland rices. Follows the active Omarchy theme automatically — colors update live when you switch themes.

![lavanda](https://raw.githubusercontent.com/sheep-farm/lavanda/master/assets/screenshot.png)

---

## Features

- **Audio formats** — MP3, FLAC, OGG, Opus, WAV, AAC, M4A, AIFF and more via [Symphonia](https://github.com/pdeljanov/Symphonia)
- **Folder-based library** — `~/Music` subdirectories shown in the sidebar; flat layout (audio files directly in `~/Music`) also supported
- **On-demand scan** — each folder is scanned when selected; results cached for the session, nothing written to disk
- **Album art** — embedded cover tag displayed; falls back to `cover.jpg`, `Cover.jpg`, `folder.jpg` (and `.png`, `.webp` variants) in the same directory
- **Real seek** — click anywhere on the progress bar to jump
- **Dynamic volume** — slider takes effect immediately, mid-playback
- **Shuffle & repeat** — per-session
- **Keyboard navigation** — arrow keys move the cursor within sidebar and track list; `←`/`→` switch focus between panels; `Enter` activates; full keyboard-first design
- **Metadata editor** — press `m` on any track to edit title, artist, album and track number; tags written back to the file via lofty
- **Play-on-click toggle** — press `i` to switch between immediate playback on click/Enter and cursor-only browse mode
- **Spectrum visualizer** — FFT bars in the player panel, auto-normalized to the signal level; toggle with `v`
- **Focus layout** — large cover art centered, library hidden, spectrum at the bottom; toggle with `f`
- **Resizable sidebar** — drag the divider; width is persisted across sessions
- **Session state** — last selected folder and volume are restored on next launch
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

# Path to your music library. Subdirectories appear as folders in the sidebar.
music_dir = "~/Music"

# Initial volume (0.0 = mute, 1.0 = 100%)
volume = 0.8

# Start with shuffle enabled
shuffle = false

# Start with repeat enabled
repeat = false

# Interface language: "auto", "en", "pt_BR", "es"
language = "auto"

# Seek step in seconds for Shift+← / Shift+→
seek_step = 5

# Volume delta per + / - keypress
volume_step = 0.05

# Play track immediately when clicking or pressing Enter (true),
# or just move the cursor without starting playback (false).
# Toggle at runtime with the i key.
play_on_click = true
```

Session state (last folder, volume) is saved separately at `~/.config/lavanda/state.toml` and restored automatically on next launch.

---

## Music library

lavanda scans `~/Music` on demand. Subdirectories appear as folders in the sidebar. Audio files placed directly in `~/Music` (flat layout) are also supported — the root directory is included as the first sidebar entry.

**Tag fallback hierarchy:**
- No artist tag → parent folder name
- No album tag → immediate parent folder name
- No title tag → filename stem

**"Playlists"** are handled via the filesystem: create a symlink directory pointing at any tracks you want grouped, and lavanda will scan it like any other folder. `WalkDir` follows symlinks transparently.

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
| `↑` / `↓` | move cursor within the focused panel |
| `←` / `→` | switch focus between sidebar and track list |
| `Enter` | activate item under cursor (select folder / play track) |

### Playback

| Key | Action |
|---|---|
| `Space` | play / pause |
| `Shift+→` / `Shift+←` | seek forward / backward |
| `n` / `p` | next / previous track |
| `s` | toggle shuffle |
| `r` | toggle repeat |
| `+` or `=` | volume up |
| `-` | volume down |

### Library

| Key | Action |
|---|---|
| `/` | search / filter tracks |
| `m` | edit metadata of the track under the cursor |
| `i` | toggle play-on-click (immediate playback vs. browse mode) |
| `v` | toggle spectrum visualizer |
| `f` | toggle focus layout (large cover, library hidden) |
| `Ctrl+K` | keybinding reference (this table) |
| `Escape` | close any overlay |

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
├── locale.rs           # i18n strings (en, pt_BR, es)
├── audio/
│   ├── player.rs       # symphonia decode + cpal output (dedicated thread)
│   └── mpris.rs        # MPRIS2 D-Bus server via mpris-server
├── library/
│   ├── scanner.rs      # on-demand folder scan (walkdir + lofty); cover loader
│   └── models.rs       # Track
└── ui/
    ├── theme.rs        # Omarchy theme reader, live palette, container styles
    ├── icons.rs        # Nerd Font codepoints and UI font constants
    ├── views/          # library view, player panel
    └── components/     # progress bar, playback controls
```

No database. No disk cache. All library data lives in memory for the duration of the session.

---

## Status

**0.1.0** — functional for daily use.

Known limitations:
- Seek accuracy depends on the container format (Symphonia limitation)
- No gapless playback between tracks
- Volume set via slider is not persisted on close (use `+`/`-` keys to adjust persistently)

---

## License

MIT
