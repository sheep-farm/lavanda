use iced::widget::{column, container, image, text, Space};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, Message};
use crate::ui::components::{controls, progress, spectrum};
use crate::ui::{icons, theme};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let cover: Element<Message> = if let Some(data) = state
        .current_track
        .as_ref()
        .and_then(|t| t.cover_data.as_ref())
    {
        let handle = image::Handle::from_bytes(data.clone());
        image(handle)
            .width(320)
            .height(320)
            .content_fit(iced::ContentFit::Cover)
            .into()
    } else {
        container(
            text(icons::ICON_MUSIC)
                .font(icons::NERD_FONT_MONO)
                .color(theme::overlay0())
                .size(80),
        )
        .width(320)
        .height(320)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(theme::card)
        .into()
    };

    let track_info: Element<Message> = if let Some(track) = &state.current_track {
        column![
            text(&track.artist).color(theme::subtext()).size(13),
            text(&track.title)
                .color(theme::text())
                .size(22)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..icons::UI_FONT
                }),
            text(format!(
                "{} · {}",
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
        .align_x(Alignment::Center)
        .into()
    } else {
        text(state.strings.no_track)
            .color(theme::overlay0())
            .size(16)
            .into()
    };

    let mut layout = column![
        Space::with_height(Length::Fill),
        container(cover).center_x(Length::Fill),
        Space::with_height(24),
        container(track_info).center_x(Length::Fill),
        Space::with_height(20),
        container(progress::progress_bar(state.position, state.duration))
            .width(Length::Fill),
        Space::with_height(12),
        container(controls::playback_controls(
            &state.playback_state,
            state.volume,
            state.shuffle,
            state.repeat,
        ))
        .center_x(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    if state.show_spectrum {
        layout = layout
            .push(Space::with_height(16))
            .push(spectrum::spectrum_view(&state.spectrum, Length::Fill, Length::Fixed(56.0)));
    }

    layout = layout.push(Space::with_height(Length::Fill));

    container(layout)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::base())),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
