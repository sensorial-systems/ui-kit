//! Shaded vertical region for XY graphs.

use crate::Color;
use serde::{Deserialize, Serialize};

/// A vertically shaded band across a horizontal interval `[x_min, x_max]`.
/// Used for passbands, stopbands, or highlight zones.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XyShadedRegion {
    pub x_min: f32,
    pub x_max: f32,
    pub color: Color,
    pub label: Option<String>,
}

impl XyShadedRegion {
    pub fn new(x_min: f32, x_max: f32, color: Color) -> Self {
        let (min, max) = if x_min <= x_max {
            (x_min, x_max)
        } else {
            (x_max, x_min)
        };
        Self {
            x_min: min,
            x_max: max,
            color,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}
