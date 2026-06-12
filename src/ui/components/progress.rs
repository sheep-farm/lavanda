use std::time::Duration;

use iced::widget::{row, slider, text};
use iced::{Alignment, Element, Length};

use crate::app::Message;
use crate::ui::theme;

fn fmt_duration(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn progress_bar<'a>(position: Duration, duration: Duration) -> Element<'a, Message> {
    let progress = if duration.is_zero() {
        0.0
    } else {
        position.as_secs_f64() / duration.as_secs_f64()
    };

    let bar = slider(0.0..=1.0f64, progress, move |v| {
        Message::Seek(Duration::from_secs_f64(v * duration.as_secs_f64()))
    })
    .step(0.001)
    .width(Length::Fill);

    row![
        text(fmt_duration(position))
            .color(theme::subtext())
            .size(13),
        bar,
        text(fmt_duration(duration))
            .color(theme::subtext())
            .size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
