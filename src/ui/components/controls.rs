use iced::widget::{button, row, slider, text, Space};
use iced::{Alignment, Element};

use crate::app::Message;
use crate::audio::PlaybackState;
use crate::ui::{icons, theme};

pub fn playback_controls<'a>(
    state: &PlaybackState,
    volume: f32,
    shuffle: bool,
    repeat: bool,
) -> Element<'a, Message> {
    let play_icon = match state {
        PlaybackState::Playing => icons::ICON_PAUSE,
        _ => icons::ICON_PLAY,
    };

    let icon_btn = |icon: &'static str, msg: Message| {
        button(
            text(icon)
                .font(icons::NERD_FONT_MONO)
                .color(theme::text())
                .size(20),
        )
        .on_press(msg)
        .style(iced::widget::button::text)
        .padding([4, 12])
    };

    let shuffle_color = if shuffle {
        theme::accent()
    } else {
        theme::overlay0()
    };
    let repeat_color = if repeat {
        theme::accent()
    } else {
        theme::overlay0()
    };

    let vol_slider = slider(0.0..=1.0f32, volume, Message::VolumeChanged)
        .step(0.01)
        .width(100);

    row![
        button(
            text(icons::ICON_SHUFFLE)
                .font(icons::NERD_FONT_MONO)
                .color(shuffle_color)
                .size(18),
        )
        .on_press(Message::ToggleShuffle)
        .style(iced::widget::button::text)
        .padding([4, 8]),
        icon_btn(icons::ICON_PREV, Message::PreviousTrack),
        icon_btn(play_icon, Message::PlayPause),
        icon_btn(icons::ICON_NEXT, Message::NextTrack),
        button(
            text(icons::ICON_REPEAT)
                .font(icons::NERD_FONT_MONO)
                .color(repeat_color)
                .size(18),
        )
        .on_press(Message::ToggleRepeat)
        .style(iced::widget::button::text)
        .padding([4, 8]),
        Space::with_width(16),
        text(icons::ICON_VOL_UP)
            .font(icons::NERD_FONT_MONO)
            .color(theme::subtext())
            .size(18),
        vol_slider,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}
