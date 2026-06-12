use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, Message};
use crate::ui::theme;

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    if !state.help_visible {
        return None;
    }

    let section = |title: &'static str| {
        text(title)
            .color(theme::accent())
            .size(12)
            .font(crate::ui::icons::UI_FONT_BOLD)
    };

    let bind = |key: &'static str, action: &'static str| -> Element<'_, Message> {
        row![
            text(key)
                .color(theme::text())
                .size(12)
                .font(crate::ui::icons::UI_FONT_BOLD)
                .width(120),
            text(action).color(theme::subtext()).size(12),
        ]
        .align_y(Alignment::Center)
        .into()
    };

    let body = column![
        text("Keyboard Shortcuts")
            .size(15)
            .font(crate::ui::icons::UI_FONT_BOLD)
            .color(theme::text()),
        Space::with_height(16),
        section("Navigation"),
        Space::with_height(6),
        bind("↑ / ↓", "move cursor"),
        bind("← / →", "switch panel"),
        bind("Enter", "activate"),
        Space::with_height(12),
        section("Playback"),
        Space::with_height(6),
        bind("Space", "play / pause"),
        bind("Shift+← / →", "seek backward / forward"),
        bind("n / p", "next / previous track"),
        bind("s", "toggle shuffle"),
        bind("r", "toggle repeat"),
        bind("+ / -", "volume up / down"),
        Space::with_height(12),
        section("Library"),
        Space::with_height(6),
        bind("/", "search / filter tracks"),
        bind("m", "edit track metadata"),
        bind("i", "toggle play-on-click"),
        Space::with_height(12),
        section("General"),
        Space::with_height(6),
        bind("Ctrl+K", "toggle this help"),
        bind("Escape", "close overlay"),
        Space::with_height(16),
        text("Press Ctrl+K or Escape to close")
            .color(theme::overlay0())
            .size(11),
    ]
    .spacing(3)
    .padding(24);

    let dialog = container(body).style(theme::dialog_card).width(360);

    Some(
        container(dialog)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::modal_backdrop)
            .into(),
    )
}
