use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path};
use iced::{Color, Element, Length, Point, Rectangle, Size};

use crate::app::Message;
use crate::ui::theme;

struct SpectrumCanvas {
    bins: Vec<f32>,
    accent: Color,
}

impl canvas::Program<Message> for SpectrumCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry<iced::Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());

        let n = self.bins.len();
        if n == 0 {
            return vec![frame.into_geometry()];
        }

        let slot = bounds.width / n as f32;
        let bar_w = (slot * 0.45).max(1.0);
        let center_y = bounds.height / 2.0;

        for (i, &amp) in self.bins.iter().enumerate() {
            if amp < 0.005 {
                continue;
            }
            let x = i as f32 * slot + (slot - bar_w) / 2.0;
            let half_h = amp * center_y * 0.88;

            let path = Path::rectangle(
                Point::new(x, center_y - half_h),
                Size::new(bar_w, half_h * 2.0),
            );
            frame.fill(&path, self.accent);
        }

        vec![frame.into_geometry()]
    }
}

pub fn spectrum_view(bins: &[f32]) -> Element<'_, Message> {
    Canvas::new(SpectrumCanvas {
        bins: bins.to_vec(),
        accent: theme::accent(),
    })
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .into()
}
