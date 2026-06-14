use iced::widget::{button, column, container, image, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, Message};
use crate::ui::components::{controls, progress, spectrum};
use crate::ui::{icons, theme};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let track_info: Element<Message> = if let Some(track) = &state.current_track {
        let like_btn = button(
            text(icons::ICON_HEART)
                .font(icons::NERD_FONT_MONO)
                .size(15)
                .color(if track.liked { theme::red() } else { theme::overlay0() }),
        )
        .on_press(Message::KeyboardLike)
        .style(iced::widget::button::text)
        .padding([0, 4]);

        let artist_btn = button(
            text(&track.artist)
                .color(theme::subtext())
                .size(13),
        )
        .on_press(Message::FocusArtistName)
        .style(iced::widget::button::text)
        .padding(0);

        let title_btn = button(
            text(&track.title)
                .color(theme::text())
                .size(20)
                .font(icons::UI_FONT_BOLD),
        )
        .on_press(Message::FocusSongName)
        .style(iced::widget::button::text)
        .padding(0);

        let album_label = format!(
            "{} ({})",
            track.album,
            track.track_number.map(|n| n.to_string()).unwrap_or_default()
        );
        let album_btn = button(
            text(album_label).color(theme::subtext()).size(13),
        )
        .on_press(Message::FocusAlbumName)
        .style(iced::widget::button::text)
        .padding(0);

        row![
            column![artist_btn, title_btn, album_btn,]
                .spacing(4)
                .width(Length::Fill),
            like_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Start)
        .into()
    } else {
        column![text(state.strings.no_track)
            .color(theme::overlay0())
            .size(16)]
        .into()
    };

    // Capa do álbum
    let cover: Element<Message> = if let Some(data) = state
        .current_track
        .as_ref()
        .and_then(|t| t.cover_data.as_ref())
    {
        let handle = image::Handle::from_bytes(data.clone());
        image(handle)
            .width(180)
            .height(180)
            .content_fit(iced::ContentFit::Cover)
            .into()
    } else {
        container(
            text(icons::ICON_MUSIC)
                .font(icons::NERD_FONT_MONO)
                .color(theme::overlay0())
                .size(48),
        )
        .width(180)
        .height(180)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(theme::card)
        .into()
    };

    let info_col = column![
        track_info,
        Space::with_height(12),
        progress::progress_bar(state.position, state.duration),
        Space::with_height(8),
        controls::playback_controls(
            &state.playback_state,
            state.volume,
            state.shuffle,
            state.repeat,
        ),
    ]
    .spacing(0);

    let mut player_row = row![cover, Space::with_width(16)];

    if state.show_spectrum {
        player_row = player_row
            .push(info_col.width(Length::FillPortion(5)))
            .push(Space::with_width(16))
            .push(spectrum::spectrum_view(
                &state.spectrum,
                Length::FillPortion(3),
                Length::Fixed(180.0),
            ));
    } else {
        player_row = player_row.push(info_col.width(Length::Fill));
    }

    let player_row = player_row
        .spacing(0)
        .align_y(Alignment::Center)
        .padding(16);

    container(player_row)
        .style(theme::player_panel)
        .width(Length::Fill)
        .into()
}
