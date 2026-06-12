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

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Sidebar,
    TrackList,
}

// ── Mensagens ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum EditField {
    Title,
    Artist,
    Album,
    TrackNumber,
}

#[derive(Debug, Clone)]
pub struct EditState {
    pub track: Track,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: String,
    pub saving: bool,
    pub error: Option<String>,
}

impl EditState {
    pub fn from_track(track: &Track) -> Self {
        EditState {
            track: track.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            track_number: track
                .track_number
                .map(|n| n.to_string())
                .unwrap_or_default(),
            saving: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectFolder(PathBuf),
    FolderScanned(PathBuf, Vec<Track>),

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

    MoveCursor(i32),
    ActivateCursor,
    SwitchFocus(Focus),

    EditCursorTrack,
    TogglePlayOnClick,
    TrackClicked(Track, usize),
    EditField(EditField, String),
    FocusNext,
    FocusPrev,
    SaveMetadata,
    MetadataSaved(Result<Track, String>),
    CancelEdit,

    SearchToggle,
    SearchInput(String),
    ToggleHelp,

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
    pub play_on_click: bool,

    pub folders: Vec<PathBuf>,
    pub selected_folder: Option<PathBuf>,
    pub tracks: Vec<Track>,
    folder_cache: HashMap<PathBuf, Vec<Track>>,

    pub focus: Focus,
    pub sidebar_cursor: usize,
    pub track_cursor: usize,

    pub sidebar_width: f32,
    dragging_sidebar: bool,

    pub iced_theme: iced::Theme,
    loaded_theme_name: String,

    pub strings: &'static crate::locale::Strings,

    pub status: Option<String>,
    pub edit_state: Option<EditState>,
    pub search_query: String,
    pub search_active: bool,
    pub help_visible: bool,

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
        let saved = crate::state::load();
        let folders = music_subfolders(cfg.music_path().as_path());

        let (mpris_cmd_tx, mpris_cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let (mpris_update_tx, mpris_update_rx) = tokio::sync::mpsc::unbounded_channel();
        mpris::launch(mpris_cmd_tx, mpris_update_rx);
        let mpris_cmds = Arc::new(Mutex::new(Some(mpris_cmd_rx)));

        let loaded_theme_name = crate::ui::theme::read_current_theme_name();
        let iced_theme = build_iced_theme();

        // Volume: state.toml overrides config.toml (config is the user default,
        // state tracks what was actually last used).
        let volume = saved.volume.unwrap_or(cfg.volume).clamp(0.0, 1.0);

        // Restore last folder only if it still exists and is still in the list.
        let selected_folder = saved
            .last_folder
            .filter(|p| p.exists() && folders.contains(p));

        let initial_task = if let Some(ref path) = selected_folder {
            let path = path.clone();
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
        } else {
            Task::none()
        };

        let sidebar_cursor = selected_folder
            .as_ref()
            .and_then(|p| folders.iter().position(|f| f == p))
            .unwrap_or(0);

        let state = AppState {
            playback_state: PlaybackState::Stopped,
            current_track: None,
            queue: Vec::new(),
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume,
            shuffle: cfg.shuffle,
            repeat: cfg.repeat,
            play_on_click: cfg.play_on_click,
            folders,
            selected_folder,
            tracks: Vec::new(),
            folder_cache: HashMap::new(),
            focus: Focus::Sidebar,
            sidebar_cursor,
            track_cursor: 0,
            sidebar_width: load_sidebar_width(),
            dragging_sidebar: false,
            iced_theme,
            loaded_theme_name,
            strings: crate::locale::get(),
            status: None,
            edit_state: None,
            search_query: String::new(),
            search_active: false,
            help_visible: false,
            audio,
            audio_events,
            mpris_cmds,
            mpris_update_tx,
        };

        (state, initial_task)
    }

    fn persist_state(&self) {
        crate::state::save(&crate::state::SavedState {
            volume: Some(self.volume),
            last_folder: self.selected_folder.clone(),
        });
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
                art_url: None,
            });
        }
        self.send_mpris(MprisUpdate::Status(status));
    }

    pub fn visible_tracks(&self) -> Vec<&Track> {
        if self.search_query.is_empty() {
            return self.tracks.iter().collect();
        }
        let q = self.search_query.to_lowercase();
        self.tracks
            .iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&q)
                    || t.artist.to_lowercase().contains(&q)
                    || t.album.to_lowercase().contains(&q)
            })
            .collect()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        // While the edit dialog is open, block playback/nav shortcuts.
        // System messages (audio, mpris, theme, drag, dialog actions) pass through.
        if self.edit_state.is_some()
            && !matches!(
                message,
                Message::EditField(..)
                    | Message::SaveMetadata
                    | Message::MetadataSaved(_)
                    | Message::CancelEdit
                    | Message::Audio(_)
                    | Message::Mpris(_)
                    | Message::CheckTheme
                    | Message::SidebarDragStart
                    | Message::SidebarDragMove(_)
                    | Message::SidebarDragEnd
                    | Message::VolumeChanged(_)
                    | Message::ActivateCursor
                    | Message::FocusNext
                    | Message::FocusPrev
            )
        {
            return Task::none();
        }

        // While search is active, block playback shortcuts but keep navigation.
        if self.search_active
            && !matches!(
                message,
                Message::SearchInput(_)
                    | Message::SearchToggle
                    | Message::CancelEdit
                    | Message::MoveCursor(_)
                    | Message::ActivateCursor
                    | Message::TrackClicked(..)
                    | Message::SwitchFocus(_)
                    | Message::Audio(_)
                    | Message::Mpris(_)
                    | Message::CheckTheme
                    | Message::SidebarDragStart
                    | Message::SidebarDragMove(_)
                    | Message::SidebarDragEnd
                    | Message::VolumeChanged(_)
                    | Message::FocusNext
                    | Message::FocusPrev
            )
        {
            return Task::none();
        }

        match message {
            Message::SelectFolder(path) => {
                if let Some(idx) = self.folders.iter().position(|f| f == &path) {
                    self.sidebar_cursor = idx;
                }
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
                    self.track_cursor = 0;
                }
                self.persist_state();
                Task::none()
            }

            Message::CoverLoaded(path, cover) => {
                if let Some(track) = &mut self.current_track {
                    if track.path == path {
                        track.cover_data = cover.clone();
                        if let Some(data) = cover {
                            let cp = cache_cover_path();
                            if let Some(dir) = cp.parent() {
                                std::fs::create_dir_all(dir).ok();
                            }
                            if std::fs::write(&cp, &data).is_ok() {
                                let art_url = format!("file://{}", cp.display());
                                let t = track.clone();
                                self.send_mpris(MprisUpdate::Metadata {
                                    title: t.title,
                                    artist: t.artist,
                                    album: t.album,
                                    duration_us: t.duration.as_micros() as i64,
                                    art_url: Some(art_url),
                                });
                            }
                        }
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
                self.persist_state();
                Task::none()
            }

            Message::VolumeStep(delta) => {
                let v = (self.volume + delta).clamp(0.0, 1.0);
                self.volume = v;
                self.audio.send(AudioCommand::SetVolume(v));
                self.send_mpris(MprisUpdate::Volume(v as f64));
                self.persist_state();
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

            Message::MoveCursor(delta) => {
                match self.focus {
                    Focus::Sidebar => {
                        let len = self.folders.len();
                        if len > 0 {
                            self.sidebar_cursor = (self.sidebar_cursor as i32 + delta)
                                .rem_euclid(len as i32)
                                as usize;
                        }
                    }
                    Focus::TrackList => {
                        let len = self.visible_tracks().len();
                        if len > 0 {
                            self.track_cursor =
                                (self.track_cursor as i32 + delta).rem_euclid(len as i32) as usize;
                        }
                    }
                }
                Task::none()
            }

            Message::ActivateCursor => {
                if self.edit_state.is_some() {
                    return Task::done(Message::SaveMetadata);
                }
                return match self.focus {
                    Focus::Sidebar => {
                        if let Some(path) = self.folders.get(self.sidebar_cursor).cloned() {
                            self.update(Message::SelectFolder(path))
                        } else {
                            Task::none()
                        }
                    }
                    Focus::TrackList => {
                        let track = self
                            .visible_tracks()
                            .get(self.track_cursor)
                            .map(|t| (*t).clone());
                        if let Some(track) = track {
                            self.queue = self.tracks.clone();
                            self.start_playback(track)
                        } else {
                            Task::none()
                        }
                    }
                };
            }

            Message::SwitchFocus(focus) => {
                // Sync cursor to the current active item when switching.
                match focus {
                    Focus::Sidebar => {
                        if let Some(ref sf) = self.selected_folder.clone() {
                            if let Some(idx) = self.folders.iter().position(|f| f == sf) {
                                self.sidebar_cursor = idx;
                            }
                        }
                    }
                    Focus::TrackList => {
                        if let Some(ref ct) = self.current_track.clone() {
                            if let Some(idx) = self.tracks.iter().position(|t| t.id == ct.id) {
                                self.track_cursor = idx;
                            }
                        }
                    }
                }
                self.focus = focus;
                Task::none()
            }

            Message::TogglePlayOnClick => {
                self.play_on_click = !self.play_on_click;
                Task::none()
            }

            Message::TrackClicked(track, idx) => {
                self.track_cursor = idx;
                self.focus = Focus::TrackList;
                if self.play_on_click {
                    self.queue = self.tracks.clone();
                    self.start_playback(track)
                } else {
                    Task::none()
                }
            }

            Message::EditCursorTrack => {
                if self.focus == Focus::TrackList {
                    if let Some(track) = self.tracks.get(self.track_cursor) {
                        self.edit_state = Some(EditState::from_track(track));
                        return iced::widget::text_input::focus(iced::widget::text_input::Id::new(
                            crate::ui::views::dialog::TITLE_INPUT_ID,
                        ));
                    }
                }
                Task::none()
            }

            Message::EditField(field, value) => {
                if let Some(ref mut es) = self.edit_state {
                    match field {
                        EditField::Title => es.title = value,
                        EditField::Artist => es.artist = value,
                        EditField::Album => es.album = value,
                        EditField::TrackNumber => es.track_number = value,
                    }
                    es.error = None;
                }
                Task::none()
            }

            Message::SaveMetadata => {
                let Some(ref mut es) = self.edit_state else {
                    return Task::none();
                };
                es.saving = true;
                es.error = None;

                let path = es.track.path.clone();
                let title = es.title.trim().to_owned();
                let artist = es.artist.trim().to_owned();
                let album = es.album.trim().to_owned();
                let track_number: Option<u32> =
                    es.track_number.trim().parse().ok().filter(|&n: &u32| n > 0);
                let mut updated = es.track.clone();
                updated.title = title.clone();
                updated.artist = artist.clone();
                updated.album = album.clone();
                updated.track_number = track_number;

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::library::write_tags(&path, &title, &artist, &album, track_number)
                                .map(|_| updated)
                                .map_err(|e| e.to_string())
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    Message::MetadataSaved,
                )
            }

            Message::MetadataSaved(result) => {
                match result {
                    Ok(updated) => {
                        if let Some(t) = self.tracks.iter_mut().find(|t| t.path == updated.path) {
                            *t = updated.clone();
                        }
                        if let Some(folder) = self.selected_folder.clone() {
                            if let Some(cached) = self.folder_cache.get_mut(&folder) {
                                if let Some(t) = cached.iter_mut().find(|t| t.path == updated.path)
                                {
                                    *t = updated.clone();
                                }
                            }
                        }
                        if self.current_track.as_ref().map(|t| &t.path) == Some(&updated.path) {
                            self.current_track = Some(updated);
                        }
                        self.edit_state = None;
                    }
                    Err(msg) => {
                        if let Some(ref mut es) = self.edit_state {
                            es.saving = false;
                            es.error = Some(msg);
                        }
                    }
                }
                Task::none()
            }

            Message::CancelEdit => {
                if self.edit_state.is_some() {
                    self.edit_state = None;
                } else if self.search_active {
                    self.search_active = false;
                    self.search_query.clear();
                    self.track_cursor = 0;
                } else {
                    self.help_visible = false;
                }
                Task::none()
            }

            Message::SearchToggle => {
                self.search_active = !self.search_active;
                if self.search_active {
                    self.search_query.clear();
                    self.track_cursor = 0;
                    self.focus = Focus::TrackList;
                    return iced::widget::text_input::focus(iced::widget::text_input::Id::new(
                        "search",
                    ));
                } else {
                    self.search_query.clear();
                    self.track_cursor = 0;
                }
                Task::none()
            }

            Message::SearchInput(q) => {
                self.search_query = q;
                self.track_cursor = 0;
                Task::none()
            }

            Message::ToggleHelp => {
                self.help_visible = !self.help_visible;
                Task::none()
            }

            Message::FocusNext => iced::widget::focus_next(),
            Message::FocusPrev => iced::widget::focus_previous(),

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
                    eprintln!("Audio error: {e}");
                    self.status = Some(e);
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
        let mut main = column![
            self.header_view(),
            views::player::view(self),
            views::library::view(self),
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        if let Some(ref msg) = self.status {
            main = main.push(status_bar_view(msg));
        }

        let base: Element<Message> = container(main)
            .style(|_: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::base())),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        if let Some(overlay) = views::help::view(self) {
            iced::widget::stack![base, overlay].into()
        } else {
            base
        }
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
            iced::keyboard::on_key_press(|key, mods| {
                use iced::keyboard::key::Named;
                use iced::keyboard::Key;
                let seek = crate::config::get().seek_step as i64;
                let vol = crate::config::get().volume_step;
                match key {
                    Key::Named(Named::Space) => Some(Message::PlayPause),
                    Key::Named(Named::ArrowUp) => Some(Message::MoveCursor(-1)),
                    Key::Named(Named::ArrowDown) => Some(Message::MoveCursor(1)),
                    Key::Named(Named::Escape) => Some(Message::CancelEdit),
                    Key::Named(Named::Tab) if mods.shift() => Some(Message::FocusPrev),
                    Key::Named(Named::Tab) => Some(Message::FocusNext),
                    Key::Named(Named::Enter) => Some(Message::ActivateCursor),
                    Key::Named(Named::ArrowRight) if mods.shift() => {
                        Some(Message::SeekRelative(seek))
                    }
                    Key::Named(Named::ArrowLeft) if mods.shift() => {
                        Some(Message::SeekRelative(-seek))
                    }
                    Key::Named(Named::ArrowRight) => Some(Message::SwitchFocus(Focus::TrackList)),
                    Key::Named(Named::ArrowLeft) => Some(Message::SwitchFocus(Focus::Sidebar)),
                    Key::Character(ref c) if c.as_str() == "k" && mods.control() => {
                        Some(Message::ToggleHelp)
                    }
                    Key::Character(ref c) => match c.as_str() {
                        "i" | "I" => Some(Message::TogglePlayOnClick),
                        "m" | "M" => Some(Message::EditCursorTrack),
                        "/" => Some(Message::SearchToggle),
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
        let mut header_row = row![
            text(crate::ui::icons::ICON_MUSIC)
                .font(crate::ui::icons::NERD_FONT_MONO)
                .color(theme::accent())
                .size(16),
            Space::with_width(6),
            text("lavanda")
                .color(theme::accent())
                .size(16)
                .font(crate::ui::icons::UI_FONT_BOLD),
            Space::with_width(Length::Fill),
        ]
        .align_y(Alignment::Center);

        if !self.play_on_click {
            header_row = header_row.push(
                text("󰆽  manual")
                    .font(crate::ui::icons::NERD_FONT_MONO)
                    .color(theme::overlay0())
                    .size(11),
            );
        }

        container(header_row)
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
        send_track_notification(&track.title, &track.artist);
        self.current_track = Some(track);
        self.playback_state = PlaybackState::Playing;
        self.position = Duration::ZERO;
        self.status = None;
        self.notify_mpris_track(PlaybackStatus::Playing);
        self.persist_state();
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

fn status_bar_view(msg: &str) -> Element<'_, Message> {
    container(text(msg).color(theme::red()).size(12))
        .style(theme::header)
        .width(Length::Fill)
        .padding([4, 16])
        .into()
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

fn cache_cover_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(home).join(".cache/lavanda/cover.jpg")
}

fn send_track_notification(title: &str, artist: &str) {
    std::process::Command::new("notify-send")
        .args([
            "lavanda",
            &format!("{title} — {artist}"),
            "--icon=audio-x-generic",
            "--expire-time=3000",
            "--urgency=low",
        ])
        .spawn()
        .ok();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn music_subfolders(music_dir: &std::path::Path) -> Vec<PathBuf> {
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(music_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    subdirs.sort();

    // Include the root dir itself when it contains audio files directly
    // (flat ~/Music layout — no artist/album subdirectories).
    if dir_has_audio(music_dir) {
        let mut all = vec![music_dir.to_path_buf()];
        all.extend(subdirs);
        all
    } else {
        subdirs
    }
}

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "opus", "wav", "aac", "m4a", "wma", "aiff",
];

fn dir_has_audio(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| AUDIO_EXTENSIONS.contains(&x.to_lowercase().as_str()))
                .unwrap_or(false)
        })
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
