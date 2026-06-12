use iced::widget::{column, container, image, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, Message};
use crate::ui::components::{controls, progress};
use crate::ui::{icons, theme};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let track_info: Element<Message> = if let Some(track) = &state.current_track {
        column![
            text(&track.artist).color(theme::subtext()).size(13),
            text(&track.title)
                .color(theme::text())
                .size(20)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..crate::ui::icons::UI_FONT
                }),
            text(format!(
                "{} ({})",
                track.album,
                track
                    .track_number
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            ))
            .color(theme::subtext())
            .size(13),
        ]
        .spacing(4)
        .into()
    } else {
        column![text(state.strings.no_track)
            .color(theme::overlay0())
            .size(16),]
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

    let player_row = row![
        cover,
        Space::with_width(16),
        column![
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
        .width(Length::Fill)
        .spacing(0),
    ]
    .spacing(0)
    .align_y(Alignment::Center)
    .padding(16);

    container(player_row)
        .style(theme::player_panel)
        .width(Length::Fill)
        .into()
}
