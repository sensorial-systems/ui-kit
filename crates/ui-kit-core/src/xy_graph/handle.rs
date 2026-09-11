//! Draggable interactive handle on an XY graph.

use super::point::XyPoint;
use super::range::XyRange;
use crate::Color;
use serde::{Deserialize, Serialize};

/// An interactive control node displayed and draggable in data space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XyHandle {
    pub id: String,
    pub position: XyPoint,
    pub color: Color,
    pub radius: f32,
    pub label: Option<String>,
    pub constrain_x: Option<XyRange>,
    pub constrain_y: Option<XyRange>,
}

impl XyHandle {
    pub fn new(id: impl Into<String>, position: XyPoint) -> Self {
        Self {
            id: id.into(),
            position,
            color: [0.95, 0.75, 0.2, 1.0],
            radius: 6.0,
            label: None,
            constrain_x: None,
            constrain_y: None,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_constraints(mut self, x_range: Option<XyRange>, y_range: Option<XyRange>) -> Self {
        self.constrain_x = x_range;
        self.constrain_y = y_range;
        self
    }

    /// Clamp a requested position according to the handle's constraints.
    pub fn clamp_position(&self, target: XyPoint) -> XyPoint {
        let x = if let Some(rx) = self.constrain_x {
            rx.clamp(target.x)
        } else {
            target.x
        };
        let y = if let Some(ry) = self.constrain_y {
            ry.clamp(target.y)
        } else {
            target.y
        };
        XyPoint::new(x, y)
    }
}
