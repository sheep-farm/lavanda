use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iced::futures::{SinkExt, Stream};
use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use mpris_server::{LoopStatus, PlaybackStatus};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::audio::mpris;
use crate::audio::{
    AudioCommand, AudioEvent, AudioPlayer, MprisCommand, MprisUpdate, PlaybackState,
};
use crate::library::models::Track;
use crate::library::{load_cover, scan_folder};
use crate::ui::{theme, views};

/// Receptor compartilhado, consumido uma única vez pela subscription.
type Shared<T> = Arc<Mutex<Option<UnboundedReceiver<T>>>>;

// ── Mensagens ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SelectFolder(PathBuf),
    FolderScanned(PathBuf, Vec<Track>),

    PlayTrack(Track),
    CoverLoaded(PathBuf, Option<Vec<u8>>),
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

    Audio(AudioEvent),
    Mpris(MprisCommand),
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
    audio_events: Shared<AudioEvent>,
    mpris_cmds: Shared<MprisCommand>,
    mpris_update_tx: tokio::sync::mpsc::UnboundedSender<MprisUpdate>,
}

impl AppState {
    fn new() -> (Self, Task<Message>) {
        let mut audio = AudioPlayer::spawn();
        let audio_events = Arc::new(Mutex::new(Some(audio.take_events())));

        let cfg = crate::config::get();
        let folders = music_subfolders(&cfg.music_path());

        let (mpris_cmd_tx, mpris_cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let (mpris_update_tx, mpris_update_rx) = tokio::sync::mpsc::unbounded_channel();
        mpris::launch(mpris_cmd_tx, mpris_update_rx);
        let mpris_cmds = Arc::new(Mutex::new(Some(mpris_cmd_rx)));

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
            audio_events,
            mpris_cmds,
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
                        let p = path.clone();
                        let tracks = tokio::task::spawn_blocking(move || scan_folder(&path))
                            .await
                            .unwrap_or_default();
                        (p, tracks)
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
                self.queue = self.tracks.clone();
                self.start_playback(track)
            }

            Message::CoverLoaded(path, cover) => {
                if let Some(track) = &mut self.current_track {
                    if track.path == path {
                        track.cover_data = cover;
                    }
                }
                Task::none()
            }

            Message::PlayPause => match self.playback_state {
                PlaybackState::Playing => {
                    self.audio.send(AudioCommand::Pause);
                    self.playback_state = PlaybackState::Paused;
                    self.send_mpris(MprisUpdate::Status(PlaybackStatus::Paused));
                    Task::none()
                }
                PlaybackState::Paused => {
                    self.audio.send(AudioCommand::Resume);
                    self.playback_state = PlaybackState::Playing;
                    self.send_mpris(MprisUpdate::Status(PlaybackStatus::Playing));
                    Task::none()
                }
                PlaybackState::Stopped => {
                    if let Some(first) = self.tracks.first().cloned() {
                        self.queue = self.tracks.clone();
                        self.start_playback(first)
                    } else {
                        Task::none()
                    }
                }
            },

            Message::NextTrack => self.advance_track(1),
            Message::PreviousTrack => self.advance_track(-1),

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

            Message::Audio(event) => match event {
                AudioEvent::Progress { position, duration } => {
                    self.position = position;
                    self.duration = duration;
                    Task::none()
                }
                AudioEvent::Paused => {
                    self.playback_state = PlaybackState::Paused;
                    Task::none()
                }
                AudioEvent::Stopped => {
                    self.playback_state = PlaybackState::Stopped;
                    self.position = Duration::ZERO;
                    self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                    Task::none()
                }
                AudioEvent::TrackEnded => {
                    if self.repeat {
                        if let Some(t) = self.current_track.clone() {
                            self.audio.send(AudioCommand::Play(t.path));
                            self.notify_mpris_track(PlaybackStatus::Playing);
                        }
                        Task::none()
                    } else {
                        self.advance_auto()
                    }
                }
                AudioEvent::Error(e) => {
                    eprintln!("Erro de áudio: {e}");
                    Task::none()
                }
                AudioEvent::Playing => {
                    self.playback_state = PlaybackState::Playing;
                    Task::none()
                }
            },

            Message::Mpris(cmd) => match cmd {
                MprisCommand::Play => {
                    if !matches!(self.playback_state, PlaybackState::Playing) {
                        self.audio.send(AudioCommand::Resume);
                        self.playback_state = PlaybackState::Playing;
                        self.send_mpris(MprisUpdate::Status(PlaybackStatus::Playing));
                    }
                    Task::none()
                }
                MprisCommand::Pause => {
                    if matches!(self.playback_state, PlaybackState::Playing) {
                        self.audio.send(AudioCommand::Pause);
                        self.playback_state = PlaybackState::Paused;
                        self.send_mpris(MprisUpdate::Status(PlaybackStatus::Paused));
                    }
                    Task::none()
                }
                MprisCommand::PlayPause => {
                    match self.playback_state {
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
                    }
                    Task::none()
                }
                MprisCommand::Next => self.advance_track(1),
                MprisCommand::Previous => self.advance_track(-1),
                MprisCommand::Stop => {
                    self.audio.send(AudioCommand::Stop);
                    self.playback_state = PlaybackState::Stopped;
                    self.position = Duration::ZERO;
                    self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                    Task::none()
                }
            },

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
            // Eventos do áudio e comandos MPRIS chegam por canal: a UI só acorda
            // quando há algo a fazer, sem polling ocioso.
            Subscription::run_with_id(
                "audio-events",
                channel_stream(self.audio_events.clone(), Message::Audio),
            ),
            Subscription::run_with_id(
                "mpris-cmds",
                channel_stream(self.mpris_cmds.clone(), Message::Mpris),
            ),
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

    /// Inicia a reprodução de `track`: dispara o áudio imediatamente e agenda
    /// o carregamento da capa fora da thread de UI.
    fn start_playback(&mut self, track: Track) -> Task<Message> {
        let path = track.path.clone();
        self.audio.send(AudioCommand::Play(path.clone()));
        self.audio.send(AudioCommand::SetVolume(self.volume));
        self.current_track = Some(track);
        self.playback_state = PlaybackState::Playing;
        self.position = Duration::ZERO;
        self.notify_mpris_track(PlaybackStatus::Playing);
        load_cover_task(path)
    }

    /// Avanço automático ao terminar a faixa: em modo sequencial, para no fim
    /// da fila em vez de reiniciá-la; em shuffle, sorteia a próxima.
    fn advance_auto(&mut self) -> Task<Message> {
        if self.queue.is_empty() {
            return Task::none();
        }
        if self.shuffle {
            return self.advance_track(1);
        }
        let current_idx = self
            .current_track
            .as_ref()
            .and_then(|ct| self.queue.iter().position(|t| t.id == ct.id));
        match current_idx {
            Some(i) if i + 1 < self.queue.len() => {
                let track = self.queue[i + 1].clone();
                self.start_playback(track)
            }
            Some(_) => {
                self.audio.send(AudioCommand::Stop);
                self.playback_state = PlaybackState::Stopped;
                self.position = Duration::ZERO;
                self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                Task::none()
            }
            None => {
                let track = self.queue[0].clone();
                self.start_playback(track)
            }
        }
    }

    fn advance_track(&mut self, delta: i32) -> Task<Message> {
        if self.queue.is_empty() {
            return Task::none();
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
            self.start_playback(track)
        } else {
            Task::none()
        }
    }
}

/// Converte um receptor `mpsc` (tomado uma única vez do holder) em um stream de
/// `Message`, fonte de uma subscription dirigida por canal.
fn channel_stream<T>(holder: Shared<T>, map: fn(T) -> Message) -> impl Stream<Item = Message>
where
    T: Send + 'static,
{
    iced::stream::channel(64, move |mut output| async move {
        let Some(mut rx) = holder.lock().unwrap().take() else {
            return;
        };
        while let Some(item) = rx.recv().await {
            if output.send(map(item)).await.is_err() {
                break;
            }
        }
    })
}

/// Carrega a capa de `path` em uma thread de bloqueio e devolve `CoverLoaded`.
fn load_cover_task(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let p = path.clone();
            let cover = tokio::task::spawn_blocking(move || load_cover(&path))
                .await
                .unwrap_or(None);
            (p, cover)
        },
        |(path, cover)| Message::CoverLoaded(path, cover),
    )
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
