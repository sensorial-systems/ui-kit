//! Axis tick mark definition.

use serde::{Deserialize, Serialize};

/// An individual tick along an XY graph axis.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XyTick {
    pub value: f32,
    pub label: String,
    pub is_major: bool,
}

impl XyTick {
    pub fn new(value: f32, label: impl Into<String>, is_major: bool) -> Self {
        Self {
            value,
            label: label.into(),
            is_major,
        }
    }

    pub fn major(value: f32, label: impl Into<String>) -> Self {
        Self::new(value, label, true)
    }

    pub fn minor(value: f32) -> Self {
        Self::new(value, String::new(), false)
    }
}
