use iced::widget::{button, checkbox, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::app::{Message, PlaylistDialogMode, PlaylistDialogState};
use crate::ui::theme;

pub fn view(state: &PlaylistDialogState) -> Element<'_, Message> {
    let title = match &state.mode {
        PlaylistDialogMode::Create => "New Playlist",
        PlaylistDialogMode::AddTrack(_) => "Add to Playlist",
        PlaylistDialogMode::Rename(_) => "Rename Playlist",
    };

    let body: Element<Message> = match &state.mode {
        PlaylistDialogMode::Create | PlaylistDialogMode::Rename(_) => {
            let submit_label = match &state.mode {
                PlaylistDialogMode::Create => "Create",
                _ => "Rename",
            };
            column![
                text("Name:").size(13).color(theme::subtext()),
                text_input("Playlist name…", &state.name_input)
                    .on_input(Message::PlaylistInputChanged)
                    .on_submit(Message::PlaylistDialogSubmit)
                    .style(theme::dialog_input)
                    .size(13)
                    .padding([6, 8])
                    .width(Length::Fill),
                Space::with_height(16),
                row![
                    Space::with_width(Length::Fill),
                    button(text("Cancel").size(13))
                        .on_press(Message::ClosePlaylistDialog)
                        .style(theme::secondary_button)
                        .padding([6, 14]),
                    button(text(submit_label).size(13))
                        .on_press(Message::PlaylistDialogSubmit)
                        .style(theme::primary_button)
                        .padding([6, 14]),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(8)
            .into()
        }

        PlaylistDialogMode::AddTrack(track) => {
            let playlists = crate::persist::get(|db| db.playlists.keys().cloned().collect::<Vec<_>>());

            let selected_pl = state.selected_playlist.clone();

            let list_body: Element<Message> = if playlists.is_empty() {
                text("No playlists yet. Create one first.").size(12).color(theme::overlay0()).into()
            } else {
                let items: Vec<Element<Message>> = playlists
                    .into_iter()
                    .map(|pl| {
                        let is_sel = selected_pl.as_deref() == Some(pl.as_str());
                        let msg = Message::PlaylistDialogSelect(pl.clone());
                        button(text(pl).size(13).color(if is_sel { theme::accent() } else { theme::text() }))
                            .on_press(msg)
                            .style(iced::widget::button::text)
                            .width(Length::Fill)
                            .padding([4, 8])
                            .into()
                    })
                    .collect();
                column(items).spacing(2).into()
            };

            let list_col: Element<Message> = column![
                text("Choose playlist:").size(12).color(theme::subtext()),
                Space::with_height(4),
                list_body,
            ]
            .spacing(4)
            .into();

            let can_submit = state.selected_playlist.is_some();
            let track_name = track.title.clone();

            column![
                text(format!("Adding: {track_name}")).size(12).color(theme::subtext()),
                Space::with_height(8),
                list_col,
                Space::with_height(8),
                checkbox("Also add entire album", state.add_album)
                    .on_toggle(Message::PlaylistDialogToggleAddAlbum)
                    .size(14)
                    .text_size(13),
                Space::with_height(16),
                row![
                    Space::with_width(Length::Fill),
                    button(text("Cancel").size(13))
                        .on_press(Message::ClosePlaylistDialog)
                        .style(theme::secondary_button)
                        .padding([6, 14]),
                    button(text("Add").size(13))
                        .on_press_maybe(can_submit.then_some(Message::PlaylistDialogSubmit))
                        .style(theme::primary_button)
                        .padding([6, 14]),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(4)
            .into()
        }
    };

    let card = container(
        column![
            text(title)
                .size(16)
                .font(crate::ui::icons::UI_FONT_BOLD)
                .color(theme::accent()),
            Space::with_height(16),
            body,
        ]
        .padding(24),
    )
    .width(360)
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::mantle())),
        border: iced::Border {
            color: theme::accent(),
            width: 1.0,
            radius: 8.0.into(),
        },
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
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.55,
            ))),
            ..Default::default()
        })
        .into()
}
