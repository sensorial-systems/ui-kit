//! 2D point in XY graph data coordinates.

use serde::{Deserialize, Serialize};

/// A 2D point `(x, y)` in data coordinate space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct XyPoint {
    pub x: f32,
    pub y: f32,
}

impl XyPoint {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<(f32, f32)> for XyPoint {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<[f32; 2]> for XyPoint {
    #[inline]
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}
