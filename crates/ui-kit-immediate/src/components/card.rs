use super::theme::is_dark;
use crate::Ui;
use ui_kit_core::{Color, Material, Rect, Style};

/// A frosted card: a translucent, blurred surface with a soft shadow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Card {
    glass: f32,
    radius: f32,
}

impl Default for Card {
    fn default() -> Self {
        Self {
            glass: 1.0,
            radius: 20.0,
        }
    }
}

impl Card {
    pub fn new() -> Self {
        Self::default()
    }

    /// How frosted the card is: 0 is flat, 1 the default, 2 very strong.
    pub fn glass(mut self, strength: f32) -> Self {
        self.glass = strength.max(0.0);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let p = ui.theme.palette;
        let dark = is_dark(&p);
        let shadow: Color = if dark {
            [0.0, 0.0, 0.0, 0.22]
        } else {
            [0.06, 0.09, 0.18, 0.05]
        };
        ui.shadow_box(
            Rect {
                y: rect.y + 3.0,
                ..rect
            },
            self.radius,
            8.0,
            shadow,
        );
        let style = Style {
            fill: if dark {
                [0.10, 0.10, 0.12, 0.75]
            } else {
                [1.0, 1.0, 1.0, 0.40]
            },
            gradient_end: Some(if dark {
                [0.08, 0.12, 0.20, 0.8]
            } else {
                [0.82, 0.88, 1.0, (self.glass * 0.42).min(0.65)]
            }),
            stroke: p.border,
            radius: self.radius,
            material: Material::Glass {
                blur: self.glass * 18.0,
            },
            ..super::theme::base(ui)
        };
        ui.with_style(style, |ui| ui.panel(rect));
    }
}

/// A plain rounded box: `fill`, with a border in `stroke` unless it is transparent.
pub fn panel(ui: &mut Ui, rect: Rect, fill: Color, stroke: Color, radius: f32) {
    let style = Style {
        fill,
        stroke,
        radius,
        stroke_width: if stroke[3] == 0.0 { 0.0 } else { 1.0 },
        ..super::theme::base(ui)
    };
    ui.with_style(style, |ui| ui.panel(rect));
}

/// A popup's surface: a card-coloured box with a border and shadow.
pub fn popup_surface(ui: &mut Ui, rect: Rect, radius: f32) {
    let p = ui.theme.palette;
    let shadow = if is_dark(&p) {
        [0.0, 0.0, 0.0, 0.25]
    } else {
        [0.06, 0.09, 0.18, 0.06]
    };
    ui.shadow_box(
        Rect {
            y: rect.y + 3.0,
            ..rect
        },
        radius,
        8.0,
        shadow,
    );
    panel(ui, rect, p.surface, p.border, radius);
}
