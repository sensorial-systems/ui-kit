//! Data series for XY graphs.

use super::point::XyPoint;
use crate::Color;
use serde::{Deserialize, Serialize};

/// A plotted series of points or curve on an XY graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XySeries {
    pub name: String,
    pub points: Vec<XyPoint>,
    pub stroke_color: Color,
    pub stroke_width: f32,
    pub fill_color: Option<Color>,
    pub fill_baseline: f32,
}

impl XySeries {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            points: Vec::new(),
            stroke_color: [0.3, 0.7, 1.0, 1.0],
            stroke_width: 2.0,
            fill_color: None,
            fill_baseline: 0.0,
        }
    }

    pub fn with_points(mut self, points: Vec<XyPoint>) -> Self {
        self.points = points;
        self
    }

    pub fn with_stroke(mut self, color: Color, width: f32) -> Self {
        self.stroke_color = color;
        self.stroke_width = width;
        self
    }

    pub fn with_fill(mut self, color: Color, baseline: f32) -> Self {
        self.fill_color = Some(color);
        self.fill_baseline = baseline;
        self
    }

    /// Construct a series by evaluating a function `f(x)` over `samples` steps in `[x_min, x_max]`.
    pub fn from_function(
        name: impl Into<String>,
        x_min: f32,
        x_max: f32,
        samples: usize,
        f: impl Fn(f32) -> f32,
    ) -> Self {
        let count = samples.max(2);
        let step = (x_max - x_min) / (count - 1) as f32;
        let mut points = Vec::with_capacity(count);
        for i in 0..count {
            let x = x_min + i as f32 * step;
            let y = f(x);
            points.push(XyPoint::new(x, y));
        }
        Self::new(name).with_points(points)
    }
}
