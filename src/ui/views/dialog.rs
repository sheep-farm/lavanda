use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::app::{AppState, EditField, Message};
use crate::ui::theme;

pub const TITLE_INPUT_ID: &str = "edit_title";

pub fn view(state: &AppState) -> Option<Element<'_, Message>> {
    let es = state.edit_state.as_ref()?;

    let field = |label: &'static str, value: &str, id: Option<&'static str>, ef: EditField| {
        let mut input = text_input("", value)
            .on_input(move |v| Message::EditField(ef.clone(), v))
            .style(theme::dialog_input)
            .size(13)
            .padding([6, 8])
            .width(Length::Fill);
        if let Some(id) = id {
            input = input.id(text_input::Id::new(id));
        }
        row![
            text(label).color(theme::subtext()).size(13).width(90),
            input,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    };

    let save_label = if es.saving { "Saving…" } else { "Save" };

    let buttons = row![
        Space::with_width(Length::Fill),
        button(text("Cancel").size(13).color(theme::subtext()))
            .on_press(Message::CancelEdit)
            .style(iced::widget::button::text)
            .padding([6, 14]),
        button(text(save_label).size(13).color(theme::base()))
            .on_press_maybe((!es.saving).then_some(Message::SaveMetadata))
            .style(accent_button)
            .padding([6, 14]),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut body = column![
        text("Edit Metadata")
            .size(15)
            .font(crate::ui::icons::UI_FONT_BOLD)
            .color(theme::text()),
        Space::with_height(20),
        field("Title", &es.title, Some(TITLE_INPUT_ID), EditField::Title),
        Space::with_height(10),
        field("Artist", &es.artist, None, EditField::Artist),
        Space::with_height(10),
        field("Album", &es.album, None, EditField::Album),
        Space::with_height(10),
        field("Track #", &es.track_number, None, EditField::TrackNumber),
        Space::with_height(16),
    ]
    .spacing(0);

    if let Some(ref err) = es.error {
        body = body.push(text(err).color(theme::red()).size(12));
        body = body.push(Space::with_height(8));
    }

    body = body.push(buttons);

    let dialog = container(body.padding(24))
        .style(theme::dialog_card)
        .width(420);

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

fn accent_button(
    _: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    use iced::widget::button::Style;
    use iced::Border;

    let bg = match status {
        iced::widget::button::Status::Hovered => {
            theme::lerp_color(theme::accent(), theme::text(), 0.1)
        }
        iced::widget::button::Status::Pressed => {
            theme::lerp_color(theme::accent(), theme::base(), 0.15)
        }
        _ => theme::accent(),
    };

    Style {
        background: Some(iced::Background::Color(bg)),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        text_color: theme::base(),
        ..Default::default()
    }
}
