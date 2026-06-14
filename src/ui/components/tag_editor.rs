use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::app::{Message, TagEditorState};
use crate::ui::{icons, theme};

pub fn view(state: &TagEditorState) -> Element<'_, Message> {
    let track_count = state.tracks.len();
    let subtitle = if track_count == 1 {
        format!("Editing: {}", state.tracks[0].title)
    } else {
        format!("Editing {track_count} tracks")
    };

    let album_apply_row: Element<Message> = if track_count == 1 {
        row![
            Space::with_width(22),
            checkbox("Apply changes to entire album", state.apply_to_album)
                .on_toggle(Message::UpdateTagFieldApplyToAlbum)
                .size(14)
                .text_size(13),
        ]
        .spacing(8)
        .into()
    } else {
        Space::with_height(0).into()
    };

    let cover_row: Element<Message> = row![
        checkbox("", state.apply_cover)
            .on_toggle(Message::ToggleTagFieldApplyCover)
            .size(14),
        text("Cover:").size(12).color(theme::subtext()).width(80),
        text_input("Path to image…", state.cover_path.as_deref().unwrap_or(""))
            .on_input(Message::UpdateTagFieldCoverPath)
            .style(theme::dialog_input)
            .size(13)
            .padding([4, 8])
            .width(Length::Fill),
        button(
            row![
                text(icons::ICON_SEARCH).font(icons::NERD_FONT_MONO).size(24).color(theme::text()),
                text("Online").size(12).color(theme::text()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SearchCoverOnline)
        .style(theme::secondary_button)
        .padding([4, 8]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into();

    let fields = scrollable(
        column![
            field_row("Title:", &state.title, Message::UpdateTagFieldTitle, state.apply_title, Message::ToggleTagFieldApplyTitle),
            field_row("Artist:", &state.artist, Message::UpdateTagFieldArtist, state.apply_artist, Message::ToggleTagFieldApplyArtist),
            field_row("Album:", &state.album, Message::UpdateTagFieldAlbum, state.apply_album, Message::ToggleTagFieldApplyAlbum),
            field_row("Genre:", &state.genre, Message::UpdateTagFieldGenre, state.apply_genre, Message::ToggleTagFieldApplyGenre),
            field_row("Year:", &state.year, Message::UpdateTagFieldYear, state.apply_year, Message::ToggleTagFieldApplyYear),
            field_row("Track #:", &state.track_number, Message::UpdateTagFieldTrackNumber, state.apply_track_num, Message::ToggleTagFieldApplyTrackNum),
            field_row("Disc #:", &state.disc_number, Message::UpdateTagFieldDiscNumber, state.apply_disc_num, Message::ToggleTagFieldApplyDiscNum),
            cover_row,
            Space::with_height(8),
            album_apply_row,
        ]
        .spacing(10),
    )
    .height(Length::Fixed(340.0));

    let buttons: Element<Message> = row![
        Space::with_width(Length::Fill),
        button(text("Cancel").size(13))
            .on_press(Message::CloseTagEditor)
            .style(theme::secondary_button)
            .padding([6, 14]),
        button(text("Save Tags").size(13))
            .on_press(Message::SaveTags)
            .style(theme::primary_button)
            .padding([6, 14]),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into();

    let hint: Element<Message> = row![
        text(icons::ICON_EDIT).font(icons::NERD_FONT_MONO).size(22).color(theme::overlay0()),
        text("Check boxes to enable each field. Unchecked fields are not written.")
            .size(11)
            .color(theme::overlay0()),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into();

    let card = container(
        column![
            row![
                text("Edit Tags").size(16).font(icons::UI_FONT_BOLD).color(theme::accent()),
                Space::with_width(Length::Fill),
                button(
                    text(icons::ICON_CLOSE).font(icons::NERD_FONT_MONO).color(theme::red()).size(28),
                )
                .on_press(Message::CloseTagEditor)
                .style(iced::widget::button::text),
            ]
            .align_y(Alignment::Center),
            text(subtitle).size(12).color(theme::subtext()),
            Space::with_height(12),
            hint,
            Space::with_height(12),
            fields,
            Space::with_height(16),
            buttons,
        ]
        .spacing(0)
        .padding(24),
    )
    .width(500)
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::mantle())),
        border: iced::Border { color: theme::accent(), width: 1.0, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: theme::base(),
            offset: iced::Vector { x: 0.0, y: 4.0 },
            blur_radius: 20.0,
        },
        ..Default::default()
    });

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
            ..Default::default()
        })
        .into()
}

fn field_row<'a>(
    label: &'static str,
    value: &'a str,
    msg: fn(String) -> Message,
    apply: bool,
    toggle: fn(bool) -> Message,
) -> Element<'a, Message> {
    row![
        checkbox("", apply).on_toggle(toggle).size(14),
        text(label).size(12).color(theme::subtext()).width(80),
        text_input("", value)
            .on_input(msg)
            .style(theme::dialog_input)
            .size(13)
            .padding([4, 8])
            .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
