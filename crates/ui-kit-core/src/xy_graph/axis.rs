//! Axis configuration for XY graphs.

use super::range::XyRange;
use super::tick::XyTick;
use serde::{Deserialize, Serialize};

/// Configuration for an axis (X or Y) of an XY graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XyAxis {
    pub range: XyRange,
    pub label: String,
    pub unit: String,
    pub ticks: Vec<XyTick>,
    pub show_grid: bool,
}

impl XyAxis {
    pub fn new(range: XyRange) -> Self {
        Self {
            range,
            label: String::new(),
            unit: String::new(),
            ticks: Vec::new(),
            show_grid: true,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn with_ticks(mut self, ticks: Vec<XyTick>) -> Self {
        self.ticks = ticks;
        self
    }

    pub fn with_grid(mut self, show_grid: bool) -> Self {
        self.show_grid = show_grid;
        self
    }

    /// Automatically generate `count` uniformly spaced major ticks.
    pub fn with_uniform_ticks(mut self, count: usize) -> Self {
        if count <= 1 {
            return self;
        }
        let mut ticks = Vec::with_capacity(count);
        let span = self.range.span();
        let step = span / (count - 1) as f32;
        for i in 0..count {
            let val = self.range.min + i as f32 * step;
            let label = if val.abs() < 1e-4 {
                "0".to_string()
            } else if val.fract().abs() < 1e-3 {
                format!("{:.0}", val)
            } else {
                format!("{:.1}", val)
            };
            ticks.push(XyTick::major(val, label));
        }
        self.ticks = ticks;
        self
    }
}

impl Default for XyAxis {
    fn default() -> Self {
        Self::new(XyRange::default())
    }
}
