use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::futures::{SinkExt, Stream};
use iced::widget::{column, container, row, text, Space, stack};
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

type Shared<T> = Arc<Mutex<Option<UnboundedReceiver<T>>>>;

// ── Enums de navegação ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    Standard,
    Focus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Artists,
    Albums,
    Genres,
    Radios,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
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

#[derive(Debug, Clone)]
pub enum ContextMenuTarget {
    Artist(String),
    Album(String),
    Track(Track),
    MultipleTracks(Vec<Track>),
    Header(crate::persist::TableColumn),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistTab {
    Playlists,
    Autoplaylists,
}

// ── Playlist dialog ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PlaylistDialogMode {
    Create,
    AddTrack(Track),
    Rename(String),
}

#[derive(Debug, Clone)]
pub struct PlaylistDialogState {
    pub mode: PlaylistDialogMode,
    pub name_input: String,
    pub selected_playlist: Option<String>,
    pub add_album: bool,
}

// ── Tag editor ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TagEditorState {
    pub tracks: Vec<Track>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub track_number: String,
    pub disc_number: String,
    pub cover_path: Option<String>,
    pub apply_to_album: bool,
    pub year: String,
    pub apply_title: bool,
    pub apply_artist: bool,
    pub apply_album: bool,
    pub apply_year: bool,
    pub apply_genre: bool,
    pub apply_track_num: bool,
    pub apply_disc_num: bool,
    pub apply_cover: bool,
}

// ── Mensagens ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // Biblioteca
    LibraryScanned(Vec<Track>),
    RescanLibrary,

    // Reprodução
    PlayPause,
    NextTrack,
    PreviousTrack,
    Seek(Duration),
    SeekRelative(i64),
    VolumeChanged(f32),
    VolumeStep(f32),
    ToggleShuffle,
    ToggleRepeat,

    // Sidebar drag
    SidebarDragStart,
    SidebarDragMove(f32),
    SidebarDragEnd,

    // Playlist drag (resize vertical)
    PlaylistDragStart,
    PlaylistDragMove(f32),
    PlaylistDragEnd,

    // Eventos internos
    Audio(AudioEvent),
    Mpris(MprisCommand),
    SpectrumData(Vec<f32>),
    CheckTheme,
    WindowResized(f32, f32),

    // Navegação ViewMode
    SelectViewMode(ViewMode),
    SelectArtist(String),
    SelectAlbum(String),
    SelectGenre(String),
    SelectPlaylist(String),
    SelectPlaylistTab(PlaylistTab),

    // Rádio (radio-browser.info)
    RadioSearchChanged(String),
    RadioSearchSubmit,
    RadioShowTop,
    RadioShowSomaFm,
    RadioCountriesLoaded(Result<Vec<crate::radio::Country>, String>),
    RadioCountrySelected(crate::radio::Country),
    RadioUrlChanged(String),
    RadioPlayUrl,
    RadioResults(Result<Vec<crate::radio::RadioStation>, String>),
    PlayStation(crate::radio::RadioStation),
    ToggleFavoriteStation(crate::radio::RadioStation),
    QuarantineStation(crate::radio::RadioStation),
    CloseRadioError,
    CheckNetwork,
    NetworkStatus(bool),

    // Double-click
    DoubleClickTrack(Track),
    DoubleClickArtist(String),
    DoubleClickAlbum(String),
    DoubleClickGenre(String),
    DoubleClickPlaylist(String),

    // Player navigation
    FocusSongName,
    FocusArtistName,
    FocusAlbumName,

    // Track selection
    SelectTrack(Track),
    ModifiersChanged(iced::keyboard::Modifiers),

    // Busca
    SearchChanged(String),
    ToggleFilterTitle,
    ToggleFilterArtist,
    ToggleFilterAlbum,
    ToggleFilterGenre,

    // Sidebar search
    SidebarSearchChanged(String),

    // Sort
    SortBy(SortColumn),

    // Like
    ToggleLikeTrack(Track),

    // Playlists
    OpenPlaylistDialog(PlaylistDialogMode),
    ClosePlaylistDialog,
    PlaylistInputChanged(String),
    PlaylistDialogSelect(String),
    PlaylistDialogToggleAddAlbum(bool),
    PlaylistDialogSubmit,
    DeletePlaylist(String),
    AddTracksToPlaylist(String, Vec<Track>),
    CreatePlaylistFromContext(String, bool),
    CreatePlaylistWithTracks(String, Vec<Track>),

    // Tag editor
    OpenTagEditor(Vec<Track>),
    CloseTagEditor,
    UpdateTagFieldTitle(String),
    UpdateTagFieldArtist(String),
    UpdateTagFieldAlbum(String),
    UpdateTagFieldGenre(String),
    UpdateTagFieldTrackNumber(String),
    UpdateTagFieldDiscNumber(String),
    UpdateTagFieldYear(String),
    UpdateTagFieldCoverPath(String),
    UpdateTagFieldApplyToAlbum(bool),
    ToggleTagFieldApplyTitle(bool),
    ToggleTagFieldApplyArtist(bool),
    ToggleTagFieldApplyAlbum(bool),
    ToggleTagFieldApplyYear(bool),
    ToggleTagFieldApplyGenre(bool),
    ToggleTagFieldApplyTrackNum(bool),
    ToggleTagFieldApplyDiscNum(bool),
    ToggleTagFieldApplyCover(bool),
    SaveTags,
    SearchCoverOnline,

    // Contexto
    ToggleContextMenu(Option<ContextMenuTarget>),
    HideAlbumOrArtist(String, bool),
    RestoreHiddenItems,

    // Hover / UI
    HoverTracklist(bool),
    HoverSidebarList(bool),

    // Group by album
    ToggleGroupByAlbum,

    // Columns
    ToggleColumnVisibility(crate::persist::TableColumn),
    MoveColumnLeft(crate::persist::TableColumn),
    MoveColumnRight(crate::persist::TableColumn),

    // Teclado (atalhos especiais do omatunes)
    KeyPressed(iced::keyboard::Key),
    KeyboardLike,
    KeyboardEdit,
    KeyboardAdd,
    KeyboardArrowUp,
    KeyboardArrowDown,

    // Help / shortcuts
    OpenShortcuts,
    CloseShortcuts,

    // Layout / espectro (lavanda original)
    ToggleSpectrum,
    ToggleLayout,

    // Open local folder
    OpenLocalFolder(PathBuf),
}

// ── Estado global ─────────────────────────────────────────────────────────────

pub struct AppState {
    // Áudio
    pub playback_state: PlaybackState,
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: bool,

    // Biblioteca
    pub all_tracks: Vec<Track>,
    pub tracks: Vec<Track>,

    // Rádio
    pub radio_results: Vec<crate::radio::RadioStation>,
    pub radio_search: String,
    /// Campo de entrada manual de estação por URL (.pls/.m3u ou stream direto).
    pub radio_url_input: String,
    pub radio_loading: bool,
    pub radio_error: Option<String>,
    /// Diálogo de erro de reprodução: (estação, mensagem). Permite quarentena.
    pub radio_error_dialog: Option<(crate::radio::RadioStation, String)>,
    pub current_station: Option<crate::radio::RadioStation>,
    pub stream_title: Option<String>,
    /// Países disponíveis (seletor da aba Radios); carregado sob demanda.
    pub radio_countries: Vec<crate::radio::Country>,
    pub radio_country: Option<crate::radio::Country>,
    /// Conectividade — a aba Radios fica desabilitada quando offline.
    pub online: bool,

    // ViewMode
    pub view_mode: ViewMode,
    pub selected_artist: Option<String>,
    pub selected_album: Option<String>,
    pub selected_genre: Option<String>,
    pub selected_playlist: Option<String>,
    pub playlist_tab: PlaylistTab,

    // Busca
    pub search_query: String,
    pub filter_title: bool,
    pub filter_artist: bool,
    pub filter_album: bool,
    pub filter_genre: bool,
    pub sidebar_search: String,

    // Seleção
    pub selected_track: Option<Track>,
    pub selected_tracks: Vec<Track>,
    pub last_clicked_track: Option<Track>,
    pub modifiers: iced::keyboard::Modifiers,

    // Double-click detection
    pub last_click_track: Option<(i64, Instant)>,
    pub last_click_artist: Option<(String, Instant)>,
    pub last_click_album: Option<(String, Instant)>,
    pub last_click_genre: Option<(String, Instant)>,
    pub last_click_playlist: Option<(String, Instant)>,

    // Sort
    pub sort_column: Option<SortColumn>,
    pub sort_ascending: bool,

    // Group
    pub group_by_album: bool,

    // Tag editor
    pub show_tag_editor: Option<TagEditorState>,

    // Playlist dialog
    pub playlist_dialog: Option<PlaylistDialogState>,

    // Context menu
    pub show_context_menu: Option<ContextMenuTarget>,

    // Hidden items
    pub hidden_artists_albums: Vec<(String, bool)>,

    // Playlist panel height
    pub playlist_height: f32,
    pub playlist_height_initialized: bool,
    pub dragging_playlist_split: bool,
    pub window_height: f32,

    // Sidebar
    pub sidebar_width: f32,
    dragging_sidebar: bool,
    pub is_hovering_tracklist: bool,
    pub is_hovering_sidebar_list: bool,

    // Shortcuts modal
    pub show_shortcuts: bool,

    // Layout / espectro (lavanda)
    pub layout: Layout,
    pub spectrum: Vec<f32>,
    pub show_spectrum: bool,

    // Tema
    pub iced_theme: Theme,
    loaded_theme_name: String,

    // i18n
    pub strings: &'static crate::locale::Strings,

    // Canais internos
    audio: AudioPlayer,
    audio_events: Shared<AudioEvent>,
    spectrum_rx: Shared<Vec<f32>>,
    mpris_cmds: Shared<MprisCommand>,
    mpris_update_tx: tokio::sync::mpsc::UnboundedSender<MprisUpdate>,
}

impl AppState {
    fn new() -> (Self, Task<Message>) {
        let mut audio = AudioPlayer::spawn();
        let audio_events = Arc::new(Mutex::new(Some(audio.take_events())));

        let (spectrum_tx, spectrum_rx) = tokio::sync::mpsc::unbounded_channel();
        crate::audio::spectrum::launch(audio.viz_buf.clone(), spectrum_tx);
        let spectrum_rx = Arc::new(Mutex::new(Some(spectrum_rx)));

        let cfg = crate::config::get();
        let saved = crate::state::load();

        let (mpris_cmd_tx, mpris_cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let (mpris_update_tx, mpris_update_rx) = tokio::sync::mpsc::unbounded_channel();
        mpris::launch(mpris_cmd_tx, mpris_update_rx);
        let mpris_cmds = Arc::new(Mutex::new(Some(mpris_cmd_rx)));

        let loaded_theme_name = crate::ui::theme::read_current_theme_name();
        let iced_theme = build_iced_theme();
        let volume = saved.volume.unwrap_or(cfg.volume).clamp(0.0, 1.0);

        let hidden = crate::persist::get(|db| db.hidden_artists_albums.clone());

        let music_dir = cfg.music_path();
        let scan_task = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || scan_folder(&music_dir))
                    .await
                    .unwrap_or_default()
            },
            Message::LibraryScanned,
        );

        let state = AppState {
            playback_state: PlaybackState::Stopped,
            current_track: None,
            queue: Vec::new(),
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume,
            shuffle: cfg.shuffle,
            repeat: cfg.repeat,
            all_tracks: Vec::new(),
            tracks: Vec::new(),
            radio_results: Vec::new(),
            radio_search: String::new(),
            radio_url_input: String::new(),
            radio_loading: false,
            radio_error: None,
            radio_error_dialog: None,
            current_station: None,
            stream_title: None,
            radio_countries: Vec::new(),
            radio_country: None,
            online: true,
            view_mode: ViewMode::Artists,
            selected_artist: None,
            selected_album: None,
            selected_genre: None,
            selected_playlist: None,
            playlist_tab: PlaylistTab::Playlists,
            search_query: String::new(),
            filter_title: true,
            filter_artist: true,
            filter_album: true,
            filter_genre: true,
            sidebar_search: String::new(),
            selected_track: None,
            selected_tracks: Vec::new(),
            last_clicked_track: None,
            modifiers: Default::default(),
            last_click_track: None,
            last_click_artist: None,
            last_click_album: None,
            last_click_genre: None,
            last_click_playlist: None,
            sort_column: None,
            sort_ascending: true,
            group_by_album: false,
            show_tag_editor: None,
            playlist_dialog: None,
            show_context_menu: None,
            hidden_artists_albums: hidden,
            playlist_height: 141.0,
            playlist_height_initialized: false,
            dragging_playlist_split: false,
            window_height: 640.0,
            sidebar_width: load_sidebar_width(),
            dragging_sidebar: false,
            is_hovering_tracklist: false,
            is_hovering_sidebar_list: false,
            show_shortcuts: false,
            layout: Layout::Standard,
            spectrum: vec![0.0; crate::audio::spectrum::NUM_BARS],
            show_spectrum: true,
            iced_theme,
            loaded_theme_name,
            strings: crate::locale::get(),
            audio,
            audio_events,
            spectrum_rx,
            mpris_cmds,
            mpris_update_tx,
        };

        // Verifica conectividade no startup (a aba Radios reflete o resultado).
        let init = Task::batch([scan_task, Task::done(Message::CheckNetwork)]);
        (state, init)
    }

    fn persist_state(&self) {
        crate::state::save(&crate::state::SavedState {
            volume: Some(self.volume),
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

    // ── Computed lists ────────────────────────────────────────────────────────

    pub fn artists(&self) -> Vec<String> {
        let query = self.sidebar_search.to_lowercase();
        let mut artists: Vec<String> = self.all_tracks.iter()
            .map(|t| if t.artist.trim().is_empty() { "Unknown Artist".to_string() } else { t.artist.clone() })
            .collect();
        artists.sort();
        artists.dedup();
        if !query.is_empty() {
            artists.retain(|a| a.to_lowercase().contains(&query));
        }
        artists.retain(|a| !self.hidden_artists_albums.contains(&(a.clone(), true)));
        artists
    }

    pub fn albums(&self) -> Vec<String> {
        let query = self.sidebar_search.to_lowercase();
        let mut albums: Vec<String> = self.all_tracks.iter()
            .map(|t| if t.album.trim().is_empty() { "Unknown Album".to_string() } else { t.album.clone() })
            .collect();
        albums.sort();
        albums.dedup();
        if !query.is_empty() {
            albums.retain(|a| a.to_lowercase().contains(&query));
        }
        albums.retain(|a| !self.hidden_artists_albums.contains(&(a.clone(), false)));
        albums
    }

    pub fn genres(&self) -> Vec<String> {
        let query = self.sidebar_search.to_lowercase();
        let mut genres: Vec<String> = self.all_tracks.iter()
            .map(|t| if t.genre.trim().is_empty() { "Unknown Genre".to_string() } else { t.genre.clone() })
            .collect();
        genres.sort();
        genres.dedup();
        if !query.is_empty() {
            genres.retain(|g| g.to_lowercase().contains(&query));
        }
        genres
    }

    // ── Filtragem e ordenação ─────────────────────────────────────────────────

    pub fn update_filtered_tracks(&mut self) {
        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            self.tracks = self.all_tracks.iter().filter(|t| {
                (self.filter_title && t.title.to_lowercase().contains(&q))
                    || (self.filter_artist && t.artist.to_lowercase().contains(&q))
                    || (self.filter_album && t.album.to_lowercase().contains(&q))
                    || (self.filter_genre && t.genre.to_lowercase().contains(&q))
            }).cloned().collect();
        } else if let Some(ref playlist_name) = self.selected_playlist.clone() {
            if playlist_name == "Liked Songs" {
                self.tracks = self.all_tracks.iter().filter(|t| t.liked).cloned().collect();
            } else if playlist_name == "Most Played" {
                let mut temp = self.all_tracks.clone();
                temp.sort_by(|a, b| b.play_count.cmp(&a.play_count));
                self.tracks = temp.into_iter().filter(|t| t.play_count > 0).collect();
                return;
            } else if playlist_name == "Recently Played" {
                let rp = crate::persist::get(|db| db.recently_played.clone());
                let mut temp = Vec::new();
                for (path, date_str) in rp {
                    if let Some(mut t) = self.all_tracks.iter().find(|t| t.path == path).cloned() {
                        t.date_played = Some(date_str);
                        temp.push(t);
                    }
                }
                self.tracks = temp;
                return;
            } else {
                let paths = crate::persist::get(|db| db.playlists.get(playlist_name).cloned().unwrap_or_default());
                self.tracks = self.all_tracks.iter().filter(|t| paths.contains(&t.path)).cloned().collect();
            }
        } else {
            match self.view_mode {
                ViewMode::Artists => {
                    if let Some(ref artist) = self.selected_artist.clone() {
                        self.tracks = self.all_tracks.iter().filter(|t| {
                            let a = if t.artist.trim().is_empty() { "Unknown Artist" } else { &t.artist };
                            a == artist
                        }).cloned().collect();
                    } else {
                        self.tracks = Vec::new();
                    }
                }
                ViewMode::Albums => {
                    if let Some(ref album) = self.selected_album.clone() {
                        self.tracks = self.all_tracks.iter().filter(|t| {
                            let al = if t.album.trim().is_empty() { "Unknown Album" } else { &t.album };
                            al == album
                        }).cloned().collect();
                    } else {
                        self.tracks = Vec::new();
                    }
                }
                ViewMode::Genres => {
                    if let Some(ref genre) = self.selected_genre.clone() {
                        self.tracks = self.all_tracks.iter().filter(|t| {
                            let g = if t.genre.trim().is_empty() { "Unknown Genre" } else { &t.genre };
                            g == genre
                        }).cloned().collect();
                    } else {
                        self.tracks = Vec::new();
                    }
                }
                ViewMode::Radios => {
                    // A aba Radios usa seu próprio painel; a lista de faixas fica vazia.
                    self.tracks = Vec::new();
                }
            }
        }

        if let Some(col) = self.sort_column {
            self.tracks.sort_by(|a, b| {
                let cmp = match col {
                    SortColumn::TrackNumber => a.track_number.unwrap_or(u32::MAX).cmp(&b.track_number.unwrap_or(u32::MAX)),
                    SortColumn::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                    SortColumn::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
                    SortColumn::Album => a.album.to_lowercase().cmp(&b.album.to_lowercase()),
                    SortColumn::Genre => a.genre.to_lowercase().cmp(&b.genre.to_lowercase()),
                    SortColumn::Year => a.year.unwrap_or(u32::MAX).cmp(&b.year.unwrap_or(u32::MAX)),
                    SortColumn::DiscNumber => a.disc_number.unwrap_or(u32::MAX).cmp(&b.disc_number.unwrap_or(u32::MAX)),
                    SortColumn::Duration => a.duration.cmp(&b.duration),
                    SortColumn::Plays => a.play_count.cmp(&b.play_count),
                    SortColumn::DatePlayed => a.date_played.as_deref().unwrap_or("").cmp(b.date_played.as_deref().unwrap_or("")),
                };
                if self.sort_ascending { cmp } else { cmp.reverse() }
            });
        }
    }

    pub fn calculate_scroll_offset(&self, track_id: i64) -> Option<f32> {
        let row_h = 34.0;
        if self.group_by_album {
            let mut y = 0.0;
            let mut groups: Vec<(String, Vec<&Track>)> = Vec::new();
            for t in &self.tracks {
                if let Some(last) = groups.last_mut() {
                    if last.0 == t.album { last.1.push(t); continue; }
                }
                groups.push((t.album.clone(), vec![t]));
            }
            for (_album, tracks) in groups {
                if tracks.iter().any(|t| t.id == track_id) {
                    let idx = tracks.iter().position(|t| t.id == track_id).unwrap();
                    y += 28.0 + idx as f32 * row_h;
                    return Some(y);
                }
                y += 28.0 + tracks.len() as f32 * row_h + 8.0;
            }
            None
        } else {
            self.tracks.iter().position(|t| t.id == track_id).map(|i| i as f32 * row_h)
        }
    }

    // ── Update ────────────────────────────────────────────────────────────────

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {

            Message::LibraryScanned(tracks) => {
                self.all_tracks = tracks;
                // Auto-select first artist
                if self.selected_artist.is_none() {
                    self.selected_artist = self.artists().first().cloned();
                }
                self.update_filtered_tracks();
                self.persist_state();
                Task::none()
            }

            Message::RescanLibrary => {
                let music_dir = crate::config::get().music_path();
                Task::perform(async move { scan_folder(&music_dir) }, Message::LibraryScanned)
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
                    if let Some(ref sel) = self.selected_track.clone() {
                        self.queue = self.tracks.clone();
                        self.play_track_internal(sel.clone())
                    } else if let Some(first) = self.tracks.first().cloned() {
                        self.queue = self.tracks.clone();
                        self.play_track_internal(first)
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
                    self.position.saturating_sub(Duration::from_secs(delta_secs.unsigned_abs()))
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
                let loop_status = if self.repeat { LoopStatus::Track } else { LoopStatus::None };
                self.send_mpris(MprisUpdate::Loop(loop_status));
                Task::none()
            }

            Message::SidebarDragStart => { self.dragging_sidebar = true; Task::none() }
            Message::SidebarDragMove(x) => { self.sidebar_width = x.clamp(120.0, 400.0); Task::none() }
            Message::SidebarDragEnd => {
                self.dragging_sidebar = false;
                save_sidebar_width(self.sidebar_width);
                Task::none()
            }

            Message::PlaylistDragStart => { self.dragging_playlist_split = true; Task::none() }
            Message::PlaylistDragMove(y) => {
                self.playlist_height = (self.window_height - y - 60.0).clamp(50.0, self.window_height - 200.0);
                Task::none()
            }
            Message::PlaylistDragEnd => { self.dragging_playlist_split = false; Task::none() }

            Message::WindowResized(_w, h) => {
                self.window_height = h;
                if !self.playlist_height_initialized {
                    self.playlist_height = ((h - 212.0) * 0.33).max(50.0);
                    self.playlist_height_initialized = true;
                }
                Task::none()
            }

            // ── ViewMode ──────────────────────────────────────────────────────

            Message::SelectViewMode(mode) => {
                self.view_mode = mode;
                self.selected_playlist = None;
                self.selected_artist = None;
                self.selected_album = None;
                self.selected_genre = None;
                self.search_query.clear();
                match mode {
                    ViewMode::Artists => { self.selected_artist = self.artists().first().cloned(); }
                    ViewMode::Albums => { self.selected_album = self.albums().first().cloned(); }
                    ViewMode::Genres => { self.selected_genre = self.genres().first().cloned(); }
                    ViewMode::Radios => {
                        // Carrega a lista de países uma vez (para o seletor).
                        let load_countries = if self.radio_countries.is_empty() {
                            Some(Task::perform(
                                async {
                                    tokio::task::spawn_blocking(crate::radio::countries)
                                        .await
                                        .unwrap_or_else(|e| Err(anyhow::anyhow!(e.to_string())))
                                        .map_err(|e| e.to_string())
                                },
                                Message::RadioCountriesLoaded,
                            ))
                        } else {
                            None
                        };
                        // Mostra os favoritos; se não há resultados ainda, busca o top.
                        let load_top = if self.radio_results.is_empty() && !self.radio_loading {
                            Some(Task::done(Message::RadioShowTop))
                        } else {
                            None
                        };
                        return match (load_countries, load_top) {
                            (Some(a), Some(b)) => Task::batch([a, b]),
                            (Some(a), None) | (None, Some(a)) => a,
                            (None, None) => Task::none(),
                        };
                    }
                }
                self.update_filtered_tracks();
                Task::none()
            }

            // ── Rádio ───────────────────────────────────────────────────────────

            Message::RadioSearchChanged(q) => { self.radio_search = q; Task::none() }

            Message::RadioUrlChanged(u) => { self.radio_url_input = u; Task::none() }

            Message::RadioPlayUrl => {
                let url = self.radio_url_input.trim().to_string();
                if url.is_empty() {
                    return Task::none();
                }
                self.radio_url_input.clear();
                let station = crate::radio::RadioStation::from_url(&url);
                self.play_station_internal(station)
            }

            Message::RadioSearchSubmit => {
                let query = self.radio_search.trim().to_string();
                if query.is_empty() {
                    return Task::done(Message::RadioShowTop);
                }
                self.radio_loading = true;
                self.radio_error = None;
                self.radio_country = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || crate::radio::search(&query))
                            .await
                            .unwrap_or_else(|e| Err(anyhow::anyhow!(e.to_string())))
                            .map_err(|e| e.to_string())
                    },
                    Message::RadioResults,
                )
            }

            Message::RadioShowTop => {
                self.radio_loading = true;
                self.radio_error = None;
                self.radio_country = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(|| crate::radio::top(100))
                            .await
                            .unwrap_or_else(|e| Err(anyhow::anyhow!(e.to_string())))
                            .map_err(|e| e.to_string())
                    },
                    Message::RadioResults,
                )
            }

            Message::RadioShowSomaFm => {
                self.radio_loading = true;
                self.radio_error = None;
                self.radio_country = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(crate::radio::somafm)
                            .await
                            .unwrap_or_else(|e| Err(anyhow::anyhow!(e.to_string())))
                            .map_err(|e| e.to_string())
                    },
                    Message::RadioResults,
                )
            }

            Message::RadioCountriesLoaded(result) => {
                if let Ok(list) = result {
                    self.radio_countries = list;
                }
                Task::none()
            }

            Message::RadioCountrySelected(country) => {
                self.radio_country = Some(country.clone());
                self.radio_loading = true;
                self.radio_error = None;
                let code = country.code;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || crate::radio::by_country(&code))
                            .await
                            .unwrap_or_else(|e| Err(anyhow::anyhow!(e.to_string())))
                            .map_err(|e| e.to_string())
                    },
                    Message::RadioResults,
                )
            }

            Message::RadioResults(result) => {
                self.radio_loading = false;
                match result {
                    Ok(mut stations) => {
                        stations.retain(|s| !crate::persist::is_quarantined(s));
                        self.radio_results = stations;
                        self.radio_error = None;
                    }
                    Err(e) => { self.radio_error = Some(e); }
                }
                Task::none()
            }

            Message::PlayStation(station) => self.play_station_internal(station),

            Message::ToggleFavoriteStation(station) => {
                crate::persist::toggle_radio_favorite(&station);
                Task::none()
            }

            Message::QuarantineStation(station) => {
                crate::persist::quarantine_station(&station);
                // Some da lista atual e fecha o diálogo de erro.
                let key_uuid = station.stationuuid.clone();
                let key_url = station.url.clone();
                self.radio_results.retain(|s| {
                    if !key_uuid.is_empty() { s.stationuuid != key_uuid } else { s.url != key_url }
                });
                self.radio_error_dialog = None;
                Task::none()
            }

            Message::CloseRadioError => { self.radio_error_dialog = None; Task::none() }

            Message::CheckNetwork => Task::perform(
                async {
                    tokio::task::spawn_blocking(crate::radio::is_online)
                        .await
                        .unwrap_or(false)
                },
                Message::NetworkStatus,
            ),

            Message::NetworkStatus(online) => {
                self.online = online;
                // Saiu do offline e a lista está vazia? Carrega o top.
                if online
                    && self.view_mode == ViewMode::Radios
                    && self.radio_results.is_empty()
                    && !self.radio_loading
                {
                    return Task::done(Message::RadioShowTop);
                }
                Task::none()
            }

            Message::SelectArtist(artist) => {
                let now = Instant::now();
                if let Some((ref prev, last)) = self.last_click_artist {
                    if prev == &artist && now.duration_since(last) < Duration::from_millis(350) {
                        self.last_click_artist = None;
                        return Task::done(Message::DoubleClickArtist(artist));
                    }
                }
                self.last_click_artist = Some((artist.clone(), now));
                self.selected_artist = Some(artist);
                self.selected_playlist = None;
                self.selected_album = None;
                self.search_query.clear();
                self.update_filtered_tracks();
                Task::none()
            }

            Message::SelectAlbum(album) => {
                let now = Instant::now();
                if let Some((ref prev, last)) = self.last_click_album {
                    if prev == &album && now.duration_since(last) < Duration::from_millis(350) {
                        self.last_click_album = None;
                        return Task::done(Message::DoubleClickAlbum(album));
                    }
                }
                self.last_click_album = Some((album.clone(), now));
                self.selected_album = Some(album);
                self.selected_playlist = None;
                self.selected_artist = None;
                self.search_query.clear();
                self.update_filtered_tracks();
                Task::none()
            }

            Message::SelectGenre(genre) => {
                let now = Instant::now();
                if let Some((ref prev, last)) = self.last_click_genre {
                    if prev == &genre && now.duration_since(last) < Duration::from_millis(350) {
                        self.last_click_genre = None;
                        return Task::done(Message::DoubleClickGenre(genre));
                    }
                }
                self.last_click_genre = Some((genre.clone(), now));
                self.selected_genre = Some(genre);
                self.selected_playlist = None;
                self.selected_artist = None;
                self.selected_album = None;
                self.search_query.clear();
                self.update_filtered_tracks();
                Task::none()
            }

            Message::SelectPlaylist(name) => {
                let now = Instant::now();
                if let Some((ref prev, last)) = self.last_click_playlist {
                    if prev == &name && now.duration_since(last) < Duration::from_millis(350) {
                        self.last_click_playlist = None;
                        return Task::done(Message::DoubleClickPlaylist(name));
                    }
                }
                self.last_click_playlist = Some((name.clone(), now));
                self.selected_playlist = Some(name);
                self.search_query.clear();
                self.update_filtered_tracks();
                Task::none()
            }

            Message::SelectPlaylistTab(tab) => {
                self.playlist_tab = tab;
                self.selected_artist = None;
                self.selected_album = None;
                self.selected_genre = None;
                self.search_query.clear();
                match tab {
                    PlaylistTab::Playlists => {
                        let playlists = crate::persist::get(|db| db.playlists.keys().cloned().collect::<Vec<_>>());
                        self.selected_playlist = playlists.first().cloned();
                    }
                    PlaylistTab::Autoplaylists => {
                        self.selected_playlist = Some("Liked Songs".to_string());
                    }
                }
                self.update_filtered_tracks();
                Task::none()
            }

            // ── Double-click ──────────────────────────────────────────────────

            Message::DoubleClickTrack(track) => {
                self.selected_track = Some(track.clone());
                self.queue = self.tracks.clone();
                self.play_track_internal(track)
            }

            Message::DoubleClickArtist(artist) => {
                self.view_mode = ViewMode::Artists;
                self.selected_artist = Some(artist);
                self.selected_playlist = None;
                self.selected_album = None;
                self.search_query.clear();
                self.update_filtered_tracks();
                self.shuffle = true;
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                let mut shuffled = self.tracks.clone();
                shuffled.shuffle(&mut rng);
                self.queue = shuffled.clone();
                if let Some(first) = shuffled.first().cloned() { self.play_track_internal(first) } else { Task::none() }
            }

            Message::DoubleClickAlbum(album) => {
                self.view_mode = ViewMode::Albums;
                self.selected_album = Some(album);
                self.selected_playlist = None;
                self.selected_artist = None;
                self.search_query.clear();
                self.update_filtered_tracks();
                self.tracks.sort_by_key(|t| t.track_number.unwrap_or(u32::MAX));
                self.queue = self.tracks.clone();
                if let Some(first) = self.tracks.first().cloned() { self.play_track_internal(first) } else { Task::none() }
            }

            Message::DoubleClickGenre(genre) => {
                self.view_mode = ViewMode::Genres;
                self.selected_genre = Some(genre);
                self.selected_playlist = None;
                self.search_query.clear();
                self.update_filtered_tracks();
                self.queue = self.tracks.clone();
                if let Some(first) = self.tracks.first().cloned() { self.play_track_internal(first) } else { Task::none() }
            }

            Message::DoubleClickPlaylist(playlist) => {
                // Playlists customizadas: clique duplo renomeia. Autoplaylists: toca.
                let is_custom = crate::persist::get(|db| db.playlists.contains_key(&playlist));
                if is_custom {
                    return Task::done(Message::OpenPlaylistDialog(PlaylistDialogMode::Rename(playlist)));
                }
                self.selected_playlist = Some(playlist);
                self.search_query.clear();
                self.update_filtered_tracks();
                self.queue = self.tracks.clone();
                if let Some(first) = self.tracks.first().cloned() { self.play_track_internal(first) } else { Task::none() }
            }

            // ── Player navigation ─────────────────────────────────────────────

            Message::FocusSongName | Message::FocusAlbumName => {
                if let Some(current) = self.current_track.clone() {
                    self.view_mode = ViewMode::Albums;
                    self.selected_album = Some(current.album.clone());
                    self.selected_playlist = None;
                        self.selected_artist = None;
                    self.search_query.clear();
                    self.update_filtered_tracks();
                    self.selected_track = Some(current.clone());
                    if let Some(y) = self.calculate_scroll_offset(current.id) {
                        return iced::widget::scrollable::scroll_to(
                            iced::widget::scrollable::Id::new("tracklist_scroll"),
                            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: (y - 120.0).max(0.0) },
                        );
                    }
                }
                Task::none()
            }

            Message::FocusArtistName => {
                if let Some(current) = self.current_track.clone() {
                    self.view_mode = ViewMode::Artists;
                    self.selected_artist = Some(current.artist.clone());
                    self.selected_playlist = None;
                        self.selected_album = None;
                    self.search_query.clear();
                    self.update_filtered_tracks();
                }
                Task::none()
            }

            // ── Track selection ───────────────────────────────────────────────

            Message::SelectTrack(track) => {
                let now = Instant::now();
                if let Some((prev_id, last)) = self.last_click_track {
                    if prev_id == track.id && now.duration_since(last) < Duration::from_millis(350) {
                        self.last_click_track = None;
                        return Task::done(Message::DoubleClickTrack(track));
                    }
                }
                self.last_click_track = Some((track.id, now));
                let cover_data = load_cover(&track.path);
                let track = Track { cover_data, ..track };

                let shift = self.modifiers.shift();
                let ctrl = self.modifiers.control() || self.modifiers.command();

                if ctrl {
                    if self.selected_tracks.iter().any(|t| t.id == track.id) {
                        self.selected_tracks.retain(|t| t.id != track.id);
                    } else {
                        self.selected_tracks.push(track.clone());
                    }
                    self.last_clicked_track = Some(track.clone());
                } else if shift {
                    if let Some(ref start) = self.last_clicked_track {
                        let s = self.tracks.iter().position(|t| t.id == start.id);
                        let e = self.tracks.iter().position(|t| t.id == track.id);
                        if let (Some(s), Some(e)) = (s, e) {
                            let (lo, hi) = if s < e { (s, e) } else { (e, s) };
                            self.selected_tracks = self.tracks[lo..=hi].to_vec();
                        }
                    } else {
                        self.selected_tracks = vec![track.clone()];
                        self.last_clicked_track = Some(track.clone());
                    }
                } else {
                    self.selected_tracks = vec![track.clone()];
                    self.last_clicked_track = Some(track.clone());
                }

                self.selected_track = Some(track);
                Task::none()
            }

            Message::ModifiersChanged(mods) => { self.modifiers = mods; Task::none() }

            // ── Busca ─────────────────────────────────────────────────────────

            Message::SearchChanged(q) => {
                self.search_query = q;
                self.update_filtered_tracks();
                Task::none()
            }

            Message::ToggleFilterTitle => { self.filter_title = !self.filter_title; self.update_filtered_tracks(); Task::none() }
            Message::ToggleFilterArtist => { self.filter_artist = !self.filter_artist; self.update_filtered_tracks(); Task::none() }
            Message::ToggleFilterAlbum => { self.filter_album = !self.filter_album; self.update_filtered_tracks(); Task::none() }
            Message::ToggleFilterGenre => { self.filter_genre = !self.filter_genre; self.update_filtered_tracks(); Task::none() }
            Message::SidebarSearchChanged(q) => { self.sidebar_search = q; Task::none() }

            // ── Sort ──────────────────────────────────────────────────────────

            Message::SortBy(col) => {
                if self.sort_column == Some(col) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(col);
                    self.sort_ascending = true;
                }
                self.update_filtered_tracks();
                Task::none()
            }

            Message::ToggleGroupByAlbum => {
                self.group_by_album = !self.group_by_album;
                Task::none()
            }

            // ── Like ──────────────────────────────────────────────────────────

            Message::ToggleLikeTrack(track) => {
                self.show_context_menu = None;
                let liked = crate::persist::toggle_favorite(track.path.clone());
                for t in self.all_tracks.iter_mut().filter(|t| t.path == track.path) { t.liked = liked; }
                for t in self.tracks.iter_mut().filter(|t| t.path == track.path) { t.liked = liked; }
                if let Some(ref mut ct) = self.current_track { if ct.path == track.path { ct.liked = liked; } }
                if let Some(ref mut st) = self.selected_track { if st.path == track.path { st.liked = liked; } }
                self.update_filtered_tracks();
                Task::none()
            }

            Message::KeyboardLike => {
                if let Some(ref track) = self.current_track.clone() {
                    let mut t = track.clone();
                    t.cover_data = None;
                    return Task::done(Message::ToggleLikeTrack(t));
                }
                Task::none()
            }

            // ── Playlists ─────────────────────────────────────────────────────

            Message::OpenPlaylistDialog(mode) => {
                let initial = match &mode {
                    PlaylistDialogMode::Create => "My Playlist".to_string(),
                    PlaylistDialogMode::AddTrack(_) => String::new(),
                    PlaylistDialogMode::Rename(old) => old.clone(),
                };
                let playlists = crate::persist::get(|db| db.playlists.keys().cloned().collect::<Vec<_>>());
                let first = playlists.first().cloned();
                self.playlist_dialog = Some(PlaylistDialogState {
                    mode,
                    name_input: initial,
                    selected_playlist: first,
                    add_album: false,
                });
                Task::none()
            }

            Message::ClosePlaylistDialog => { self.playlist_dialog = None; Task::none() }

            Message::PlaylistInputChanged(val) => {
                if let Some(ref mut d) = self.playlist_dialog { d.name_input = val; }
                Task::none()
            }

            Message::PlaylistDialogSelect(name) => {
                if let Some(ref mut d) = self.playlist_dialog { d.selected_playlist = Some(name); }
                Task::none()
            }

            Message::PlaylistDialogToggleAddAlbum(val) => {
                if let Some(ref mut d) = self.playlist_dialog { d.add_album = val; }
                Task::none()
            }

            Message::PlaylistDialogSubmit => {
                if let Some(dialog) = self.playlist_dialog.clone() {
                    match dialog.mode {
                        PlaylistDialogMode::Create => {
                            let name = dialog.name_input.trim().to_string();
                            if !name.is_empty() { crate::persist::create_playlist(name); }
                        }
                        PlaylistDialogMode::AddTrack(track) => {
                            if let Some(pl) = dialog.selected_playlist {
                                if dialog.add_album {
                                    let album_tracks: Vec<_> = self.all_tracks.iter()
                                        .filter(|t| t.album == track.album).cloned().collect();
                                    for t in album_tracks { crate::persist::add_to_playlist(pl.clone(), t.path); }
                                } else {
                                    crate::persist::add_to_playlist(pl, track.path);
                                }
                            }
                        }
                        PlaylistDialogMode::Rename(old) => {
                            let new = dialog.name_input.trim().to_string();
                            if !new.is_empty() && new != old {
                                crate::persist::rename_playlist(old.clone(), new.clone());
                                if self.selected_playlist.as_ref() == Some(&old) {
                                    self.selected_playlist = Some(new);
                                }
                            }
                        }
                    }
                    self.playlist_dialog = None;
                    self.update_filtered_tracks();
                }
                Task::none()
            }

            Message::DeletePlaylist(name) => {
                crate::persist::delete_playlist(name.clone());
                if self.selected_playlist.as_ref() == Some(&name) {
                    self.selected_playlist = None;
                }
                self.update_filtered_tracks();
                Task::none()
            }

            Message::AddTracksToPlaylist(pl, tracks) => {
                for t in tracks { crate::persist::add_to_playlist(pl.clone(), t.path); }
                self.show_context_menu = None;
                self.update_filtered_tracks();
                Task::none()
            }

            Message::CreatePlaylistFromContext(target, is_artist) => {
                crate::persist::create_playlist(target.clone());
                let matched: Vec<_> = self.all_tracks.iter()
                    .filter(|t| {
                        if is_artist { t.artist == target } else { t.album == target }
                    })
                    .cloned().collect();
                for t in matched { crate::persist::add_to_playlist(target.clone(), t.path); }
                self.show_context_menu = None;
                self.update_filtered_tracks();
                Task::none()
            }

            Message::CreatePlaylistWithTracks(name, tracks) => {
                crate::persist::create_playlist(name.clone());
                for t in tracks { crate::persist::add_to_playlist(name.clone(), t.path); }
                self.show_context_menu = None;
                self.update_filtered_tracks();
                Task::none()
            }

            // ── Tag editor ────────────────────────────────────────────────────

            Message::OpenTagEditor(tracks) => {
                self.show_context_menu = None;
                if tracks.is_empty() { return Task::none(); }
                let first = &tracks[0];
                let all_same = |f: fn(&Track) -> &str| tracks.iter().all(|t| f(t) == f(first));
                let all_same_opt = |f: fn(&Track) -> Option<u32>| tracks.iter().all(|t| f(t) == f(first));
                self.show_tag_editor = Some(TagEditorState {
                    title: if all_same(|t| &t.title) { first.title.clone() } else { String::new() },
                    artist: if all_same(|t| &t.artist) { first.artist.clone() } else { String::new() },
                    album: if all_same(|t| &t.album) { first.album.clone() } else { String::new() },
                    genre: if all_same(|t| &t.genre) { first.genre.clone() } else { String::new() },
                    track_number: if all_same_opt(|t| t.track_number) { first.track_number.map(|n| n.to_string()).unwrap_or_default() } else { String::new() },
                    disc_number: if all_same_opt(|t| t.disc_number) { first.disc_number.map(|n| n.to_string()).unwrap_or_default() } else { String::new() },
                    year: if all_same_opt(|t| t.year) { first.year.map(|n| n.to_string()).unwrap_or_default() } else { String::new() },
                    cover_path: None,
                    apply_to_album: false,
                    apply_title: false, apply_artist: false, apply_album: false,
                    apply_year: false, apply_genre: false, apply_track_num: false,
                    apply_disc_num: false, apply_cover: false,
                    tracks,
                });
                Task::none()
            }

            Message::CloseTagEditor => { self.show_tag_editor = None; Task::none() }

            Message::UpdateTagFieldTitle(v) => { if let Some(ref mut s) = self.show_tag_editor { s.title = v; s.apply_title = true; } Task::none() }
            Message::UpdateTagFieldArtist(v) => { if let Some(ref mut s) = self.show_tag_editor { s.artist = v; s.apply_artist = true; } Task::none() }
            Message::UpdateTagFieldAlbum(v) => { if let Some(ref mut s) = self.show_tag_editor { s.album = v; s.apply_album = true; } Task::none() }
            Message::UpdateTagFieldGenre(v) => { if let Some(ref mut s) = self.show_tag_editor { s.genre = v; s.apply_genre = true; } Task::none() }
            Message::UpdateTagFieldTrackNumber(v) => { if let Some(ref mut s) = self.show_tag_editor { s.track_number = v; s.apply_track_num = true; } Task::none() }
            Message::UpdateTagFieldDiscNumber(v) => { if let Some(ref mut s) = self.show_tag_editor { s.disc_number = v; s.apply_disc_num = true; } Task::none() }
            Message::UpdateTagFieldYear(v) => { if let Some(ref mut s) = self.show_tag_editor { s.year = v; s.apply_year = true; } Task::none() }
            Message::UpdateTagFieldCoverPath(v) => { if let Some(ref mut s) = self.show_tag_editor { s.cover_path = Some(v); s.apply_cover = true; } Task::none() }
            Message::UpdateTagFieldApplyToAlbum(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_to_album = v; } Task::none() }
            Message::ToggleTagFieldApplyTitle(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_title = v; } Task::none() }
            Message::ToggleTagFieldApplyArtist(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_artist = v; } Task::none() }
            Message::ToggleTagFieldApplyAlbum(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_album = v; } Task::none() }
            Message::ToggleTagFieldApplyYear(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_year = v; } Task::none() }
            Message::ToggleTagFieldApplyGenre(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_genre = v; } Task::none() }
            Message::ToggleTagFieldApplyTrackNum(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_track_num = v; } Task::none() }
            Message::ToggleTagFieldApplyDiscNum(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_disc_num = v; } Task::none() }
            Message::ToggleTagFieldApplyCover(v) => { if let Some(ref mut s) = self.show_tag_editor { s.apply_cover = v; } Task::none() }

            Message::SearchCoverOnline => {
                if let Some(ref s) = self.show_tag_editor {
                    let q = format!("{} {} album art", s.artist, s.album)
                        .chars().map(|c| if c == ' ' { '+' } else { c }).collect::<String>();
                    let url = format!("https://www.google.com/search?q={}&tbm=isch", q);
                    std::process::Command::new("xdg-open").arg(url).spawn().ok();
                }
                Task::none()
            }

            Message::SaveTags => {
                let Some(ref state) = self.show_tag_editor else { return Task::none(); };
                let track_num = state.track_number.trim().parse::<u32>().ok();
                let disc_num = state.disc_number.trim().parse::<u32>().ok();
                let year_num = state.year.trim().parse::<u32>().ok();

                let tracks_to_update: Vec<Track> = if state.apply_to_album {
                    let albums: Vec<_> = state.tracks.iter().map(|t| t.album.clone()).collect();
                    self.all_tracks.iter().filter(|t| albums.contains(&t.album)).cloned().collect()
                } else {
                    state.tracks.clone()
                };

                // Clone state fields needed
                let apply_title = state.apply_title;
                let apply_artist = state.apply_artist;
                let apply_album = state.apply_album;
                let apply_genre = state.apply_genre;
                let apply_track_num = state.apply_track_num;
                let apply_disc_num = state.apply_disc_num;
                let apply_year = state.apply_year;
                let apply_cover = state.apply_cover;
                let title_val = state.title.clone();
                let artist_val = state.artist.clone();
                let album_val = state.album.clone();
                let genre_val = state.genre.clone();
                let cover_path_val = state.cover_path.clone();

                for track in tracks_to_update {
                    let title = if apply_title { &title_val } else { &track.title };
                    let artist = if apply_artist { &artist_val } else { &track.artist };
                    let album = if apply_album { &album_val } else { &track.album };
                    let genre = if apply_genre { &genre_val } else { &track.genre };
                    let tn = if apply_track_num { track_num } else { track.track_number };
                    let dn = if apply_disc_num { disc_num } else { track.disc_number };
                    let yr = if apply_year { year_num } else { track.year };
                    let cp = if apply_cover { cover_path_val.as_deref() } else { None };

                    if let Err(e) = crate::library::write_tags(&track.path, title, artist, album, genre, tn, dn, cp, yr) {
                        eprintln!("Error saving tags for {}: {e}", track.path.display());
                    } else {
                        for t in self.all_tracks.iter_mut().filter(|t| t.path == track.path) {
                            t.title = title.clone();
                            t.artist = artist.clone();
                            t.album = album.clone();
                            t.genre = genre.clone();
                            t.track_number = tn;
                            t.disc_number = dn;
                            t.year = yr;
                        }
                        for t in self.tracks.iter_mut().filter(|t| t.path == track.path) {
                            t.title = title.clone();
                            t.artist = artist.clone();
                            t.album = album.clone();
                            t.genre = genre.clone();
                            t.track_number = tn;
                            t.disc_number = dn;
                            t.year = yr;
                        }
                        if let Some(ref mut ct) = self.current_track {
                            if ct.path == track.path {
                                ct.title = title.clone();
                                ct.artist = artist.clone();
                                ct.album = album.clone();
                            }
                        }
                    }
                }
                self.show_tag_editor = None;
                self.update_filtered_tracks();
                Task::none()
            }

            // ── Keyboard edit helper ──────────────────────────────────────────

            Message::KeyboardEdit => {
                let tracks = if !self.selected_tracks.is_empty() {
                    let mut t = self.selected_tracks.clone();
                    for tr in &mut t { tr.cover_data = None; }
                    t
                } else if let Some(ref track) = self.current_track.clone() {
                    let mut t = track.clone();
                    t.cover_data = None;
                    vec![t]
                } else {
                    Vec::new()
                };
                if !tracks.is_empty() { return Task::done(Message::OpenTagEditor(tracks)); }
                Task::none()
            }

            Message::KeyboardAdd => {
                if let Some(ref track) = self.current_track.clone() {
                    let mut t = track.clone();
                    t.cover_data = None;
                    return Task::done(Message::OpenPlaylistDialog(PlaylistDialogMode::AddTrack(t)));
                }
                Task::none()
            }

            // ── Context menu ──────────────────────────────────────────────────

            Message::ToggleContextMenu(val) => { self.show_context_menu = val; Task::none() }

            Message::HideAlbumOrArtist(name, is_artist) => {
                self.hidden_artists_albums.push((name.clone(), is_artist));
                crate::persist::write(|db| { db.hidden_artists_albums.push((name, is_artist)); });
                self.show_context_menu = None;
                self.selected_artist = None;
                self.selected_album = None;
                self.selected_genre = None;
                self.update_filtered_tracks();
                Task::none()
            }

            Message::RestoreHiddenItems => {
                self.hidden_artists_albums.clear();
                crate::persist::write(|db| { db.hidden_artists_albums.clear(); });
                self.update_filtered_tracks();
                Task::none()
            }

            // ── Hover ─────────────────────────────────────────────────────────

            Message::HoverTracklist(v) => { self.is_hovering_tracklist = v; Task::none() }
            Message::HoverSidebarList(v) => { self.is_hovering_sidebar_list = v; Task::none() }

            // ── Columns ───────────────────────────────────────────────────────

            Message::ToggleColumnVisibility(col) => {
                crate::persist::write(|db| {
                    if db.table_columns.contains(&col) {
                        if db.table_columns.len() > 1 { db.table_columns.retain(|&c| c != col); }
                    } else {
                        db.table_columns.push(col);
                    }
                });
                Task::none()
            }

            Message::MoveColumnLeft(col) => {
                crate::persist::write(|db| {
                    if let Some(pos) = db.table_columns.iter().position(|&c| c == col) {
                        if pos > 0 { db.table_columns.swap(pos, pos - 1); }
                    }
                });
                Task::none()
            }

            Message::MoveColumnRight(col) => {
                crate::persist::write(|db| {
                    if let Some(pos) = db.table_columns.iter().position(|&c| c == col) {
                        if pos + 1 < db.table_columns.len() { db.table_columns.swap(pos, pos + 1); }
                    }
                });
                Task::none()
            }

            // ── Shortcuts ─────────────────────────────────────────────────────

            Message::OpenShortcuts => { self.show_shortcuts = true; Task::none() }
            Message::CloseShortcuts => { self.show_shortcuts = false; Task::none() }

            // ── Layout / espectro (lavanda) ────────────────────────────────────

            Message::ToggleSpectrum => { self.show_spectrum = !self.show_spectrum; Task::none() }

            Message::ToggleLayout => {
                self.layout = match self.layout {
                    Layout::Standard => Layout::Focus,
                    Layout::Focus => Layout::Standard,
                };
                Task::none()
            }

            Message::SpectrumData(bins) => {
                for (a, b) in self.spectrum.iter_mut().zip(bins.iter()) {
                    *a = *a * 0.55 + b * 0.45;
                }
                Task::none()
            }

            // ── Keyboard ─────────────────────────────────────────────────────

            Message::KeyboardArrowUp => {
                if self.is_hovering_tracklist && !self.tracks.is_empty() {
                    let cur_idx = self.selected_track.as_ref()
                        .and_then(|st| self.tracks.iter().position(|t| t.id == st.id));
                    let next_idx = match cur_idx {
                        Some(i) => if i == 0 { self.tracks.len() - 1 } else { i - 1 },
                        None => 0,
                    };
                    if let Some(track) = self.tracks.get(next_idx).cloned() {
                        let cover_data = load_cover(&track.path);
                        let track = Track { cover_data, ..track };
                        self.selected_track = Some(track.clone());
                        self.selected_tracks = vec![track.clone()];
                        self.last_clicked_track = Some(track.clone());
                        if let Some(y) = self.calculate_scroll_offset(track.id) {
                            return iced::widget::scrollable::scroll_to(
                                iced::widget::scrollable::Id::new("tracklist_scroll"),
                                iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: (y - 120.0).max(0.0) },
                            );
                        }
                    }
                }
                Task::none()
            }

            Message::KeyboardArrowDown => {
                if self.is_hovering_tracklist && !self.tracks.is_empty() {
                    let cur_idx = self.selected_track.as_ref()
                        .and_then(|st| self.tracks.iter().position(|t| t.id == st.id));
                    let next_idx = match cur_idx {
                        Some(i) => (i + 1) % self.tracks.len(),
                        None => 0,
                    };
                    if let Some(track) = self.tracks.get(next_idx).cloned() {
                        let cover_data = load_cover(&track.path);
                        let track = Track { cover_data, ..track };
                        self.selected_track = Some(track.clone());
                        self.selected_tracks = vec![track.clone()];
                        self.last_clicked_track = Some(track.clone());
                        if let Some(y) = self.calculate_scroll_offset(track.id) {
                            return iced::widget::scrollable::scroll_to(
                                iced::widget::scrollable::Id::new("tracklist_scroll"),
                                iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: (y - 120.0).max(0.0) },
                            );
                        }
                    }
                }
                Task::none()
            }

            Message::KeyPressed(key) => {
                use iced::keyboard::Key;
                use iced::keyboard::key::Named;
                let seek = crate::config::get().seek_step as i64;
                let vol = crate::config::get().volume_step;

                let has_tag_editor = self.show_tag_editor.is_some();
                let has_playlist_dialog = self.playlist_dialog.is_some();
                let has_shortcuts = self.show_shortcuts;
                let has_context_menu = self.show_context_menu.is_some();
                let ctrl = self.modifiers.control() || self.modifiers.command();

                match key {
                    Key::Named(Named::Escape) => {
                        if self.radio_error_dialog.is_some() { return Task::done(Message::CloseRadioError); }
                        if has_shortcuts { return Task::done(Message::CloseShortcuts); }
                        if has_playlist_dialog { return Task::done(Message::ClosePlaylistDialog); }
                        if has_tag_editor { return Task::done(Message::CloseTagEditor); }
                        if has_context_menu { return Task::done(Message::ToggleContextMenu(None)); }
                    }
                    Key::Named(Named::Enter) => {
                        if has_tag_editor { return Task::done(Message::SaveTags); }
                        if has_playlist_dialog { return Task::done(Message::PlaylistDialogSubmit); }
                        if !has_shortcuts && !has_context_menu {
                            if let Some(ref track) = self.selected_track.clone() {
                                return Task::done(Message::DoubleClickTrack(track.clone()));
                            }
                        }
                    }
                    Key::Named(Named::Space) if !has_playlist_dialog && !has_tag_editor => {
                        return Task::done(Message::PlayPause);
                    }
                    Key::Named(Named::ArrowRight) => return Task::done(Message::SeekRelative(seek)),
                    Key::Named(Named::ArrowLeft) => return Task::done(Message::SeekRelative(-seek)),
                    Key::Named(Named::ArrowUp) => return Task::done(Message::KeyboardArrowUp),
                    Key::Named(Named::ArrowDown) => return Task::done(Message::KeyboardArrowDown),
                    Key::Named(Named::F5) => return Task::done(Message::RescanLibrary),
                    Key::Character(ref c) if ctrl => {
                        if c.as_str() == "k" || c.as_str() == "K" {
                            return if has_shortcuts {
                                Task::done(Message::CloseShortcuts)
                            } else {
                                Task::done(Message::OpenShortcuts)
                            };
                        }
                    }
                    Key::Character(ref c) if !has_playlist_dialog && !has_tag_editor => {
                        match c.as_str() {
                            "n" | "N" => return Task::done(Message::NextTrack),
                            "p" | "P" => return Task::done(Message::PreviousTrack),
                            "s" | "S" => return Task::done(Message::ToggleShuffle),
                            "r" | "R" => return Task::done(Message::ToggleRepeat),
                            "+" | "=" => return Task::done(Message::VolumeStep(vol)),
                            "-" => return Task::done(Message::VolumeStep(-vol)),
                            "/" => return Task::done(Message::SearchChanged(String::new())),
                            "l" | "L" | "f" | "F" => return Task::done(Message::KeyboardLike),
                            "e" | "E" | "m" | "M" => return Task::done(Message::KeyboardEdit),
                            "c" | "C" => return Task::done(Message::OpenPlaylistDialog(PlaylistDialogMode::Create)),
                            "a" | "A" => return Task::done(Message::KeyboardAdd),
                            "t" | "T" => return Task::done(Message::ToggleLayout),
                            "v" | "V" => return Task::done(Message::ToggleSpectrum),
                            "?" => return Task::done(Message::OpenShortcuts),
                            _ => {}
                        }
                    }
                    _ => {}
                }
                Task::none()
            }

            // ── Open local folder ─────────────────────────────────────────────

            Message::OpenLocalFolder(path) => {
                self.show_context_menu = None;
                if let Some(parent) = path.parent() {
                    let folder = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
                    for fm in &["nautilus", "thunar", "dolphin", "nemo", "pcmanfm"] {
                        if std::process::Command::new(fm).arg(&folder).spawn().is_ok() { break; }
                    }
                }
                Task::none()
            }

            // ── Eventos de áudio ──────────────────────────────────────────────

            Message::Audio(event) => match event {
                AudioEvent::Progress { position, duration } => {
                    self.position = position;
                    self.duration = duration;
                    Task::none()
                }
                AudioEvent::Paused => { self.playback_state = PlaybackState::Paused; Task::none() }
                AudioEvent::Stopped => {
                    self.playback_state = PlaybackState::Stopped;
                    self.position = Duration::ZERO;
                    self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                    Task::none()
                }
                AudioEvent::StreamTitle(title) => {
                    let title = title.trim().to_string();
                    self.stream_title = if title.is_empty() { None } else { Some(title) };
                    if let Some(station) = self.current_station.clone() {
                        let now = self.stream_title.clone().unwrap_or_else(|| station.name.clone());
                        self.send_mpris(MprisUpdate::Metadata {
                            title: now,
                            artist: station.name.clone(),
                            album: "Radio".to_string(),
                            duration_us: 0,
                            art_url: None,
                        });
                    }
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
                        let current_idx = self.current_track.as_ref()
                            .and_then(|ct| self.queue.iter().position(|t| t.id == ct.id));
                        let is_last = match current_idx {
                            Some(i) => i + 1 >= self.queue.len(),
                            None => true,
                        };
                        if is_last && !self.shuffle {
                            self.audio.send(AudioCommand::Stop);
                            self.playback_state = PlaybackState::Stopped;
                            self.position = Duration::ZERO;
                            self.send_mpris(MprisUpdate::Status(PlaybackStatus::Stopped));
                            Task::none()
                        } else {
                            self.advance_track(1)
                        }
                    }
                }
                AudioEvent::Error(e) => {
                    if let Some(station) = self.current_station.clone() {
                        // Identifica a estação na log e abre um diálogo (mantém a lista).
                        eprintln!(
                            "Radio error: \"{}\" [{}] codec={} bitrate={} url={} :: {}",
                            station.name,
                            station.countrycode,
                            station.codec,
                            station.bitrate,
                            station.stream_url(),
                            e
                        );
                        self.radio_error_dialog = Some((station, e));
                        self.current_station = None;
                        self.stream_title = None;
                        self.playback_state = PlaybackState::Stopped;
                    } else {
                        eprintln!("Audio error: {e}");
                    }
                    Task::none()
                }
                AudioEvent::Playing => { self.playback_state = PlaybackState::Playing; Task::none() }
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

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let main: Element<Message> = match self.layout {
            Layout::Focus => column![self.header_view(), views::focus::view(self)]
                .spacing(0).width(Length::Fill).height(Length::Fill).into(),
            Layout::Standard => {
                let main_col = column![
                    self.header_view(),
                    views::player::view(self),
                    views::library::view(self),
                ]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill);

                container(main_col)
                    .style(|_: &Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(theme::base())),
                        ..Default::default()
                    })
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
        };

        let mut view_stack = stack![main];

        // Tag editor overlay
        if let Some(ref editor_state) = self.show_tag_editor {
            view_stack = view_stack.push(crate::ui::components::tag_editor::view(editor_state));
        }

        // Playlist dialog overlay
        if let Some(ref pd) = self.playlist_dialog {
            view_stack = view_stack.push(crate::ui::components::playlist_dialog::view(pd));
        }

        // Shortcuts overlay
        if self.show_shortcuts {
            view_stack = view_stack.push(self.shortcuts_modal_view());
        }

        // Context menu overlay
        if let Some(ref target) = self.show_context_menu {
            view_stack = view_stack.push(self.context_menu_view(target));
        }

        // Diálogo de erro de rádio (com opção de quarentena)
        if let Some((ref station, ref msg)) = self.radio_error_dialog {
            view_stack = view_stack.push(self.radio_error_dialog_view(station, msg));
        }

        view_stack.into()
    }

    fn radio_error_dialog_view<'a>(
        &self,
        station: &crate::radio::RadioStation,
        msg: &str,
    ) -> Element<'a, Message> {
        use iced::widget::button;

        let title = format!("Não foi possível tocar \"{}\"", station.name);
        let content = column![
            row![
                text(crate::ui::icons::ICON_BROADCAST)
                    .font(crate::ui::icons::NERD_FONT_MONO)
                    .size(28)
                    .color(theme::red()),
                text(title).size(15).font(crate::ui::icons::UI_FONT_BOLD).color(theme::text()),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            Space::with_height(10),
            text(msg.to_string()).size(12).color(theme::subtext()),
            Space::with_height(18),
            row![
                button(text("Quarentenar estação").size(13).color(theme::base()))
                    .on_press(Message::QuarantineStation(station.clone()))
                    .style(theme::primary_button)
                    .padding([6, 14]),
                Space::with_width(Length::Fill),
                button(text("Fechar").size(13))
                    .on_press(Message::CloseRadioError)
                    .style(theme::secondary_button)
                    .padding([6, 14]),
            ]
            .align_y(Alignment::Center),
            Space::with_height(8),
            text("A quarentena remove a estação das listas permanentemente.")
                .size(11)
                .color(theme::overlay0()),
        ]
        .spacing(0)
        .padding(20);

        let card = container(content)
            .width(420)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::mantle())),
                border: iced::Border { color: theme::red(), width: 1.0, radius: 8.0.into() },
                shadow: iced::Shadow {
                    color: theme::base(),
                    offset: iced::Vector { x: 0.0, y: 4.0 },
                    blur_radius: 16.0,
                },
                ..Default::default()
            });

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55))),
                ..Default::default()
            })
            .into()
    }

    fn header_view(&self) -> Element<'_, Message> {
        container(
            row![
                text(crate::ui::icons::ICON_MUSIC)
                    .font(crate::ui::icons::NERD_FONT_MONO)
                    .color(theme::accent())
                    .size(32),
                Space::with_width(6),
                text("lavanda")
                    .color(theme::accent())
                    .size(16)
                    .font(crate::ui::icons::UI_FONT_BOLD),
                Space::with_width(Length::Fill),
            ]
            .align_y(Alignment::Center),
        )
        .style(theme::header)
        .width(Length::Fill)
        .padding([0, 16])
        .into()
    }

    fn shortcuts_modal_view(&self) -> Element<'_, Message> {
        use iced::widget::button;

        let row_item = |keys: &'static str, desc: &'static str| -> Element<'_, Message> {
            row![
                text(keys).width(Length::Fixed(120.0)).font(crate::ui::icons::UI_FONT_BOLD)
                    .color(theme::accent()).size(13),
                text(desc).color(theme::text()).size(13),
            ]
            .spacing(12).align_y(Alignment::Center).into()
        };

        let content = column![
            row![
                text("Keyboard Shortcuts").size(18).font(crate::ui::icons::UI_FONT_BOLD).color(theme::accent()),
                Space::with_width(Length::Fill),
                button(text(crate::ui::icons::ICON_CLOSE).font(crate::ui::icons::NERD_FONT_MONO).color(theme::red()).size(32))
                    .on_press(Message::CloseShortcuts).style(iced::widget::button::text),
            ].align_y(Alignment::Center),
            Space::with_height(16),
            row_item("Space", "Play / Pause"),
            row_item("N", "Next track"),
            row_item("P", "Previous track"),
            row_item("L / F", "Like / Unlike song"),
            row_item("E / M", "Edit metadata tags"),
            row_item("C", "Create playlist"),
            row_item("A", "Add song to playlist"),
            row_item("T", "Toggle focus mode"),
            row_item("V", "Toggle spectrum visualizer"),
            row_item("←/→", "Seek backward / forward"),
            row_item("↑/↓", "Navigate track list"),
            row_item("F5", "Rescan library"),
            row_item("+ / -", "Volume up / down"),
            row_item("?", "This help"),
        ]
        .spacing(8).padding(24);

        let dialog = container(content)
            .width(440)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::base())),
                border: iced::Border { color: theme::accent(), width: 1.0, radius: 8.0.into() },
                shadow: iced::Shadow {
                    color: theme::mantle(),
                    offset: iced::Vector { x: 0.0, y: 4.0 },
                    blur_radius: 12.0,
                },
                ..Default::default()
            });

        container(dialog)
            .width(Length::Fill).height(Length::Fill)
            .center_x(Length::Fill).center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                ..Default::default()
            })
            .into()
    }

    fn context_menu_view(&self, target: &ContextMenuTarget) -> Element<'_, Message> {
        use iced::widget::button;

        let custom_playlists = crate::persist::get(|db| db.playlists.keys().cloned().collect::<Vec<_>>());

        let item_style = |_: &iced::Theme, status: iced::widget::button::Status| {
            let hovered = matches!(status, iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed);
            iced::widget::button::Style {
                background: if hovered { Some(iced::Background::Color(theme::with_alpha(theme::accent(), 0.2))) } else { None },
                text_color: if hovered { theme::accent() } else { theme::text() },
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }
        };

        let accent_item_style = |_: &iced::Theme, status: iced::widget::button::Status| {
            let hovered = matches!(status, iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed);
            iced::widget::button::Style {
                background: if hovered { Some(iced::Background::Color(theme::with_alpha(theme::accent(), 0.2))) } else { None },
                text_color: theme::accent(),
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }
        };

        let mut playlist_section = column![
            text("Add to Playlist:").size(11).color(theme::subtext()).font(crate::ui::icons::UI_FONT_BOLD)
        ].spacing(4);

        let (title, extra_btns, create_btn) = match target {
            ContextMenuTarget::Artist(name) => {
                let hide_btn: Element<Message> = button(text("Hide from UI").size(13))
                    .on_press(Message::HideAlbumOrArtist(name.clone(), true))
                    .style(item_style).padding([4, 8]).width(Length::Fill).into();
                for pl in &custom_playlists {
                    let artist_tracks: Vec<_> = self.all_tracks.iter()
                        .filter(|t| t.artist == *name).cloned().collect();
                    playlist_section = playlist_section.push(
                        button(text(format!("  + {pl}")).size(12))
                            .on_press(Message::AddTracksToPlaylist(pl.clone(), artist_tracks))
                            .style(item_style).padding([4, 8]).width(Length::Fill)
                    );
                }
                let create: Element<Message> = button(text("+ Create playlist").size(12))
                    .on_press(Message::CreatePlaylistFromContext(name.clone(), true))
                    .style(accent_item_style).padding([4, 8]).width(Length::Fill).into();
                (format!("Artist: {name}"), Some(hide_btn), create)
            }
            ContextMenuTarget::Album(name) => {
                let hide_btn: Element<Message> = button(text("Hide from UI").size(13))
                    .on_press(Message::HideAlbumOrArtist(name.clone(), false))
                    .style(item_style).padding([4, 8]).width(Length::Fill).into();
                for pl in &custom_playlists {
                    let album_tracks: Vec<_> = self.all_tracks.iter()
                        .filter(|t| t.album == *name).cloned().collect();
                    playlist_section = playlist_section.push(
                        button(text(format!("  + {pl}")).size(12))
                            .on_press(Message::AddTracksToPlaylist(pl.clone(), album_tracks))
                            .style(item_style).padding([4, 8]).width(Length::Fill)
                    );
                }
                let create: Element<Message> = button(text("+ Create playlist").size(12))
                    .on_press(Message::CreatePlaylistFromContext(name.clone(), false))
                    .style(accent_item_style).padding([4, 8]).width(Length::Fill).into();
                (format!("Album: {name}"), Some(hide_btn), create)
            }
            ContextMenuTarget::Track(track) => {
                let like_label = if track.liked { "Unlike this song" } else { "Like this song" };
                let extra: Element<Message> = column![
                    button(text(like_label).size(12))
                        .on_press(Message::ToggleLikeTrack(track.clone()))
                        .style(item_style).padding([4, 8]).width(Length::Fill),
                    button(text("Edit ID3 tag").size(12))
                        .on_press(Message::OpenTagEditor(vec![track.clone()]))
                        .style(item_style).padding([4, 8]).width(Length::Fill),
                    button(text("Open file folder").size(12))
                        .on_press(Message::OpenLocalFolder(track.path.clone()))
                        .style(item_style).padding([4, 8]).width(Length::Fill),
                ].spacing(4).into();
                for pl in &custom_playlists {
                    playlist_section = playlist_section.push(
                        button(text(format!("  + {pl}")).size(12))
                            .on_press(Message::AddTracksToPlaylist(pl.clone(), vec![track.clone()]))
                            .style(item_style).padding([4, 8]).width(Length::Fill)
                    );
                }
                let create: Element<Message> = button(text("+ Create playlist with song").size(12))
                    .on_press(Message::CreatePlaylistWithTracks(track.title.clone(), vec![track.clone()]))
                    .style(accent_item_style).padding([4, 8]).width(Length::Fill).into();
                (format!("Song: {}", track.title), Some(extra), create)
            }
            ContextMenuTarget::MultipleTracks(tracks) => {
                let extra: Element<Message> = button(text("Edit ID3 tags").size(12))
                    .on_press(Message::OpenTagEditor(tracks.clone()))
                    .style(item_style).padding([4, 8]).width(Length::Fill).into();
                for pl in &custom_playlists {
                    playlist_section = playlist_section.push(
                        button(text(format!("  + {pl}")).size(12))
                            .on_press(Message::AddTracksToPlaylist(pl.clone(), tracks.clone()))
                            .style(item_style).padding([4, 8]).width(Length::Fill)
                    );
                }
                let create: Element<Message> = button(text("+ Create playlist with selection").size(12))
                    .on_press(Message::CreatePlaylistWithTracks("Selected Tracks".to_string(), tracks.clone()))
                    .style(accent_item_style).padding([4, 8]).width(Length::Fill).into();
                (format!("{} Songs", tracks.len()), Some(extra), create)
            }
            ContextMenuTarget::Header(col) => {
                let active_cols = crate::persist::get(|db| db.table_columns.clone());
                let mut cols_col = column![
                    text("Show / Hide:").size(11).color(theme::subtext()).font(crate::ui::icons::UI_FONT_BOLD),
                    Space::with_height(4),
                ].spacing(4);
                for &c in crate::persist::TableColumn::all() {
                    let visible = active_cols.contains(&c);
                    cols_col = cols_col.push(
                        button(row![
                            text(if visible { "" } else { "" })
                                .font(crate::ui::icons::NERD_FONT_MONO)
                                .color(if visible { theme::accent() } else { theme::overlay0() })
                                .size(28),
                            text(c.label()).size(13).color(theme::text()),
                        ].spacing(8))
                        .on_press(Message::ToggleColumnVisibility(c))
                        .style(item_style).padding([4, 8]).width(Length::Fill)
                    );
                }
                let header_extra: Element<Message> = column![
                    text(format!("Column: {}", col.label())).size(11).color(theme::subtext()).font(crate::ui::icons::UI_FONT_BOLD),
                    Space::with_height(4),
                    button(text("<- Move Left").size(12)).on_press(Message::MoveColumnLeft(*col)).style(item_style).padding([4, 8]).width(Length::Fill),
                    button(text("-> Move Right").size(12)).on_press(Message::MoveColumnRight(*col)).style(item_style).padding([4, 8]).width(Length::Fill),
                    Space::with_height(8),
                    cols_col,
                ].spacing(4).into();
                let dummy: Element<Message> = Space::with_height(0).into();
                ("Table Columns".to_string(), Some(header_extra), dummy)
            }
        };

        playlist_section = playlist_section.push(Space::with_height(4)).push(create_btn);

        let mut menu_col = column![
            row![
                text(title).size(13).font(crate::ui::icons::UI_FONT_BOLD).color(theme::accent()),
                Space::with_width(Length::Fill),
                button(text(crate::ui::icons::ICON_CLOSE).font(crate::ui::icons::NERD_FONT_MONO).color(theme::red()).size(26))
                    .on_press(Message::ToggleContextMenu(None)).style(iced::widget::button::text),
            ].align_y(Alignment::Center),
            Space::with_height(8),
        ];

        if let Some(extra) = extra_btns {
            menu_col = menu_col.push(extra).push(Space::with_height(6));
        }

        let menu_card = container(
            menu_col.push(playlist_section).spacing(4).padding(16)
        )
        .width(260)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::mantle())),
            border: iced::Border { color: theme::accent(), width: 1.0, radius: 8.0.into() },
            shadow: iced::Shadow {
                color: theme::base(),
                offset: iced::Vector { x: 0.0, y: 4.0 },
                blur_radius: 8.0,
            },
            ..Default::default()
        });

        container(menu_card)
            .width(Length::Fill).height(Length::Fill)
            .center_x(Length::Fill).center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                ..Default::default()
            })
            .into()
    }

    // ── Subscription ──────────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Message> {
        let base = Subscription::batch([
            Subscription::run_with_id("audio-events", channel_stream(self.audio_events.clone(), Message::Audio)),
            Subscription::run_with_id("mpris-cmds", channel_stream(self.mpris_cmds.clone(), Message::Mpris)),
            Subscription::run_with_id("spectrum", channel_stream(self.spectrum_rx.clone(), Message::SpectrumData)),
            iced::time::every(Duration::from_secs(3)).map(|_| Message::CheckTheme),
            iced::time::every(Duration::from_secs(15)).map(|_| Message::CheckNetwork),
            iced::keyboard::on_key_press(|key, _mods| Some(Message::KeyPressed(key))),
            iced::event::listen_with(|event, _, _| match event {
                iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(mods)) => {
                    Some(Message::ModifiersChanged(mods))
                }
                iced::Event::Window(iced::window::Event::Resized(size)) => {
                    Some(Message::WindowResized(size.width as f32, size.height as f32))
                }
                _ => None,
            }),
        ]);

        let mut subs = vec![base];

        if self.dragging_sidebar {
            subs.push(iced::event::listen_with(|event, _, _| {
                use iced::mouse;
                match event {
                    iced::Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Message::SidebarDragMove(position.x)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(Message::SidebarDragEnd),
                    _ => None,
                }
            }));
        }

        if self.dragging_playlist_split {
            subs.push(iced::event::listen_with(|event, _, _| {
                use iced::mouse;
                match event {
                    iced::Event::Mouse(mouse::Event::CursorMoved { position }) => Some(Message::PlaylistDragMove(position.y)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => Some(Message::PlaylistDragEnd),
                    _ => None,
                }
            }));
        }

        Subscription::batch(subs)
    }

    // ── Helpers de reprodução ─────────────────────────────────────────────────

    fn play_track_internal(&mut self, track: Track) -> Task<Message> {
        let cover_data = load_cover(&track.path);
        let track = Track { cover_data: cover_data.clone(), ..track };
        self.audio.send(AudioCommand::Play(track.path.clone()));
        self.audio.send(AudioCommand::SetVolume(self.volume));
        self.current_station = None;
        self.stream_title = None;
        self.current_track = Some(track.clone());
        self.selected_track = Some(track.clone());
        self.playback_state = PlaybackState::Playing;
        self.position = Duration::ZERO;
        self.duration = Duration::ZERO;
        self.notify_mpris_track(PlaybackStatus::Playing);
        self.persist_state();

        crate::persist::add_to_recently_played(track.path.clone());

        // Contabiliza a reprodução ao iniciar a faixa.
        let count = crate::persist::increment_play_count(track.path.clone());
        let path = track.path.clone();
        for t in self.all_tracks.iter_mut().filter(|t| t.path == path) { t.play_count = count; }
        for t in self.tracks.iter_mut().filter(|t| t.path == path) { t.play_count = count; }
        if let Some(ref mut ct) = self.current_track { ct.play_count = count; }

        if self.selected_playlist.as_deref() == Some("Recently Played") {
            self.update_filtered_tracks();
        }

        // Cache cover para MPRIS art_url
        if let Some(data) = cover_data {
            let cp = cache_cover_path();
            if let Some(dir) = cp.parent() { std::fs::create_dir_all(dir).ok(); }
            if std::fs::write(&cp, &data).is_ok() {
                let art_url = format!("file://{}", cp.display());
                let t = track.clone();
                self.send_mpris(MprisUpdate::Metadata {
                    title: t.title, artist: t.artist, album: t.album,
                    duration_us: t.duration.as_micros() as i64, art_url: Some(art_url),
                });
            }
        }

        send_track_notification(&track.title, &track.artist);

        if let Some(y) = self.calculate_scroll_offset(track.id) {
            iced::widget::scrollable::scroll_to(
                iced::widget::scrollable::Id::new("tracklist_scroll"),
                iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: (y - 120.0).max(0.0) },
            )
        } else {
            Task::none()
        }
    }

    fn play_station_internal(&mut self, station: crate::radio::RadioStation) -> Task<Message> {
        let url = station.stream_url().to_string();
        if url.is_empty() {
            self.radio_error = Some("Estação sem URL de stream".into());
            return Task::none();
        }
        self.audio.send(AudioCommand::PlayStream {
            url,
            codec: station.codec.clone(),
        });
        self.audio.send(AudioCommand::SetVolume(self.volume));

        self.current_track = None;
        self.stream_title = None;
        self.current_station = Some(station.clone());
        self.playback_state = PlaybackState::Playing;
        self.position = Duration::ZERO;
        self.duration = Duration::ZERO;

        // Metadata inicial no MPRIS (o título "now playing" chega depois via ICY).
        self.send_mpris(MprisUpdate::Metadata {
            title: station.name.clone(),
            artist: station.name.clone(),
            album: "Radio".to_string(),
            duration_us: 0,
            art_url: None,
        });
        self.send_mpris(MprisUpdate::Status(PlaybackStatus::Playing));

        // Registra o play no diretório (educado, em background).
        let uuid = station.stationuuid.clone();
        std::thread::spawn(move || crate::radio::register_click(&uuid));

        send_track_notification(&station.name, "Radio");
        Task::none()
    }

    fn advance_track(&mut self, delta: i32) -> Task<Message> {
        if self.queue.is_empty() { return Task::none(); }
        let next_idx = if self.shuffle {
            use rand::Rng;
            let cur = self.current_track.as_ref().and_then(|ct| self.queue.iter().position(|t| t.id == ct.id));
            let len = self.queue.len();
            if len == 1 { 0 } else {
                let mut rng = rand::thread_rng();
                let mut idx = rng.gen_range(0..len);
                if let Some(c) = cur { while idx == c { idx = rng.gen_range(0..len); } }
                idx
            }
        } else {
            let cur = self.current_track.as_ref().and_then(|ct| self.queue.iter().position(|t| t.id == ct.id));
            match cur {
                Some(i) => { let n = i as i32 + delta; if n < 0 { self.queue.len() - 1 } else { n as usize % self.queue.len() } }
                None => 0,
            }
        };
        if let Some(track) = self.queue.get(next_idx).cloned() {
            self.play_track_internal(track)
        } else {
            Task::none()
        }
    }
}

// ── Helpers globais ───────────────────────────────────────────────────────────

fn channel_stream<T>(holder: Shared<T>, map: fn(T) -> Message) -> impl Stream<Item = Message>
where
    T: Send + 'static,
{
    iced::stream::channel(64, move |mut output| async move {
        let Some(mut rx) = holder.lock().unwrap().take() else { return; };
        while let Some(item) = rx.recv().await {
            if output.send(map(item)).await.is_err() { break; }
        }
    })
}

fn cache_cover_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cache/lavanda/cover.jpg")
}

fn send_track_notification(title: &str, artist: &str) {
    std::process::Command::new("notify-send")
        .args(["lavanda", &format!("{title} — {artist}"), "--icon=audio-x-generic", "--expire-time=3000", "--urgency=low"])
        .spawn().ok();
}

fn sidebar_width_path() -> PathBuf {
    let xdg = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!("{}/.config", std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
    });
    PathBuf::from(xdg).join("lavanda/sidebar_width")
}

fn load_sidebar_width() -> f32 {
    std::fs::read_to_string(sidebar_width_path()).ok()
        .and_then(|s| s.trim().parse().ok()).unwrap_or(200.0)
}

fn save_sidebar_width(width: f32) {
    let path = sidebar_width_path();
    if let Some(dir) = path.parent() { std::fs::create_dir_all(dir).ok(); }
    std::fs::write(path, width.to_string()).ok();
}

fn build_iced_theme() -> Theme {
    Theme::custom("Omarchy".into(), iced::theme::Palette {
        background: theme::base(), text: theme::text(),
        primary: theme::accent(), success: theme::green(), danger: theme::red(),
    })
}

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
