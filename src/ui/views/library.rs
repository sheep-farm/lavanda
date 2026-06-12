use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text, Space};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, Focus, Message};
use crate::ui::{theme, views};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let sidebar = folder_sidebar(state);
    let track_list = track_list_view(state);

    let drag_handle = mouse_area(
        container(Space::new(Length::Fixed(4.0), Length::Fill)).style(|_| {
            iced::widget::container::Style {
                background: Some(iced::Background::Color(crate::ui::theme::surface0())),
                ..Default::default()
            }
        }),
    )
    .on_press(Message::SidebarDragStart);

    let base: Element<Message> = row![sidebar, drag_handle, track_list]
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    if let Some(dialog) = views::dialog::view(state) {
        stack![base, dialog].into()
    } else {
        base
    }
}

fn folder_sidebar(state: &AppState) -> Element<'_, Message> {
    let is_sidebar_focused = state.focus == Focus::Sidebar;
    let items: Element<Message> = column(
        state
            .folders
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let is_selected = state.selected_folder.as_ref() == Some(path);
                let is_cursor = is_sidebar_focused && i == state.sidebar_cursor;
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

                let label = text(name)
                    .color(if is_selected {
                        theme::accent()
                    } else {
                        theme::text()
                    })
                    .size(14);

                let btn = button(label)
                    .on_press(Message::SelectFolder(path.clone()))
                    .style(iced::widget::button::text)
                    .width(Length::Fill)
                    .padding([6, 12]);

                if is_cursor {
                    container(btn)
                        .style(theme::cursor_row)
                        .width(Length::Fill)
                        .into()
                } else if is_selected {
                    container(btn)
                        .style(theme::selected_row)
                        .width(Length::Fill)
                        .into()
                } else {
                    container(btn).width(Length::Fill).into()
                }
            })
            .collect::<Vec<_>>(),
    )
    .spacing(2)
    .into();

    container(
        column![
            text(state.strings.sidebar_folders)
                .color(theme::subtext())
                .size(11)
                .font(crate::ui::icons::UI_FONT_BOLD),
            Space::with_height(8),
            scrollable(items).height(Length::Fill),
        ]
        .padding(8),
    )
    .style(theme::sidebar)
    .width(state.sidebar_width)
    .height(Length::Fill)
    .into()
}

fn track_list_view(state: &AppState) -> Element<'_, Message> {
    if state.tracks.is_empty() {
        return container(
            text(if state.selected_folder.is_some() {
                state.strings.no_tracks_found
            } else {
                state.strings.select_folder
            })
            .color(theme::overlay0())
            .size(15),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let current_id = state.current_track.as_ref().map(|t| t.id);
    let is_tracklist_focused = state.focus == Focus::TrackList;

    // Agrupa faixas por álbum mantendo a ordem de inserção
    let mut groups: Vec<(String, Vec<&crate::library::models::Track>)> = Vec::new();
    for track in &state.tracks {
        if let Some(last) = groups.last_mut() {
            if last.0 == track.album {
                last.1.push(track);
                continue;
            }
        }
        groups.push((track.album.clone(), vec![track]));
    }

    let mut rows: Vec<Element<Message>> = Vec::new();
    let mut track_idx: usize = 0;

    for (album_name, tracks) in groups.into_iter() {
        let n = tracks.len();
        let header = container(
            row![
                text(album_name)
                    .color(theme::accent())
                    .size(13)
                    .font(crate::ui::icons::UI_FONT_BOLD),
                Space::with_width(Length::Fill),
                text(state.strings.track_count(n))
                    .color(theme::overlay0())
                    .size(11),
            ]
            .align_y(Alignment::Center)
            .padding([6, 12]),
        )
        .style(theme::album_header)
        .width(Length::Fill);

        rows.push(header.into());

        for track in tracks.into_iter() {
            let is_current = current_id == Some(track.id);
            let is_cursor = is_tracklist_focused && track_idx == state.track_cursor;
            let row_color = if is_current {
                theme::accent()
            } else {
                theme::text()
            };

            let num = track
                .track_number
                .map(|n| n.to_string())
                .unwrap_or_else(|| "·".to_string());

            let track_row = row![
                text(num).color(theme::overlay0()).size(13).width(30),
                text(track.title.clone())
                    .color(row_color)
                    .size(14)
                    .width(Length::FillPortion(3)),
                text(track.artist.clone())
                    .color(theme::subtext())
                    .size(13)
                    .width(Length::FillPortion(2)),
                text(track.duration_str())
                    .color(theme::subtext())
                    .size(13)
                    .width(60),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .padding([5, 12]);

            let styled = if is_cursor {
                container(track_row)
                    .style(theme::cursor_row)
                    .width(Length::Fill)
            } else if is_current {
                container(track_row)
                    .style(theme::selected_row)
                    .width(Length::Fill)
            } else {
                container(track_row).width(Length::Fill)
            };

            let track_btn = button(styled)
                .on_press(Message::PlayTrack(track.clone()))
                .style(iced::widget::button::text)
                .width(Length::Fill)
                .padding(0);

            rows.push(
                mouse_area(track_btn)
                    .on_right_press(Message::OpenEditDialog(track.clone()))
                    .into(),
            );
            track_idx += 1;
        }

        rows.push(Space::with_height(8).into());
    }

    container(scrollable(column(rows).spacing(1)))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
