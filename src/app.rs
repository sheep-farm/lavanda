use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use mpris_server::{LoopStatus, PlaybackStatus};

use crate::audio::mpris;
use crate::audio::{
    AudioCommand, AudioEvent, AudioPlayer, MprisCommand, MprisUpdate, PlaybackState,
};
use crate::library::models::Track;
use crate::library::{load_cover, scan_folder};
use crate::ui::{theme, views};

// ── Mensagens ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectFolder(PathBuf),
    FolderScanned(PathBuf, Vec<Track>),

    PlayTrack(Track),
    PlayPause,
    NextTrack,
    PreviousTrack,
    Seek(Duration),
    VolumeChanged(f32),
    ToggleShuffle,
    ToggleRepeat,
    SeekRelative(i64),
    VolumeStep(f32),

    SidebarDragStart,
    SidebarDragMove(f32),
    SidebarDragEnd,

    PollAudio,
    CheckTheme,
}

// ── Estado global ─────────────────────────────────────────────────────────────

pub struct AppState {
    pub playback_state: PlaybackState,
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: bool,

    pub folders: Vec<PathBuf>,
    pub selected_folder: Option<PathBuf>,
    pub tracks: Vec<Track>,
    folder_cache: HashMap<PathBuf, Vec<Track>>,

    pub sidebar_width: f32,
    dragging_sidebar: bool,

    pub iced_theme: iced::Theme,
    loaded_theme_name: String,

    pub strings: &'static crate::locale::Strings,

    audio: AudioPlayer,
    mpris_cmd_rx: tokio::sync::mpsc::UnboundedReceiver<MprisCommand>,
    mpris_update_tx: tokio::sync::mpsc::UnboundedSender<MprisUpdate>,
}

impl AppState {
    fn new() -> (Self, Task<Message>) {
        let audio = AudioPlayer::spawn();

        let cfg = crate::config::get();
        let folders = music_subfolders(&cfg.music_path());

        let (mpris_cmd_tx, mpris_cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let (mpris_update_tx, mpris_update_rx) = tokio::sync::mpsc::unbounded_channel();
        mpris::launch(mpris_cmd_tx, mpris_update_rx);

        let loaded_theme_name = crate::ui::theme::read_current_theme_name();
        let iced_theme = build_iced_theme();

        let state = AppState {
            playback_state: PlaybackState::Stopped,
            current_track: None,
            queue: Vec::new(),
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: cfg.volume.clamp(0.0, 1.0),
            shuffle: cfg.shuffle,
            repeat: cfg.repeat,
            folders,
            selected_folder: None,
            tracks: Vec::new(),
            folder_cache: HashMap::new(),
            sidebar_width: load_sidebar_width(),
            dragging_sidebar: false,
            iced_theme,
            loaded_theme_name,
            strings: crate::locale::get(),
            audio,
            mpris_cmd_rx,
            mpris_update_tx,
        };

        (state, Task::none())
    }

    fn send_mpris(&self, update: MprisUpdate) {
        let _ = self.mpris_update_tx.send(update);
    }

    fn notify_mpris_track(&self, status: PlaybackStatus) {
        if let Some(track) = &self.current_track {
            self.send_mpris(MprisUpdate::Metadata {
                title: track.title.clone(),
                artist: track.artist.clone(),
                album: track.album.clone(),
                duration_us: track.duration.as_micros() as i64,
            });
        }
        self.send_mpris(MprisUpdate::Status(status));
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectFolder(path) => {
                self.selected_folder = Some(path.clone());
                if let Some(cached) = self.folder_cache.get(&path) {
                    self.tracks = cached.clone();
                    return Task::none();
                }
                self.tracks.clear();
                Task::perform(
                    async move {
                        let tracks = scan_folder(&path);
                        (path, tracks)
                    },
                    |(path, tracks)| Message::FolderScanned(path, tracks),
                )
            }

            Message::FolderScanned(path, tracks) => {
                self.folder_cache.insert(path.clone(), tracks.clone());
                if self.selected_folder.as_ref() == Some(&path) {
                    self.tracks = tracks;
                }
                Task::none()
            }

            Message::PlayTrack(track) => {
                let cover_data = load_cover(&track.path);
                let track = Track {
                    cover_data,
                    ..track
                };
                self.audio.send(AudioCommand::Play(track.path.clone()));
                self.audio.send(AudioCommand::SetVolume(self.volume));
                self.queue = self.tracks.clone();
                self.current_track = Some(track);
                self.playback_state = PlaybackState::Playing;
                self.position = Duration::ZERO;
                self.notify_mpris_track(PlaybackStatus::Playing);
                Task::none()
            }

            Message::PlayPause => {
                match self.playback_state {
                    PlaybackState::Playing => {
                        self.audio.send(AudioCommand::Pause);
                        self.playback_state = PlaybackState::Paused;
                        self.send_mpris(MprisUpdate::Status(PlaybackStatus::Paused));
                    }
                    PlaybackState::Paused => {
                        self.audio.send(AudioCommand::Resume);
                        self.playback_state = PlaybackState::Playing;
                        self.send_mpris(MprisUpdate::Status(PlaybackStatus::Playing));
                    }
                    PlaybackState::Stopped => {
                        if let Some(first) = self.tracks.first().cloned() {
                            self.queue = self.tracks.clone();
                            let cover_data = load_cover(&first.path);
                            let first = Track {
                                cover_data,
                                ..first
                            };
                            self.audio.send(AudioCommand::Play(first.path.clone()));
                            self.current_track = Some(first);
                            self.playback_state = PlaybackState::Playing;
                            self.position = Duration::ZERO;
                            self.notify_mpris_track(PlaybackStatus::Playing);
                        }
                    }
                }
                Task::none()
            }

            Message::NextTrack => {
                self.advance_track(1);
                Task::none()
            }
            Message::PreviousTrack => {
                self.advance_track(-1);
                Task::none()
            }

            Message::Seek(dur) => {
                self.audio.send(AudioCommand::Seek(dur));
                self.position = dur;
                Task::none()
            }

            Message::SeekRelative(delta_secs) => {
                let new_pos = if delta_secs < 0 {
                    self.position
                        .saturating_sub(Duration::from_secs(delta_secs.unsigned_abs()))
                } else {
                    (self.position + Duration::from_secs(delta_secs as u64)).min(self.duration)
                };
                self.audio.send(AudioCommand::Seek(new_pos));
                self.position = new_pos;
                Task::none()
            }

            Message::VolumeChanged(v) => {
                self.volume = v;
                self.audio.send(AudioCommand::SetVolume(v));
                self.send_mpris(MprisUpdate::Volume(v as f64));
                Task::none()
            }

            Message::VolumeStep(delta) => {
                let v = (self.volume + delta).clamp(0.0, 1.0);
                self.volume = v;
                self.audio.send(AudioCommand::SetVolume(v));
                self.send_mpris(MprisUpdate::Volume(v as f64));
                Task::none()
            }

            Message::ToggleShuffle => {
                self.shuffle = !self.shuffle;
                self.send_mpris(MprisUpdate::Shuffle(self.shuffle));
                Task::none()
            }

            Message::ToggleRepeat => {
                self.repeat = !self.repeat;
                let loop_status = if self.repeat {
                    LoopStatus::Track
                } else {
                    LoopStatus::None
                };
                self.send_mpris(MprisUpdate::Loop(loop_status));
                Task::none()
            }

            Message::SidebarDragStart => {
                self.dragging_sidebar = true;
                Task::none()
            }

            Message::SidebarDragMove(x) => {
                self.sidebar_width = x.clamp(120.0, 400.0);
                Task::none()
            }

            Message::SidebarDragEnd => {
                self.dragging_sidebar = false;
                save_sidebar_width(self.sidebar_width);
                Task::none()
            }

            Message::PollAudio => {
                while let Ok(event) = self.audio.event_rx.try_recv() {
                    match event {
                        AudioEvent::Progress { position, duration } => {
                            self.position = position;
                            self.duration = duration;
                        }
                        AudioEvent::Paused => {
                            self.playback_state = PlaybackState::Paused;
                        }
                        AudioEvent::Stopped => {
                            self.playback_state = PlaybackState::Stopped;
                            self.position = Duration::ZERO;
                            self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                        }
                        AudioEvent::TrackEnded => {
                            if self.repeat {
                                if let Some(t) = self.current_track.clone() {
                                    self.audio.send(AudioCommand::Play(t.path));
                                    self.notify_mpris_track(PlaybackStatus::Playing);
                                }
                            } else {
                                self.advance_track(1);
                            }
                        }
                        AudioEvent::Error(e) => eprintln!("Erro de áudio: {e}"),
                        AudioEvent::Playing => {
                            self.playback_state = PlaybackState::Playing;
                        }
                    }
                }

                while let Ok(cmd) = self.mpris_cmd_rx.try_recv() {
                    match cmd {
                        MprisCommand::Play => {
                            if !matches!(self.playback_state, PlaybackState::Playing) {
                                self.audio.send(AudioCommand::Resume);
                                self.playback_state = PlaybackState::Playing;
                                self.send_mpris(MprisUpdate::Status(PlaybackStatus::Playing));
                            }
                        }
                        MprisCommand::Pause => {
                            if matches!(self.playback_state, PlaybackState::Playing) {
                                self.audio.send(AudioCommand::Pause);
                                self.playback_state = PlaybackState::Paused;
                                self.send_mpris(MprisUpdate::Status(PlaybackStatus::Paused));
                            }
                        }
                        MprisCommand::PlayPause => match self.playback_state {
                            PlaybackState::Playing => {
                                self.audio.send(AudioCommand::Pause);
                                self.playback_state = PlaybackState::Paused;
                                self.send_mpris(MprisUpdate::Status(PlaybackStatus::Paused));
                            }
                            _ => {
                                self.audio.send(AudioCommand::Resume);
                                self.playback_state = PlaybackState::Playing;
                                self.send_mpris(MprisUpdate::Status(PlaybackStatus::Playing));
                            }
                        },
                        MprisCommand::Next => {
                            self.advance_track(1);
                        }
                        MprisCommand::Previous => {
                            self.advance_track(-1);
                        }
                        MprisCommand::Stop => {
                            self.audio.send(AudioCommand::Stop);
                            self.playback_state = PlaybackState::Stopped;
                            self.position = Duration::ZERO;
                            self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                        }
                    }
                }

                Task::none()
            }

            Message::CheckTheme => {
                let current = crate::ui::theme::read_current_theme_name();
                if !current.is_empty() && current != self.loaded_theme_name {
                    crate::ui::theme::reload_system_theme();
                    self.iced_theme = build_iced_theme();
                    self.loaded_theme_name = current;
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let main = column![
            self.header_view(),
            views::player::view(self),
            views::library::view(self),
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        container(main)
            .style(|_: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::base())),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let base = Subscription::batch([
            iced::time::every(Duration::from_millis(100)).map(|_| Message::PollAudio),
            iced::time::every(Duration::from_secs(3)).map(|_| Message::CheckTheme),
            iced::keyboard::on_key_press(|key, _mods| {
                use iced::keyboard::key::Named;
                use iced::keyboard::Key;
                let seek = crate::config::get().seek_step as i64;
                let vol = crate::config::get().volume_step;
                match key {
                    Key::Named(Named::Space) => Some(Message::PlayPause),
                    Key::Named(Named::ArrowRight) => Some(Message::SeekRelative(seek)),
                    Key::Named(Named::ArrowLeft) => Some(Message::SeekRelative(-seek)),
                    Key::Character(ref c) => match c.as_str() {
                        "n" | "N" => Some(Message::NextTrack),
                        "p" | "P" => Some(Message::PreviousTrack),
                        "s" | "S" => Some(Message::ToggleShuffle),
                        "r" | "R" => Some(Message::ToggleRepeat),
                        "+" | "=" => Some(Message::VolumeStep(vol)),
                        "-" => Some(Message::VolumeStep(-vol)),
                        _ => None,
                    },
                    _ => None,
                }
            }),
        ]);

        if self.dragging_sidebar {
            Subscription::batch([
                base,
                iced::event::listen_with(|event, _, _| {
                    use iced::mouse;
                    match event {
                        iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                            Some(Message::SidebarDragMove(position.x))
                        }
                        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                            Some(Message::SidebarDragEnd)
                        }
                        _ => None,
                    }
                }),
            ])
        } else {
            base
        }
    }

    fn header_view(&self) -> Element<'_, Message> {
        container(
            row![
                text(crate::ui::icons::ICON_MUSIC)
                    .font(crate::ui::icons::NERD_FONT_MONO)
                    .color(theme::accent())
                    .size(16),
                Space::with_width(6),
                text("lavanda")
                    .color(theme::accent())
                    .size(16)
                    .font(crate::ui::icons::UI_FONT_BOLD),
            ]
            .align_y(Alignment::Center),
        )
        .style(theme::header)
        .width(Length::Fill)
        .padding([0, 16])
        .into()
    }

    fn advance_track(&mut self, delta: i32) {
        if self.queue.is_empty() {
            return;
        }

        let next_idx = if self.shuffle {
            use rand::Rng;
            let current_idx = self
                .current_track
                .as_ref()
                .and_then(|ct| self.queue.iter().position(|t| t.id == ct.id));
            let len = self.queue.len();
            if len == 1 {
                0
            } else {
                let mut rng = rand::thread_rng();
                let mut idx = rng.gen_range(0..len);
                if let Some(cur) = current_idx {
                    while idx == cur {
                        idx = rng.gen_range(0..len);
                    }
                }
                idx
            }
        } else {
            let current_idx = self
                .current_track
                .as_ref()
                .and_then(|ct| self.queue.iter().position(|t| t.id == ct.id));
            match current_idx {
                Some(i) => {
                    let new = i as i32 + delta;
                    if new < 0 {
                        self.queue.len() - 1
                    } else {
                        new as usize % self.queue.len()
                    }
                }
                None => 0,
            }
        };

        if let Some(track) = self.queue.get(next_idx).cloned() {
            let cover_data = load_cover(&track.path);
            let track = Track {
                cover_data,
                ..track
            };
            self.audio.send(AudioCommand::Play(track.path.clone()));
            self.current_track = Some(track);
            self.playback_state = PlaybackState::Playing;
            self.position = Duration::ZERO;
            self.notify_mpris_track(PlaybackStatus::Playing);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn music_subfolders(music_dir: &PathBuf) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(music_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    folders.sort();
    folders
}

fn sidebar_width_path() -> PathBuf {
    let xdg = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!(
            "{}/.config",
            std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
        )
    });
    PathBuf::from(xdg).join("lavanda").join("sidebar_width")
}

fn load_sidebar_width() -> f32 {
    std::fs::read_to_string(sidebar_width_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(200.0)
}

fn save_sidebar_width(width: f32) {
    let path = sidebar_width_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(path, width.to_string()).ok();
}

fn build_iced_theme() -> Theme {
    Theme::custom(
        "Omarchy".into(),
        iced::theme::Palette {
            background: theme::base(),
            text: theme::text(),
            primary: theme::accent(),
            success: theme::green(),
            danger: theme::red(),
        },
    )
}

// ── Ponto de entrada iced ─────────────────────────────────────────────────────

pub fn run() -> iced::Result {
    iced::application("lavanda", AppState::update, AppState::view)
        .subscription(AppState::subscription)
        .default_font(iced::Font {
            family: iced::font::Family::Name("JetBrainsMono Nerd Font Mono"),
            weight: iced::font::Weight::Normal,
            stretch: iced::font::Stretch::Normal,
            style: iced::font::Style::Normal,
        })
        .theme(|state: &AppState| state.iced_theme.clone())
        .window(iced::window::Settings {
            size: iced::Size::new(960.0, 640.0),
            min_size: Some(iced::Size::new(700.0, 480.0)),
            ..Default::default()
        })
        .run_with(AppState::new)
}
