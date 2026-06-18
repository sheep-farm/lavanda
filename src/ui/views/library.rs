use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, ContextMenuTarget, Message, PlaylistDialogMode, PlaylistTab, SortColumn, ViewMode};
use crate::library::models::Track;
use crate::persist::TableColumn;
use crate::ui::{icons, theme};

// ── Jellyfin helpers (para evitar repetição de código de loading/error) ────────

fn loading_indicator<'a>() -> Element<'a, Message> {
    container(text("Loading…").size(14).color(theme::overlay0()))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn error_indicator<'a>(msg: &str) -> Element<'a, Message> {
    container(text(format!("Error: {msg}")).size(13).color(theme::red()))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn hint_indicator<'a>(msg: &'static str) -> Element<'a, Message> {
    container(text(msg).size(14).color(theme::overlay0()))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

const ROW_H: f32 = 34.0;
/// Largura da coluna de favorito (coração), compartilhada entre header e linhas.
const LIKE_COL_W: f32 = 60.0;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let sidebar = sidebar_view(state);

    let drag_handle = mouse_area(
        container(Space::new(Length::Fixed(8.0), Length::Fill)).style(|_| {
            iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::surface0())),
                ..Default::default()
            }
        }),
    )
    .interaction(iced::mouse::Interaction::ResizingHorizontally)
    .on_press(Message::SidebarDragStart);

    let main_panel = if state.view_mode == ViewMode::Radios {
        radio_panel_view(state)
    } else if state.view_mode == ViewMode::Jellyfin {
        jf_main_panel_view(state)
    } else if state.view_mode == ViewMode::Navidrome {
        nd_main_panel_view(state)
    } else {
        track_list_view(state)
    };

    row![sidebar, drag_handle, main_panel]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn sidebar_view(state: &AppState) -> Element<'_, Message> {
    let tab_btn = |label: &'static str, mode: ViewMode, icon: &'static str, enabled: bool| -> Element<'_, Message> {
        let active = state.view_mode == mode && state.selected_playlist.is_none();
        let color = if !enabled {
            theme::with_alpha(theme::overlay0(), 0.5)
        } else if active {
            theme::accent()
        } else {
            theme::subtext()
        };
        button(
            row![
                text(icon).font(icons::NERD_FONT_MONO).size(24).color(color),
                text(label).size(12).color(color),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .on_press_maybe(enabled.then_some(Message::SelectViewMode(mode)))
        .style(iced::widget::button::text)
        .padding([4, 6])
        .into()
    };

    let mut tabs = row![
        tab_btn("Artists", ViewMode::Artists, icons::ICON_MUSIC, true),
        tab_btn("Albums", ViewMode::Albums, icons::ICON_LIST, true),
        tab_btn("Genres", ViewMode::Genres, icons::ICON_PODIUM, true),
        // Radios fica desabilitada quando offline.
        tab_btn("Radios", ViewMode::Radios, icons::ICON_BROADCAST, state.online),
    ]
    .spacing(4);

    let cfg = crate::config::get();
    if !cfg.jellyfin_url.is_empty() {
        tabs = tabs.push(tab_btn("Jellyfin", ViewMode::Jellyfin, icons::ICON_CLOUD, true));
    }
    if !cfg.navidrome_url.is_empty() {
        tabs = tabs.push(tab_btn("Navidrome", ViewMode::Navidrome, icons::ICON_CLOUD, true));
    }

    let (search_value, search_msg): (&str, fn(String) -> Message) =
        if state.view_mode == ViewMode::Jellyfin {
            (&state.jf_sidebar_search, Message::JellyfinSidebarSearchChanged)
        } else if state.view_mode == ViewMode::Navidrome {
            (&state.nd_sidebar_search, Message::NavidromeSidebarSearchChanged)
        } else {
            (&state.sidebar_search, Message::SidebarSearchChanged)
        };

    let sidebar_search = row![
        text(icons::ICON_SEARCH)
            .font(icons::NERD_FONT_MONO)
            .size(22)
            .color(theme::overlay0()),
        text_input("Filter…", search_value)
            .on_input(search_msg)
            .style(theme::dialog_input)
            .size(12)
            .padding([3, 6])
            .width(Length::Fill),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 4]);

    // Build sidebar item list based on ViewMode
    let list_items: Element<Message> = match state.view_mode {
        ViewMode::Artists => {
            let items = state.artists();
            let selected = state.selected_artist.clone();
            let active = state.current_track.as_ref().map(|t| t.artist.clone());
            build_sidebar_list(items, selected, active, Message::SelectArtist, Some(ContextMenuTarget::Artist), false)
        }
        ViewMode::Albums => {
            let items = state.albums();
            let selected = state.selected_album.clone();
            let active = state.current_track.as_ref().map(|t| t.album.clone());
            build_sidebar_list(items, selected, active, Message::SelectAlbum, Some(ContextMenuTarget::Album), true)
        }
        ViewMode::Genres => {
            let items = state.genres();
            let selected = state.selected_genre.clone();
            build_sidebar_list(items, selected, None, Message::SelectGenre, None, false)
        }
        ViewMode::Radios => radio_favorites_list(state),
        ViewMode::Jellyfin => jf_sidebar_artists(state),
        ViewMode::Navidrome => nd_sidebar_artists(state),
    };

    let list_with_context: Element<Message> = mouse_area(
        scrollable(list_items)
            .id(scrollable::Id::new("sidebar_scroll"))
            .height(Length::Fill),
    )
    .on_enter(Message::HoverSidebarList(true))
    .on_exit(Message::HoverSidebarList(false))
    .into();

    let playlist_panel = playlist_panel_view(state);

    let pl_resize = mouse_area(
        container(
            container(Space::new(Length::Fill, Length::Fixed(8.0))).style(move |_| {
                iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme::surface0())),
                    ..Default::default()
                }
            }),
        )
        .padding([2, 0]),
    )
    .interaction(iced::mouse::Interaction::ResizingVertically)
    .on_press(Message::PlaylistDragStart);

    let sidebar_body = column![
        container(tabs).padding([4, 4]),
        sidebar_search,
        list_with_context,
        pl_resize,
        playlist_panel,
    ]
    .spacing(0);

    let sidebar_w = state.sidebar_width.max(crate::config::min_sidebar_width());
    container(sidebar_body)
        .style(theme::sidebar)
        .width(Length::Fixed(sidebar_w))
        .height(Length::Fill)
        .into()
}

fn build_sidebar_list(
    items: Vec<String>,
    selected: Option<String>,
    active: Option<String>,
    make_msg: fn(String) -> Message,
    make_ctx: Option<fn(String) -> ContextMenuTarget>,
    mark_fav: bool,
) -> Element<'static, Message> {
    if items.is_empty() {
        return container(text("Nothing here").color(theme::overlay0()).size(13))
            .center_x(Length::Fill)
            .padding([12, 0])
            .width(Length::Fill)
            .into();
    }

    // Carrega os álbuns favoritos uma vez (evita travar o DB por item).
    let favs = if mark_fav {
        crate::persist::get(|db| db.favorite_albums.clone())
    } else {
        std::collections::HashSet::new()
    };

    let rows: Vec<Element<'static, Message>> = items
        .into_iter()
        .map(|name| {
            let is_selected = selected.as_deref() == Some(name.as_str());
            let is_active = active.as_deref() == Some(name.as_str());
            let color = if is_selected {
                theme::accent()
            } else if is_active {
                theme::green()
            } else {
                theme::text()
            };

            let ctx = make_ctx.map(|f| f(name.clone()));
            let msg = make_msg(name.clone());
            let label: Element<'static, Message> = if favs.contains(&name) {
                row![
                    text(name.clone()).color(color).size(13).width(Length::Fill),
                    text(icons::ICON_HEART)
                        .font(icons::NERD_FONT_MONO)
                        .size(26)
                        .color(theme::red()),
                    Space::with_width(Length::Fixed(6.0)),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
            } else {
                text(name.clone()).color(color).size(13).width(Length::Fill).into()
            };
            let btn = button(label)
                .on_press(msg)
                .style(move |_, status| {
                    let bg = match status {
                        button::Status::Hovered if !is_selected => Some(
                            iced::Background::Color(theme::with_alpha(theme::overlay0(), 0.13)),
                        ),
                        _ => None,
                    };
                    button::Style { background: bg, ..Default::default() }
                })
                .width(Length::Fill)
                .padding([5, 10]);

            let styled: Element<'static, Message> = if is_selected {
                container(btn).style(theme::selected_row).width(Length::Fill).into()
            } else {
                container(btn).width(Length::Fill).into()
            };

            if let Some(target) = ctx {
                mouse_area(styled)
                    .on_right_press(Message::ToggleContextMenu(Some(target)))
                    .into()
            } else {
                styled
            }
        })
        .collect();

    column(rows).spacing(1).into()
}

// ── Playlist panel ────────────────────────────────────────────────────────────

fn playlist_panel_view(state: &AppState) -> Element<'_, Message> {
    let is_playlists = state.playlist_tab == PlaylistTab::Playlists;

    let tab_row: Element<Message> = row![
        button(
            text("Playlists").size(12).color(
                if is_playlists { theme::accent() } else { theme::subtext() },
            ),
        )
        .on_press(Message::SelectPlaylistTab(PlaylistTab::Playlists))
        .style(iced::widget::button::text)
        .padding([3, 6]),
        button(
            text("Auto").size(12).color(
                if !is_playlists { theme::accent() } else { theme::subtext() },
            ),
        )
        .on_press(Message::SelectPlaylistTab(PlaylistTab::Autoplaylists))
        .style(iced::widget::button::text)
        .padding([3, 6]),
        Space::with_width(Length::Fill),
        button(
            text(icons::ICON_PLUS)
                .font(icons::NERD_FONT_MONO)
                .size(26)
                .color(theme::accent()),
        )
        .on_press(Message::OpenPlaylistDialog(PlaylistDialogMode::Create))
        .style(iced::widget::button::text)
        .padding([3, 6]),
    ]
    .spacing(0)
    .align_y(Alignment::Center)
    .padding([0, 4])
    .into();

    let list: Element<Message> = if is_playlists {
        let playlists =
            crate::persist::get(|db| db.playlists.keys().cloned().collect::<Vec<_>>());

        if playlists.is_empty() {
            container(
                text("No playlists. Press C or + to create.")
                    .size(12)
                    .color(theme::overlay0()),
            )
            .center_x(Length::Fill)
            .padding([8, 8])
            .width(Length::Fill)
            .into()
        } else {
            let selected_pl = state.selected_playlist.clone();

            let rows: Vec<Element<Message>> = playlists
                .into_iter()
                .map(|pl| {
                    let is_sel = selected_pl.as_deref() == Some(pl.as_str());

                    let delete_btn: Element<Message> = if is_sel {
                        button(
                            text(icons::ICON_TRASH)
                                .font(icons::NERD_FONT_MONO)
                                .size(22)
                                .color(theme::red()),
                        )
                        .on_press(Message::DeletePlaylist(pl.clone()))
                        .style(iced::widget::button::text)
                        .padding([2, 4])
                        .into()
                    } else {
                        Space::with_width(20).into()
                    };

                    let msg = Message::SelectPlaylist(pl.clone());
                    let pl_label = pl.clone();
                    let pl_btn = button(
                        row![
                            text(pl_label)
                                .size(12)
                                .color(if is_sel { theme::accent() } else { theme::text() })
                                .width(Length::Fill),
                            delete_btn,
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .on_press(msg)
                    .style(iced::widget::button::text)
                    .width(Length::Fill)
                    .padding([4, 8]);

                    if is_sel {
                        container(pl_btn).style(theme::selected_row).width(Length::Fill).into()
                    } else {
                        container(pl_btn).width(Length::Fill).into()
                    }
                })
                .collect();

            column(rows).spacing(1).into()
        }
    } else {
        let selected_pl = state.selected_playlist.clone();
        let autoplaylists: Vec<(&'static str, &'static str, iced::Color)> = vec![
            ("Liked Songs", icons::ICON_HEART, theme::red()),
            ("Liked Albums", icons::ICON_LIST, theme::red()),
            ("Recently Played", icons::ICON_CLOCK, theme::accent()),
            ("Most Played", icons::ICON_PODIUM, theme::green()),
        ];
        let rows: Vec<Element<Message>> = autoplaylists
            .into_iter()
            .map(|(name, icon, color)| {
                let is_sel = selected_pl.as_deref() == Some(name);
                let pl_btn = button(
                    row![
                        text(icon).font(icons::NERD_FONT_MONO).size(22).color(color),
                        text(name)
                            .size(12)
                            .color(if is_sel { theme::accent() } else { theme::text() }),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::SelectPlaylist(name.to_string()))
                .style(iced::widget::button::text)
                .width(Length::Fill)
                .padding([4, 8]);

                if is_sel {
                    container(pl_btn).style(theme::selected_row).width(Length::Fill).into()
                } else {
                    container(pl_btn).width(Length::Fill).into()
                }
            })
            .collect();
        column(rows).spacing(1).into()
    };

    let scroll_h = (state.playlist_height - 30.0).max(0.0);

    container(
        column![tab_row, scrollable(list).height(Length::Fixed(scroll_h))]
            .spacing(0),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::with_alpha(theme::mantle(), 0.6))),
        border: iced::Border {
            color: theme::surface0(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .height(Length::Fixed(state.playlist_height))
    .width(Length::Fill)
    .into()
}

// ── Rádio ───────────────────────────────────────────────────────────────────

/// Lista de estações favoritas na sidebar (aba Radios).
fn radio_favorites_list(state: &AppState) -> Element<'static, Message> {
    let favorites = crate::persist::radio_favorites();
    if favorites.is_empty() {
        return container(
            text("No favorite stations.\nStar one from the list →")
                .size(12)
                .color(theme::overlay0()),
        )
        .padding([12, 10])
        .width(Length::Fill)
        .into();
    }

    let current = state.current_station.as_ref().map(|s| s.stationuuid.clone());
    let rows: Vec<Element<'static, Message>> = favorites
        .into_iter()
        .map(|st| {
            let is_current = current.as_deref() == Some(st.stationuuid.as_str());
            let color = if is_current { theme::green() } else { theme::text() };
            let label = st.name.clone();
            button(text(label).size(13).color(color).width(Length::Fill))
                .on_press(Message::PlayStation(st))
                .style(iced::widget::button::text)
                .width(Length::Fill)
                .padding([5, 10])
                .into()
        })
        .collect();

    column(rows).spacing(1).into()
}

/// Painel principal da aba Radios: busca + resultados.
fn radio_panel_view(state: &AppState) -> Element<'_, Message> {
    let search = row![
        text(icons::ICON_SEARCH)
            .font(icons::NERD_FONT_MONO)
            .size(24)
            .color(theme::overlay0()),
        text_input("Search stations…", &state.radio_search)
            .on_input(Message::RadioSearchChanged)
            .on_submit(Message::RadioSearchSubmit)
            .style(theme::dialog_input)
            .size(13)
            .padding([4, 8])
            .width(Length::Fill),
        button(text("Search").size(13))
            .on_press(Message::RadioSearchSubmit)
            .style(theme::primary_button)
            .padding([5, 14]),
        button(text("Top").size(13))
            .on_press(Message::RadioShowTop)
            .style(theme::secondary_button)
            .padding([5, 14]),
        button(text("SomaFM").size(13))
            .on_press(Message::RadioShowSomaFm)
            .style(theme::secondary_button)
            .padding([5, 14]),
        pick_list(
            state.radio_countries.as_slice(),
            state.radio_country.clone(),
            Message::RadioCountrySelected,
        )
        .placeholder("Country")
        .text_size(13)
        .padding([5, 10]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([6, 12]);

    // Entrada manual: cola uma URL (.pls/.m3u ou stream direto) e toca.
    let url_entry = row![
        text(icons::ICON_BROADCAST)
            .font(icons::NERD_FONT_MONO)
            .size(24)
            .color(theme::overlay0()),
        text_input("Paste a stream URL (.pls, .m3u or direct)…", &state.radio_url_input)
            .on_input(Message::RadioUrlChanged)
            .on_submit(Message::RadioPlayUrl)
            .style(theme::dialog_input)
            .size(13)
            .padding([4, 8])
            .width(Length::Fill),
        button(text("Play URL").size(13))
            .on_press(Message::RadioPlayUrl)
            .style(theme::secondary_button)
            .padding([5, 14]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 6.0,
        left: 12.0,
    });

    // Resultados têm prioridade: um erro de API/reprodução nunca apaga a lista.
    let body: Element<Message> = if state.radio_loading {
        container(text("Loading…").size(15).color(theme::overlay0()))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else if !state.radio_results.is_empty() {
        let current = state.current_station.as_ref().map(|s| s.stationuuid.clone());
        let rows: Vec<Element<Message>> = state
            .radio_results
            .iter()
            .map(|st| radio_row(st, current.as_deref()))
            .collect();
        scrollable(column(rows).spacing(1))
            .id(scrollable::Id::new("radio_scroll"))
            .height(Length::Fill)
            .into()
    } else if let Some(err) = &state.radio_error {
        container(text(format!("Error: {err}")).size(14).color(theme::red()))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(text("No stations. Try a search.").size(14).color(theme::overlay0()))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    column![search, url_entry, body]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn radio_row(st: &crate::radio::RadioStation, current_uuid: Option<&str>) -> Element<'static, Message> {
    let is_current = current_uuid == Some(st.stationuuid.as_str());
    let is_fav = crate::persist::is_radio_favorite(st);

    let name_color = if is_current { theme::accent() } else { theme::text() };
    let name = if is_current { format!("▶ {}", st.name) } else { st.name.clone() };

    let mut meta_parts: Vec<String> = Vec::new();
    if !st.codec.is_empty() { meta_parts.push(st.codec.clone()); }
    if st.bitrate > 0 { meta_parts.push(format!("{} kbps", st.bitrate)); }
    if !st.countrycode.is_empty() { meta_parts.push(st.countrycode.clone()); }
    if !st.tags.is_empty() {
        let tags: String = st.tags.split(',').take(3).collect::<Vec<_>>().join(", ");
        if !tags.is_empty() { meta_parts.push(tags); }
    }
    let meta = meta_parts.join("  ·  ");

    let star = button(
        text(icons::ICON_STAR)
            .font(icons::NERD_FONT_MONO)
            .size(18)
            .color(if is_fav { theme::accent() } else { theme::with_alpha(theme::overlay0(), 0.5) }),
    )
    .on_press(Message::ToggleFavoriteStation(st.clone()))
    .style(iced::widget::button::text)
    .padding([0, 6]);

    let info = column![
        text(name).size(14).color(name_color),
        text(meta).size(11).color(theme::subtext()),
    ]
    .spacing(2)
    .width(Length::Fill);

    let play = button(info)
        .on_press(Message::PlayStation(st.clone()))
        .style(iced::widget::button::text)
        .width(Length::Fill)
        .padding([6, 12]);

    let row_inner = row![play, star]
        .align_y(Alignment::Center)
        .padding([0, 6]);

    let styled: Element<'static, Message> = if is_current {
        container(row_inner)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::with_alpha(theme::accent(), 0.08))),
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
    } else {
        container(row_inner).width(Length::Fill).into()
    };
    styled
}

// ── Track list ────────────────────────────────────────────────────────────────

fn track_list_view(state: &AppState) -> Element<'_, Message> {
    if state.all_tracks.is_empty() {
        return container(text("Scanning library…").color(theme::overlay0()).size(15))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    if state.tracks.is_empty() {
        let hint = if state.selected_artist.is_none()
            && state.selected_album.is_none()
            && state.selected_genre.is_none()
            && state.selected_playlist.is_none()
            && state.search_query.is_empty()
        {
            "Select an artist, album or genre"
        } else if !state.search_query.is_empty() {
            "No tracks match your search"
        } else {
            "No tracks found"
        };
        return container(text(hint).color(theme::overlay0()).size(14))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    // Fetch column config once
    let visible_cols = crate::persist::get(|db| db.table_columns.clone());
    let col_widths = column_widths(&visible_cols);

    let sort_col = state.sort_column;
    let sort_asc = state.sort_ascending;
    let group = state.group_by_album;

    // Toolbar (busca) e cabeçalho de colunas ficam fixos no topo, fora do scroll.
    let toolbar = toolbar_view(state);
    let header = build_column_header(visible_cols.clone(), col_widths.clone(), sort_col, sort_asc);

    let mut rows: Vec<Element<Message>> = Vec::new();

    let current_id = state.current_track.as_ref().map(|t| t.id);
    let selected_ids: Vec<i64> = state.selected_tracks.iter().map(|t| t.id).collect();
    let multi_selected = state.selected_tracks.clone();

    if group {
        let mut groups: Vec<(String, Vec<Track>)> = Vec::new();
        for track in &state.tracks {
            if let Some(last) = groups.last_mut() {
                if last.0 == track.album {
                    last.1.push(track.clone());
                    continue;
                }
            }
            groups.push((track.album.clone(), vec![track.clone()]));
        }

        let fav_albums = crate::persist::get(|db| db.favorite_albums.clone());
        for (album_name, tracks) in groups {
            let count = tracks.len();
            let album_label = album_name.clone();
            let is_fav = fav_albums.contains(&album_name);
            // Coração: marcador de favorito e também botão para alternar.
            let fav_btn = button(
                text(icons::ICON_HEART)
                    .font(icons::NERD_FONT_MONO)
                    .size(26)
                    .color(if is_fav { theme::red() } else { theme::overlay0() }),
            )
            .on_press(Message::ToggleFavoriteAlbum(album_name.clone()))
            .style(iced::widget::button::text)
            .padding([0, 4]);
            let album_hdr: Element<Message> = container(
                row![
                    text(icons::ICON_LIST)
                        .font(icons::NERD_FONT_MONO)
                        .size(22)
                        .color(theme::accent()),
                    text(album_label)
                        .size(12)
                        .color(theme::accent())
                        .font(icons::UI_FONT_BOLD)
                        .width(Length::Fill),
                    fav_btn,
                    text(format!("{count} tracks")).size(11).color(theme::overlay0()),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding([5, 12]),
            )
            .style(theme::album_header)
            .width(Length::Fill)
            .into();
            rows.push(album_hdr);

            for track in tracks {
                rows.push(build_track_row(
                    track,
                    current_id,
                    &selected_ids,
                    &multi_selected,
                    visible_cols.clone(),
                    col_widths.clone(),
                ));
            }
            rows.push(Space::with_height(6).into());
        }
    } else {
        for track in state.tracks.iter() {
            rows.push(build_track_row(
                track.clone(),
                current_id,
                &selected_ids,
                &multi_selected,
                visible_cols.clone(),
                col_widths.clone(),
            ));
        }
    }

    let scroll = scrollable(column(rows).spacing(0))
        .id(scrollable::Id::new("tracklist_scroll"))
        .height(Length::Fill);

    let content = column![toolbar, header, scroll]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    mouse_area(container(content).width(Length::Fill).height(Length::Fill))
        .on_enter(Message::HoverTracklist(true))
        .on_exit(Message::HoverTracklist(false))
        .into()
}

fn build_column_header(
    cols: Vec<TableColumn>,
    widths: Vec<Length>,
    sort_col: Option<SortColumn>,
    sort_asc: bool,
) -> Element<'static, Message> {
    let header_cells: Vec<Element<'static, Message>> = cols
        .into_iter()
        .zip(widths.into_iter())
        .map(|(col, w)| -> Element<'static, Message> {
            let sc = table_col_to_sort(col);
            let sort_arrow = if sort_col == Some(sc) {
                if sort_asc { " ↑" } else { " ↓" }
            } else {
                ""
            };
            let label = format!("{}{}", col.label(), sort_arrow);
            let color = if sort_col == Some(sc) { theme::accent() } else { theme::subtext() };

            let hdr_btn = button(text(label).size(11).color(color).font(icons::UI_FONT_BOLD))
                .on_press(Message::SortBy(sc))
                .style(iced::widget::button::text)
                .padding([4, 0])
                .width(w);

            mouse_area(hdr_btn)
                .on_right_press(Message::ToggleContextMenu(Some(
                    ContextMenuTarget::Header(col),
                )))
                .into()
        })
        .collect();

    // Célula vazia final equivalente à coluna de favorito nas linhas de faixa,
    // para que as colunas FillPortion resolvam à mesma largura nos dois lugares.
    let header_row = row(header_cells)
        .push(Space::with_width(Length::Fixed(LIKE_COL_W)))
        .spacing(0)
        .align_y(Alignment::Center);

    container(header_row)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::mantle())),
            ..Default::default()
        })
        .padding([0, 12])
        .width(Length::Fill)
        .into()
}

fn toolbar_view(state: &AppState) -> Element<'_, Message> {
    use iced::widget::checkbox;

    let search_input: Element<Message> = row![
        text(icons::ICON_SEARCH)
            .font(icons::NERD_FONT_MONO)
            .size(24)
            .color(theme::overlay0()),
        text_input("Search…", &state.search_query)
            .on_input(Message::SearchChanged)
            .style(theme::dialog_input)
            .size(12)
            .padding([3, 6])
            .width(Length::Fixed(160.0)),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into();

    let group_color = if state.group_by_album { theme::accent() } else { theme::subtext() };
    let group_btn = button(
        row![
            text(icons::ICON_LIST).font(icons::NERD_FONT_MONO).size(22).color(group_color),
            text("Album").size(11).color(group_color),
        ]
        .spacing(3)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ToggleGroupByAlbum)
    .style(iced::widget::button::text)
    .padding([3, 6]);

    let restore_btn: Element<Message> = if !state.hidden_artists_albums.is_empty() {
        button(
            text(format!("Restore {} hidden", state.hidden_artists_albums.len()))
                .size(11)
                .color(theme::red()),
        )
        .on_press(Message::RestoreHiddenItems)
        .style(iced::widget::button::text)
        .padding([3, 6])
        .into()
    } else {
        Space::with_width(0).into()
    };

    container(
        row![
            search_input,
            Space::with_width(8),
            checkbox("Title", state.filter_title).on_toggle(|_| Message::ToggleFilterTitle).size(12).text_size(11),
            checkbox("Artist", state.filter_artist).on_toggle(|_| Message::ToggleFilterArtist).size(12).text_size(11),
            checkbox("Album", state.filter_album).on_toggle(|_| Message::ToggleFilterAlbum).size(12).text_size(11),
            checkbox("Genre", state.filter_genre).on_toggle(|_| Message::ToggleFilterGenre).size(12).text_size(11),
            Space::with_width(Length::Fill),
            group_btn,
            restore_btn,
            button(text("?").size(12).color(theme::subtext()))
                .on_press(Message::OpenShortcuts)
                .style(iced::widget::button::text)
                .padding([3, 6]),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([4, 12]),
    )
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::with_alpha(theme::mantle(), 0.7))),
        border: iced::Border {
            color: theme::surface0(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

fn build_track_row(
    track: Track,
    current_id: Option<i64>,
    selected_ids: &[i64],
    multi_selected: &[Track],
    cols: Vec<TableColumn>,
    widths: Vec<Length>,
) -> Element<'static, Message> {
    let is_current = current_id == Some(track.id);
    let is_selected = selected_ids.contains(&track.id);
    let liked = track.liked;
    let is_multi = selected_ids.len() > 1 && is_selected;

    let base_color = if is_current { theme::accent() } else { theme::text() };

    let cells: Vec<Element<'static, Message>> = cols
        .into_iter()
        .zip(widths.into_iter())
        .map(|(col, w)| -> Element<'static, Message> {
            // Indicador de reprodução como widget separado — sem misturar com o texto do título.
            if col == TableColumn::Title {
                let icon_color = if is_current { theme::accent() } else { iced::Color::TRANSPARENT };
                return row![
                    text(icons::ICON_PLAY)
                        .font(icons::NERD_FONT_MONO)
                        .size(11)
                        .color(icon_color),
                    text(track.title.clone())
                        .size(13)
                        .color(base_color)
                        .width(Length::Fill),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .width(w)
                .into();
            }

            let cell_text = match col {
                TableColumn::TrackNumber => track.track_number.map(|n| n.to_string()).unwrap_or_else(|| "·".to_string()),
                TableColumn::Title => unreachable!(),
                TableColumn::Artist => track.artist.clone(),
                TableColumn::Album => track.album.clone(),
                TableColumn::Genre => track.genre.clone(),
                TableColumn::Year => track.year.map(|y| y.to_string()).unwrap_or_default(),
                TableColumn::DiscNumber => track.disc_number.map(|d| d.to_string()).unwrap_or_default(),
                TableColumn::Duration => track.duration_str(),
                TableColumn::Plays => track.play_count.to_string(),
                TableColumn::DatePlayed => track.date_played.clone().unwrap_or_default(),
            };
            let cell_color = match col {
                TableColumn::Title => unreachable!(),
                TableColumn::Artist | TableColumn::Album | TableColumn::Genre if is_current => {
                    theme::with_alpha(theme::accent(), 0.8)
                }
                _ => theme::subtext(),
            };
            text(cell_text).size(13).color(cell_color).width(w).into()
        })
        .collect();

    // Coração sempre presente para evitar layout shift — apenas a cor varia.
    let heart_color = if liked {
        theme::red()
    } else {
        theme::with_alpha(theme::overlay0(), 0.22)
    };
    let like_msg = Message::ToggleLikeTrack(Track { cover_data: None, ..track.clone() });
    let like_icon: Element<'static, Message> = button(
        text(icons::ICON_HEART)
            .font(icons::NERD_FONT_MONO)
            .size(22)
            .color(heart_color),
    )
    .on_press(like_msg)
    .style(iced::widget::button::text)
    .width(Length::Fixed(LIKE_COL_W))
    .padding([0, 4])
    .into();

    let track_row_inner = row(cells)
        .push(like_icon)
        .spacing(0)
        .align_y(Alignment::Center)
        .padding([0, 12]);

    let row_styled: Element<'static, Message> = if is_selected {
        container(track_row_inner)
            .style(theme::selected_row)
            .height(Length::Fixed(ROW_H))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    } else if is_current {
        container(track_row_inner)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::with_alpha(theme::accent(), 0.08))),
                ..Default::default()
            })
            .height(Length::Fixed(ROW_H))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    } else {
        container(track_row_inner)
            .height(Length::Fixed(ROW_H))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    };

    let select_msg = Message::SelectTrack(track.clone());
    let ctx_msg = if is_multi {
        Message::ToggleContextMenu(Some(ContextMenuTarget::MultipleTracks(multi_selected.to_vec())))
    } else {
        Message::ToggleContextMenu(Some(ContextMenuTarget::Track(track)))
    };

    let row_btn = button(row_styled)
        .on_press(select_msg)
        .style(move |_, status| {
            let bg = match status {
                button::Status::Hovered if !is_selected && !is_current => Some(
                    iced::Background::Color(theme::with_alpha(theme::overlay0(), 0.13)),
                ),
                _ => None,
            };
            button::Style { background: bg, ..Default::default() }
        })
        .width(Length::Fill)
        .padding(0);

    mouse_area(row_btn).on_right_press(ctx_msg).into()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn column_widths(cols: &[TableColumn]) -> Vec<Length> {
    cols.iter()
        .map(|col| match col {
            TableColumn::TrackNumber => Length::Fixed(32.0),
            TableColumn::Title => Length::FillPortion(4),
            TableColumn::Artist => Length::FillPortion(3),
            TableColumn::Album => Length::FillPortion(3),
            TableColumn::Genre => Length::FillPortion(2),
            TableColumn::Year => Length::Fixed(48.0),
            TableColumn::DiscNumber => Length::Fixed(44.0),
            TableColumn::Duration => Length::Fixed(78.0),
            TableColumn::Plays => Length::Fixed(56.0),
            TableColumn::DatePlayed => Length::Fixed(110.0),
        })
        .collect()
}

fn table_col_to_sort(col: TableColumn) -> SortColumn {
    match col {
        TableColumn::TrackNumber => SortColumn::TrackNumber,
        TableColumn::Title => SortColumn::Title,
        TableColumn::Artist => SortColumn::Artist,
        TableColumn::Album => SortColumn::Album,
        TableColumn::Genre => SortColumn::Genre,
        TableColumn::Year => SortColumn::Year,
        TableColumn::DiscNumber => SortColumn::DiscNumber,
        TableColumn::Duration => SortColumn::Duration,
        TableColumn::Plays => SortColumn::Plays,
        TableColumn::DatePlayed => SortColumn::DatePlayed,
    }
}

// ── Jellyfin ──────────────────────────────────────────────────────────────────

/// Lista de artistas na sidebar (aba Jellyfin).
fn jf_sidebar_artists(state: &AppState) -> Element<'static, Message> {
    if state.jf_loading {
        return container(text("Loading…").size(13).color(theme::overlay0()))
            .padding([12, 10])
            .width(Length::Fill)
            .into();
    }
    if let Some(ref err) = state.jf_error {
        return container(text(err.clone()).size(12).color(theme::red()))
            .padding([8, 10])
            .width(Length::Fill)
            .into();
    }
    if state.jf_artists.is_empty() {
        return container(
            text("No artists found.\nCheck jellyfin_url in config.toml.")
                .size(12)
                .color(theme::overlay0()),
        )
        .padding([12, 10])
        .width(Length::Fill)
        .into();
    }

    let query = state.jf_sidebar_search.to_lowercase();
    let selected_id = state.jf_selected_artist.as_ref().map(|a| a.id.clone());

    let rows: Vec<Element<'static, Message>> = state
        .jf_artists
        .iter()
        .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
        .map(|artist| {
            let is_selected = selected_id.as_deref() == Some(artist.id.as_str());
            let color = if is_selected { theme::accent() } else { theme::text() };
            let artist = artist.clone();
            let btn = button(
                text(artist.name.clone())
                    .size(13)
                    .color(color)
                    .width(Length::Fill),
            )
            .on_press(Message::JellyfinSelectArtist(artist))
            .style(iced::widget::button::text)
            .width(Length::Fill)
            .padding([5, 10]);

            if is_selected {
                container(btn)
                    .style(theme::selected_row)
                    .width(Length::Fill)
                    .into()
            } else {
                container(btn).width(Length::Fill).into()
            }
        })
        .collect();

    column(rows).spacing(1).into()
}

/// Painel principal da aba Jellyfin.
fn jf_main_panel_view(state: &AppState) -> Element<'_, Message> {
    if let Some(ref err) = state.jf_error {
        return error_indicator(err);
    }

    if state.jf_selected_album.is_some() {
        return jf_track_list_view(state);
    }

    if state.jf_selected_artist.is_some() {
        return jf_albums_panel_view(state);
    }

    hint_indicator("Select an artist")
}

/// Lista de álbuns do artista selecionado.
fn jf_albums_panel_view(state: &AppState) -> Element<'_, Message> {
    if state.jf_loading {
        return loading_indicator();
    }
    if state.jf_albums.is_empty() {
        return hint_indicator("No albums found");
    }

    let selected_id = state.jf_selected_album.as_ref().map(|a| a.id.clone());

    let rows: Vec<Element<Message>> = state
        .jf_albums
        .iter()
        .map(|album| {
            let is_sel = selected_id.as_deref() == Some(album.id.as_str());
            let color = if is_sel { theme::accent() } else { theme::text() };
            let album = album.clone();
            let btn = button(
                text(album.name.clone())
                    .size(13)
                    .color(color)
                    .width(Length::Fill),
            )
            .on_press(Message::JellyfinSelectAlbum(album))
            .style(iced::widget::button::text)
            .width(Length::Fill)
            .padding([6, 16]);

            if is_sel {
                container(btn)
                    .style(theme::selected_row)
                    .width(Length::Fill)
                    .into()
            } else {
                container(btn).width(Length::Fill).into()
            }
        })
        .collect();

    scrollable(column(rows).spacing(1))
        .height(Length::Fill)
        .into()
}

/// Tracklist do álbum Jellyfin selecionado.
fn jf_track_list_view(state: &AppState) -> Element<'_, Message> {
    if state.jf_loading { return loading_indicator(); }
    if state.jf_tracks.is_empty() { return hint_indicator("No tracks found"); }
    remote_track_list_view(state, &state.jf_tracks.clone(), "jf_tracklist_scroll")
}

/// Tracklist compartilhada para fontes remotas (Jellyfin e Navidrome).
fn remote_track_list_view<'a>(
    state: &'a AppState,
    tracks: &[Track],
    scroll_id: &'static str,
) -> Element<'a, Message> {
    let visible_cols = crate::persist::get(|db| db.table_columns.clone());
    let col_widths = column_widths(&visible_cols);
    let header = build_column_header(visible_cols.clone(), col_widths.clone(), None, true);

    let current_id = state.current_track.as_ref().map(|t| t.id);
    let selected_ids: Vec<i64> = state.selected_tracks.iter().map(|t| t.id).collect();
    let multi_selected = state.selected_tracks.clone();

    let rows: Vec<Element<Message>> = tracks
        .iter()
        .map(|track| {
            build_track_row(
                track.clone(),
                current_id,
                &selected_ids,
                &multi_selected,
                visible_cols.clone(),
                col_widths.clone(),
            )
        })
        .collect();

    let scroll = scrollable(column(rows).spacing(0))
        .id(scrollable::Id::new(scroll_id))
        .height(Length::Fill);

    column![header, scroll]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Navidrome ─────────────────────────────────────────────────────────────────

fn nd_sidebar_artists(state: &AppState) -> Element<'static, Message> {
    if state.nd_loading {
        return container(text("Loading…").size(13).color(theme::overlay0()))
            .padding([12, 10])
            .width(Length::Fill)
            .into();
    }
    if let Some(ref err) = state.nd_error {
        return container(text(err.clone()).size(12).color(theme::red()))
            .padding([8, 10])
            .width(Length::Fill)
            .into();
    }
    if state.nd_artists.is_empty() {
        return container(
            text("No artists found.\nCheck navidrome_url in config.toml.")
                .size(12)
                .color(theme::overlay0()),
        )
        .padding([12, 10])
        .width(Length::Fill)
        .into();
    }

    let query = state.nd_sidebar_search.to_lowercase();
    let selected_id = state.nd_selected_artist.as_ref().map(|a| a.id.clone());

    let rows: Vec<Element<'static, Message>> = state
        .nd_artists
        .iter()
        .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
        .map(|artist| {
            let is_selected = selected_id.as_deref() == Some(artist.id.as_str());
            let color = if is_selected { theme::accent() } else { theme::text() };
            let artist = artist.clone();
            let btn = button(
                text(artist.name.clone())
                    .size(13)
                    .color(color)
                    .width(Length::Fill),
            )
            .on_press(Message::NavidromeSelectArtist(artist))
            .style(iced::widget::button::text)
            .width(Length::Fill)
            .padding([5, 10]);

            if is_selected {
                container(btn).style(theme::selected_row).width(Length::Fill).into()
            } else {
                container(btn).width(Length::Fill).into()
            }
        })
        .collect();

    column(rows).spacing(1).into()
}

fn nd_main_panel_view(state: &AppState) -> Element<'_, Message> {
    if let Some(ref err) = state.nd_error {
        return error_indicator(err);
    }
    if state.nd_selected_album.is_some() {
        if state.nd_loading { return loading_indicator(); }
        if state.nd_tracks.is_empty() { return hint_indicator("No tracks found"); }
        return remote_track_list_view(state, &state.nd_tracks.clone(), "nd_tracklist_scroll");
    }
    if state.nd_selected_artist.is_some() {
        return nd_albums_panel_view(state);
    }
    hint_indicator("Select an artist")
}

fn nd_albums_panel_view(state: &AppState) -> Element<'_, Message> {
    if state.nd_loading { return loading_indicator(); }
    if state.nd_albums.is_empty() { return hint_indicator("No albums found"); }

    let selected_id = state.nd_selected_album.as_ref().map(|a| a.id.clone());

    let rows: Vec<Element<Message>> = state
        .nd_albums
        .iter()
        .map(|album| {
            let is_sel = selected_id.as_deref() == Some(album.id.as_str());
            let color = if is_sel { theme::accent() } else { theme::text() };
            let album = album.clone();
            let btn = button(
                text(album.name.clone())
                    .size(13)
                    .color(color)
                    .width(Length::Fill),
            )
            .on_press(Message::NavidromeSelectAlbum(album))
            .style(iced::widget::button::text)
            .width(Length::Fill)
            .padding([6, 16]);

            if is_sel {
                container(btn).style(theme::selected_row).width(Length::Fill).into()
            } else {
                container(btn).width(Length::Fill).into()
            }
        })
        .collect();

    scrollable(column(rows).spacing(1))
        .height(Length::Fill)
        .into()
}
