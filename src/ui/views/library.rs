use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, ContextMenuTarget, Message, PlaylistDialogMode, PlaylistTab, SortColumn, ViewMode};
use crate::library::models::Track;
use crate::persist::TableColumn;
use crate::ui::{icons, theme};

const ROW_H: f32 = 34.0;

pub fn view(state: &AppState) -> Element<'_, Message> {
    let sidebar = sidebar_view(state);

    let drag_handle = mouse_area(
        container(Space::new(Length::Fixed(4.0), Length::Fill)).style(|_| {
            iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::surface0())),
                ..Default::default()
            }
        }),
    )
    .on_press(Message::SidebarDragStart);

    let track_list = track_list_view(state);

    row![sidebar, drag_handle, track_list]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn sidebar_view(state: &AppState) -> Element<'_, Message> {
    let tab_btn = |label: &'static str, mode: ViewMode, icon: &'static str| -> Element<'_, Message> {
        let active = state.view_mode == mode && state.selected_playlist.is_none();
        let color = if active { theme::accent() } else { theme::subtext() };
        button(
            row![
                text(icon).font(icons::NERD_FONT_MONO).size(12).color(color),
                text(label).size(12).color(color),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SelectViewMode(mode))
        .style(iced::widget::button::text)
        .padding([4, 6])
        .into()
    };

    let tabs = row![
        tab_btn("Artists", ViewMode::Artists, icons::ICON_MUSIC),
        tab_btn("Albums", ViewMode::Albums, icons::ICON_LIST),
        tab_btn("Genres", ViewMode::Genres, icons::ICON_PODIUM),
    ]
    .spacing(4);

    let sidebar_search = row![
        text(icons::ICON_SEARCH)
            .font(icons::NERD_FONT_MONO)
            .size(11)
            .color(theme::overlay0()),
        text_input("Filter…", &state.sidebar_search)
            .on_input(Message::SidebarSearchChanged)
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
            build_sidebar_list(items, selected, active, Message::SelectArtist)
        }
        ViewMode::Albums => {
            let items = state.albums();
            let selected = state.selected_album.clone();
            let active = state.current_track.as_ref().map(|t| t.album.clone());
            build_sidebar_list(items, selected, active, Message::SelectAlbum)
        }
        ViewMode::Genres => {
            let items = state.genres();
            let selected = state.selected_genre.clone();
            build_sidebar_list(items, selected, None, Message::SelectGenre)
        }
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
            container(Space::new(Length::Fill, Length::Fixed(3.0))).style(move |_| {
                iced::widget::container::Style {
                    background: Some(iced::Background::Color(theme::surface0())),
                    ..Default::default()
                }
            }),
        )
        .padding([2, 0]),
    )
    .on_press(Message::PlaylistDragStart)
    .on_enter(Message::HoverPlaylistResizer(true))
    .on_exit(Message::HoverPlaylistResizer(false));

    let sidebar_body = column![
        container(tabs).padding([4, 4]),
        sidebar_search,
        list_with_context,
        pl_resize,
        playlist_panel,
    ]
    .spacing(0);

    container(sidebar_body)
        .style(theme::sidebar)
        .width(Length::Fixed(state.sidebar_width))
        .height(Length::Fill)
        .into()
}

fn build_sidebar_list(
    items: Vec<String>,
    selected: Option<String>,
    active: Option<String>,
    make_msg: fn(String) -> Message,
) -> Element<'static, Message> {
    if items.is_empty() {
        return container(text("Nothing here").color(theme::overlay0()).size(13))
            .center_x(Length::Fill)
            .padding([12, 0])
            .width(Length::Fill)
            .into();
    }

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

            let msg = make_msg(name.clone());
            let btn = button(text(name).color(color).size(13).width(Length::Fill))
                .on_press(msg)
                .style(iced::widget::button::text)
                .width(Length::Fill)
                .padding([5, 10]);

            let styled: Element<'static, Message> = if is_selected {
                container(btn).style(theme::selected_row).width(Length::Fill).into()
            } else {
                container(btn).width(Length::Fill).into()
            };
            styled
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
                .size(13)
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
            let hovered_pl = state.hovered_playlist.clone();

            let rows: Vec<Element<Message>> = playlists
                .into_iter()
                .map(|pl| {
                    let is_sel = selected_pl.as_deref() == Some(pl.as_str());
                    let is_hov = hovered_pl.as_deref() == Some(pl.as_str());

                    let delete_btn: Element<Message> = if is_hov {
                        button(
                            text(icons::ICON_TRASH)
                                .font(icons::NERD_FONT_MONO)
                                .size(11)
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
                    let hover_enter = Message::HoverPlaylist(Some(pl.clone()));
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

                    let wrapped: Element<Message> = if is_sel {
                        container(pl_btn).style(theme::selected_row).width(Length::Fill).into()
                    } else {
                        container(pl_btn).width(Length::Fill).into()
                    };

                    mouse_area(wrapped)
                        .on_enter(hover_enter)
                        .on_exit(Message::HoverPlaylist(None))
                        .into()
                })
                .collect();

            column(rows).spacing(1).into()
        }
    } else {
        let selected_pl = state.selected_playlist.clone();
        let autoplaylists: Vec<(&'static str, &'static str, iced::Color)> = vec![
            ("Liked Songs", icons::ICON_HEART, theme::red()),
            ("Recently Played", icons::ICON_CLOCK, theme::accent()),
            ("Most Played", icons::ICON_PODIUM, theme::green()),
        ];
        let rows: Vec<Element<Message>> = autoplaylists
            .into_iter()
            .map(|(name, icon, color)| {
                let is_sel = selected_pl.as_deref() == Some(name);
                let pl_btn = button(
                    row![
                        text(icon).font(icons::NERD_FONT_MONO).size(11).color(color),
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

    container(
        column![tab_row, scrollable(list).height(Length::Fill)].spacing(0),
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

    let mut rows: Vec<Element<Message>> = Vec::new();

    // Column header row
    rows.push(build_column_header(visible_cols.clone(), col_widths.clone(), sort_col, sort_asc));

    // Toolbar
    rows.push(toolbar_view(state));

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

        for (album_name, tracks) in groups {
            let count = tracks.len();
            let album_label = album_name.clone();
            let album_hdr: Element<Message> = container(
                row![
                    text(icons::ICON_LIST)
                        .font(icons::NERD_FONT_MONO)
                        .size(11)
                        .color(theme::accent()),
                    text(album_label)
                        .size(12)
                        .color(theme::accent())
                        .font(icons::UI_FONT_BOLD)
                        .width(Length::Fill),
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

    let content = scrollable(column(rows).spacing(0))
        .id(scrollable::Id::new("tracklist_scroll"))
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
                .padding([4, 4])
                .width(w);

            mouse_area(hdr_btn)
                .on_right_press(Message::ToggleContextMenu(Some(
                    ContextMenuTarget::Header(col),
                )))
                .into()
        })
        .collect();

    container(row(header_cells).spacing(0).align_y(Alignment::Center))
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
            .size(12)
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
            text(icons::ICON_LIST).font(icons::NERD_FONT_MONO).size(11).color(group_color),
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
            let cell_text = match col {
                TableColumn::TrackNumber => track.track_number.map(|n| n.to_string()).unwrap_or_else(|| "·".to_string()),
                TableColumn::Title => {
                    if is_current { format!("▶ {}", track.title) } else { track.title.clone() }
                }
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
                TableColumn::Title => base_color,
                TableColumn::Artist | TableColumn::Album | TableColumn::Genre if is_current => {
                    theme::with_alpha(theme::accent(), 0.8)
                }
                TableColumn::Artist | TableColumn::Album | TableColumn::Genre => theme::subtext(),
                _ => theme::subtext(),
            };
            text(cell_text).size(13).color(cell_color).width(w).into()
        })
        .collect();

    let like_icon: Element<'static, Message> = if liked || is_selected || is_current {
        let like_msg = Message::ToggleLikeTrack(Track { cover_data: None, ..track.clone() });
        button(
            text(icons::ICON_HEART)
                .font(icons::NERD_FONT_MONO)
                .size(11)
                .color(if liked { theme::red() } else { theme::with_alpha(theme::overlay0(), 0.5) }),
        )
        .on_press(like_msg)
        .style(iced::widget::button::text)
        .padding([0, 4])
        .into()
    } else {
        Space::with_width(20).into()
    };

    let track_row_inner = row(cells)
        .push(like_icon)
        .spacing(0)
        .align_y(Alignment::Center)
        .padding([0, 12]);

    let row_styled: Element<'static, Message> = if is_selected {
        container(track_row_inner).style(theme::selected_row).height(Length::Fixed(ROW_H)).width(Length::Fill).into()
    } else if is_current {
        container(track_row_inner)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::with_alpha(theme::accent(), 0.08))),
                ..Default::default()
            })
            .height(Length::Fixed(ROW_H))
            .width(Length::Fill)
            .into()
    } else {
        container(track_row_inner).height(Length::Fixed(ROW_H)).width(Length::Fill).into()
    };

    let select_msg = Message::SelectTrack(track.clone());
    let ctx_msg = if is_multi {
        Message::ToggleContextMenu(Some(ContextMenuTarget::MultipleTracks(multi_selected.to_vec())))
    } else {
        Message::ToggleContextMenu(Some(ContextMenuTarget::Track(track)))
    };

    let row_btn = button(row_styled)
        .on_press(select_msg)
        .style(iced::widget::button::text)
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
            TableColumn::Year => Length::Fixed(44.0),
            TableColumn::DiscNumber => Length::Fixed(36.0),
            TableColumn::Duration => Length::Fixed(52.0),
            TableColumn::Plays => Length::Fixed(44.0),
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
