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

        let src = &self.bins;
        if src.is_empty() {
            return vec![frame.into_geometry()];
        }

        const BAR_W: f32 = 1.5;
        const GAP: f32 = 3.0;
        const SLOT: f32 = BAR_W + GAP;

        let n_bars = ((bounds.width / SLOT).floor() as usize).clamp(1, src.len());
        let center_y = bounds.height / 2.0;
        let x_offset = (bounds.width - n_bars as f32 * SLOT) / 2.0;

        for i in 0..n_bars {
            // Evenly resample src into n_bars
            let src_idx = i * src.len() / n_bars;
            let amp = src[src_idx];

            if amp < 0.005 {
                continue;
            }

            let x = x_offset + i as f32 * SLOT + (SLOT - BAR_W) / 2.0;
            let half_h = amp * center_y * 0.88;

            let path = Path::rectangle(
                Point::new(x, center_y - half_h),
                Size::new(BAR_W, half_h * 2.0),
            );
            frame.fill(&path, self.accent);
        }

        vec![frame.into_geometry()]
    }
}

pub fn spectrum_view(bins: &[f32], width: Length, height: Length) -> Element<'_, Message> {
    Canvas::new(SpectrumCanvas {
        bins: bins.to_vec(),
        accent: theme::accent(),
    })
    .width(width)
    .height(height)
    .into()
}
