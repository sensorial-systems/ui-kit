//! 1D data interval for XY graph axes.

use serde::{Deserialize, Serialize};

/// A continuous 1D interval `[min, max]`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct XyRange {
    pub min: f32,
    pub max: f32,
}

impl XyRange {
    /// Construct a new range ensuring `min <= max`.
    pub fn new(a: f32, b: f32) -> Self {
        if a <= b {
            Self { min: a, max: b }
        } else {
            Self { min: b, max: a }
        }
    }

    /// Total span of the range (`max - min`).
    #[inline]
    pub fn span(&self) -> f32 {
        self.max - self.min
    }

    /// Check if a value falls inside the range (inclusive).
    #[inline]
    pub fn contains(&self, val: f32) -> bool {
        val >= self.min && val <= self.max
    }

    /// Clamp a value within `[min, max]`.
    #[inline]
    pub fn clamp(&self, val: f32) -> f32 {
        val.clamp(self.min, self.max)
    }

    /// Normalize a data value to `[0.0, 1.0]` along this range.
    #[inline]
    pub fn normalize(&self, val: f32) -> f32 {
        let span = self.span();
        if span.abs() < 1e-7 {
            0.5
        } else {
            (val - self.min) / span
        }
    }

    /// Denormalize a `[0.0, 1.0]` scalar back to data coordinates.
    #[inline]
    pub fn denormalize(&self, t: f32) -> f32 {
        self.min + t * self.span()
    }
}

impl Default for XyRange {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_normalization() {
        let r = XyRange::new(10.0, 30.0);
        assert_eq!(r.span(), 20.0);
        assert_eq!(r.normalize(10.0), 0.0);
        assert_eq!(r.normalize(20.0), 0.5);
        assert_eq!(r.normalize(30.0), 1.0);
        assert_eq!(r.denormalize(0.5), 20.0);
    }
}
