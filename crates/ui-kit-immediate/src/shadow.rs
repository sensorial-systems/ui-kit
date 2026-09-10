//! Soft back shadow descriptor and helpers for distance-field rendered shadows.

use crate::immediate::Ui;
use serde::{Deserialize, Serialize};
use ui_kit_core::{Color, Point as Vec2, Rect};

/// Configuration for an analytical signed distance field soft shadow.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    /// Spatial offset of the shadow casting shape (e.g. `Vec2::new(0.0, 6.0)`).
    pub offset: Vec2,
    /// Corner radius of the shadow source shape.
    pub radius: f32,
    /// Distance field blur radius (smoothstep falloff span).
    pub blur: f32,
    /// Color and peak opacity of the shadow.
    pub color: Color,
}

impl Shadow {
    pub fn new(offset: Vec2, radius: f32, blur: f32, color: Color) -> Self {
        Self {
            offset,
            radius,
            blur,
            color,
        }
    }

    /// Soft elevation shadow for floating panels and cards.
    pub fn elevation(radius: f32, blur: f32, color: Color) -> Self {
        Self {
            offset: Vec2::new(0.0, blur * 0.25),
            radius,
            blur,
            color,
        }
    }

    /// Subdued card shadow preset for light or dark modes.
    pub fn card(dark: bool) -> Self {
        if dark {
            Self {
                offset: Vec2::new(0.0, 3.0),
                radius: 20.0,
                blur: 8.0,
                color: [0.0, 0.0, 0.0, 0.22],
            }
        } else {
            Self {
                offset: Vec2::new(0.0, 3.0),
                radius: 20.0,
                blur: 8.0,
                color: [0.06, 0.09, 0.18, 0.05],
            }
        }
    }

    /// Render this shadow behind the given bounds.
    pub fn show(&self, ui: &mut Ui, rect: Rect) {
        let shadow_rect = Rect {
            x: rect.x + self.offset.x,
            y: rect.y + self.offset.y,
            width: rect.width,
            height: rect.height,
        };
        let prev_fill = ui.style.fill;
        let prev_radius = ui.style.radius;
        ui.style.fill = self.color;
        ui.style.radius = self.radius;
        ui.paint(
            "shadow",
            "",
            shadow_rect,
            self.blur,
            Default::default(),
            false,
        );
        ui.style.fill = prev_fill;
        ui.style.radius = prev_radius;
    }
}
