//! Instrument-panel components: titled panels, needle dials, oscilloscope
//! traces and indicator lamps, drawn in the palette's colours unless a
//! colour is given.
use super::theme::{base, DANGER};
use super::Label;
use crate::vector::{circle, line, polyline};
use crate::Ui;
use ui_kit_core::{Color, Point, Rect, Style, TextAlign};

/// A bordered, square-cornered panel with a title in its top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstrumentPanel {
    fill: Option<Color>,
    stroke: Option<Color>,
    title_size: f32,
    title_color: Option<Color>,
}

impl Default for InstrumentPanel {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            title_size: 15.0,
            title_color: None,
        }
    }
}

impl InstrumentPanel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }
    pub fn stroke(mut self, color: Color) -> Self {
        self.stroke = Some(color);
        self
    }
    pub fn title_size(mut self, size: f32) -> Self {
        self.title_size = size.max(1.0);
        self
    }
    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = Some(color);
        self
    }
    /// Height taken by the title strip (zero for an empty title).
    pub fn title_height(&self) -> f32 {
        self.title_size * 2.0
    }
    /// Draws the panel and returns the content area below the title.
    pub fn show(&self, ui: &mut Ui, rect: Rect, title: &str) -> Rect {
        let p = ui.theme.palette;
        let style = Style {
            fill: self.fill.unwrap_or(p.surface),
            stroke: self.stroke.unwrap_or(p.border),
            stroke_width: 1.0,
            radius: 0.0,
            ..base(ui)
        };
        ui.with_style(style, |ui| ui.panel(rect));
        if title.is_empty() {
            return rect;
        }
        let strip = Rect::new(
            rect.x + 10.0,
            rect.y + 4.0,
            rect.width - 20.0,
            self.title_height() - 4.0,
        );
        Label::new()
            .size(self.title_size)
            .weight(600)
            .color(self.title_color.unwrap_or(p.text))
            .show(ui, strip, title);
        Rect::new(
            rect.x,
            rect.y + self.title_height(),
            rect.width,
            (rect.height - self.title_height()).max(0.0),
        )
    }
}

/// A needle dial over a 270 degree scale with a numeric readout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dial {
    min: f32,
    max: f32,
    danger_from: f32,
    scale_color: Option<Color>,
    needle_color: Option<Color>,
    readout_size: f32,
    ticks: usize,
}

impl Dial {
    /// A dial spanning `min` to `max`.
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max: if max > min { max } else { min + 1.0 },
            danger_from: 0.85,
            scale_color: None,
            needle_color: None,
            readout_size: 18.0,
            ticks: 40,
        }
    }
    /// Fraction of the scale from which ticks are drawn in the danger colour.
    pub fn danger_from(mut self, fraction: f32) -> Self {
        self.danger_from = fraction.clamp(0.0, 1.0);
        self
    }
    pub fn scale_color(mut self, color: Color) -> Self {
        self.scale_color = Some(color);
        self
    }
    pub fn needle_color(mut self, color: Color) -> Self {
        self.needle_color = Some(color);
        self
    }
    pub fn readout_size(mut self, size: f32) -> Self {
        self.readout_size = size.max(1.0);
        self
    }
    pub fn ticks(mut self, ticks: usize) -> Self {
        self.ticks = ticks.clamp(2, 200);
        self
    }
    /// Draws the dial in `rect` (already inside a panel) with `readout` under it.
    pub fn show(&self, ui: &mut Ui, rect: Rect, value: f32, readout: &str) {
        let p = ui.theme.palette;
        let scale_color = self.scale_color.unwrap_or(p.text);
        let needle_color = self.needle_color.unwrap_or(p.danger);
        let readout_height = self.readout_size * 1.6;
        let radius = (rect.width * 0.42).min((rect.height - readout_height) * 0.5 - 4.0);
        if radius < 6.0 {
            return;
        }
        let middle = Point::new(rect.x + rect.width * 0.5, rect.y + 4.0 + radius);
        let start = std::f32::consts::PI * 0.75;
        let sweep = std::f32::consts::PI * 1.5;
        for i in 0..=self.ticks {
            let f = i as f32 / self.ticks as f32;
            let a = start + f * sweep;
            let major = i % 5 == 0;
            let length = if major { radius * 0.18 } else { radius * 0.09 };
            let at = |d: f32| Point::new(middle.x + a.cos() * d, middle.y + a.sin() * d);
            line(
                ui,
                at(radius),
                at(radius - length),
                if major { 1.8 } else { 1.0 },
                if f >= self.danger_from {
                    DANGER
                } else {
                    scale_color
                },
            );
        }
        let fraction = if value.is_finite() {
            ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let a = start + fraction * sweep;
        line(
            ui,
            middle,
            Point::new(
                middle.x + a.cos() * (radius * 0.8),
                middle.y + a.sin() * (radius * 0.8),
            ),
            2.5,
            needle_color,
        );
        circle(ui, middle, radius * 0.07 + 1.5, scale_color);
        Label::new()
            .size(self.readout_size)
            .weight(600)
            .color(scale_color)
            .centered()
            .show(
                ui,
                Rect::new(
                    rect.x,
                    rect.bottom() - readout_height,
                    rect.width,
                    readout_height,
                ),
                readout,
            );
    }
}

/// A trace of values around a centre line, scaled to `+-scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oscilloscope {
    scale: f32,
    color: Option<Color>,
    divisions: usize,
    line_width: f32,
}

impl Oscilloscope {
    pub fn new(scale: f32) -> Self {
        Self {
            scale: if scale > 0.0 { scale } else { 1.0 },
            color: None,
            divisions: 4,
            line_width: 1.3,
        }
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn divisions(mut self, divisions: usize) -> Self {
        self.divisions = divisions.max(1);
        self
    }
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.5);
        self
    }
    pub fn show(&self, ui: &mut Ui, rect: Rect, values: &[f32]) {
        let p = ui.theme.palette;
        let inset = Rect::new(
            rect.x + 6.0,
            rect.y + 6.0,
            rect.width - 12.0,
            rect.height - 12.0,
        );
        let middle_y = inset.y + inset.height * 0.5;
        for i in 1..self.divisions {
            let x = inset.x + i as f32 * inset.width / self.divisions as f32;
            line(
                ui,
                Point::new(x, inset.y),
                Point::new(x, inset.bottom()),
                0.5,
                p.border,
            );
        }
        line(
            ui,
            Point::new(inset.x, middle_y),
            Point::new(inset.right(), middle_y),
            1.0,
            p.border,
        );
        if values.len() < 2 {
            return;
        }
        let last = (values.len() - 1) as f32;
        let points: Vec<Point> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let v = if v.is_finite() {
                    v.clamp(-self.scale, self.scale)
                } else {
                    0.0
                };
                Point::new(
                    inset.x + i as f32 / last * inset.width,
                    middle_y - v / self.scale * inset.height * 0.5,
                )
            })
            .collect();
        polyline(ui, &points, self.line_width, self.color.unwrap_or(p.accent));
    }
}

/// A round lamp with a short label centred in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Indicator {
    radius: f32,
    on: Option<Color>,
    off: Option<Color>,
    text: Option<Color>,
    font_size: f32,
}

impl Indicator {
    pub fn new(radius: f32) -> Self {
        Self {
            radius: radius.max(1.0),
            on: None,
            off: None,
            text: None,
            font_size: (radius * 0.9).max(8.0),
        }
    }
    pub fn on_color(mut self, color: Color) -> Self {
        self.on = Some(color);
        self
    }
    pub fn off_color(mut self, color: Color) -> Self {
        self.off = Some(color);
        self
    }
    pub fn text_color(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.max(1.0);
        self
    }
    pub fn show(&self, ui: &mut Ui, middle: Point, lit: bool, label: &str) {
        let p = ui.theme.palette;
        let color = if lit {
            self.on.unwrap_or(p.accent)
        } else {
            self.off.unwrap_or(p.raised)
        };
        circle(ui, middle, self.radius, color);
        let r = self.radius;
        Label::new()
            .size(self.font_size)
            .weight(600)
            .color(self.text.unwrap_or(p.text))
            .centered()
            .show(
                ui,
                Rect::new(middle.x - r, middle.y - r, 2.0 * r, 2.0 * r),
                label,
            );
    }
}

/// Text centred both ways in `rect`, in `color` at `size`.
pub fn centered_text(ui: &mut Ui, rect: Rect, text: &str, size: f32, color: Color) {
    Label::new()
        .size(size)
        .color(color)
        .align(TextAlign::Center)
        .show(ui, rect, text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;

    #[test]
    fn panel_returns_content_below_title() {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        let panel = InstrumentPanel::new().title_size(15.0);
        let content = panel.show(&mut ui, Rect::new(0.0, 0.0, 200.0, 100.0), "SPEED");
        assert_eq!(content.y, 30.0);
        assert_eq!(content.height, 70.0);
        assert_eq!(
            panel
                .show(&mut ui, Rect::new(0.0, 0.0, 200.0, 100.0), "")
                .height,
            100.0
        );
    }

    #[test]
    fn indicator_label_is_centered_in_its_lamp() {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        Indicator::new(15.0).show(&mut ui, Point::new(100.0, 50.0), true, "8");
        let label = ui
            .end()
            .iter()
            .find(|c| c.kind == "label")
            .cloned()
            .expect("label painted");
        assert_eq!(label.style.text_align, TextAlign::Center);
        assert_eq!(label.style.text_padding, 0.0);
        assert_eq!(label.rect, Rect::new(85.0, 35.0, 30.0, 30.0));
    }

    #[test]
    fn dial_and_scope_do_not_panic_on_degenerate_input() {
        let mut ui = Ui::default();
        ui.begin(Input::default());
        Dial::new(0.0, 0.0).show(&mut ui, Rect::new(0.0, 0.0, 10.0, 10.0), f32::NAN, "");
        Oscilloscope::new(0.0).show(&mut ui, Rect::new(0.0, 0.0, 100.0, 50.0), &[f32::NAN, 1.0]);
        Oscilloscope::new(1.0).show(&mut ui, Rect::new(0.0, 0.0, 100.0, 50.0), &[1.0]);
    }
}
