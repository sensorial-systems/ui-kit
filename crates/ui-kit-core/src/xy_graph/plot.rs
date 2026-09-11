//! Top-level container for an XY plot specification.

use super::axis::XyAxis;
use super::handle::XyHandle;
use super::point::XyPoint;
use super::range::XyRange;
use super::series::XySeries;
use super::shaded_region::XyShadedRegion;
use crate::{Point, Rect};
use serde::{Deserialize, Serialize};

/// An XY plot definition combining axes, series, shaded regions, and interactive handles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct XyPlot {
    pub x_axis: XyAxis,
    pub y_axis: XyAxis,
    pub series: Vec<XySeries>,
    pub handles: Vec<XyHandle>,
    pub regions: Vec<XyShadedRegion>,
    pub title: Option<String>,
}

impl XyPlot {
    pub fn new(x_axis: XyAxis, y_axis: XyAxis) -> Self {
        Self {
            x_axis,
            y_axis,
            series: Vec::new(),
            handles: Vec::new(),
            regions: Vec::new(),
            title: None,
        }
    }

    pub fn with_series(mut self, series: XySeries) -> Self {
        self.series.push(series);
        self
    }

    pub fn with_handle(mut self, handle: XyHandle) -> Self {
        self.handles.push(handle);
        self
    }

    pub fn with_region(mut self, region: XyShadedRegion) -> Self {
        self.regions.push(region);
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Map a data point `(x, y)` to screen coordinates within `plot_rect`.
    /// Note: screen Y increases downward, so higher data Y maps to lower screen Y.
    pub fn data_to_screen(&self, pt: XyPoint, plot_rect: Rect) -> Point {
        let tx = self.x_axis.range.normalize(pt.x).clamp(0.0, 1.0);
        let ty = self.y_axis.range.normalize(pt.y).clamp(0.0, 1.0);
        Point::new(
            plot_rect.x + tx * plot_rect.width,
            plot_rect.y + (1.0 - ty) * plot_rect.height,
        )
    }

    /// Map a screen coordinate back to data space `(x, y)`.
    pub fn screen_to_data(&self, pt: Point, plot_rect: Rect) -> XyPoint {
        let tx = if plot_rect.width > 0.0 {
            (pt.x - plot_rect.x) / plot_rect.width
        } else {
            0.0
        };
        let ty = if plot_rect.height > 0.0 {
            1.0 - (pt.y - plot_rect.y) / plot_rect.height
        } else {
            0.0
        };
        let x = self.x_axis.range.denormalize(tx);
        let y = self.y_axis.range.denormalize(ty);
        XyPoint::new(x, y)
    }

    /// Find an interactive handle by its ID.
    pub fn handle_mut(&mut self, id: &str) -> Option<&mut XyHandle> {
        self.handles.iter_mut().find(|h| h.id == id)
    }
}

impl Default for XyPlot {
    fn default() -> Self {
        Self::new(
            XyAxis::new(XyRange::new(0.0, 1.0)),
            XyAxis::new(XyRange::new(0.0, 1.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_transforms() {
        let plot = XyPlot::new(
            XyAxis::new(XyRange::new(0.0, 100.0)),
            XyAxis::new(XyRange::new(0.0, 50.0)),
        );
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
        };

        // Origin in data (0, 0) should map to bottom-left of plot rect: (10, 120)
        let screen_origin = plot.data_to_screen(XyPoint::new(0.0, 0.0), rect);
        assert!((screen_origin.x - 10.0).abs() < 1e-4);
        assert!((screen_origin.y - 120.0).abs() < 1e-4);

        // Max in data (100, 50) should map to top-right of plot rect: (210, 20)
        let screen_max = plot.data_to_screen(XyPoint::new(100.0, 50.0), rect);
        assert!((screen_max.x - 210.0).abs() < 1e-4);
        assert!((screen_max.y - 20.0).abs() < 1e-4);

        // Roundtrip from screen back to data
        let roundtrip = plot.screen_to_data(screen_origin, rect);
        assert!((roundtrip.x - 0.0).abs() < 1e-4);
        assert!((roundtrip.y - 0.0).abs() < 1e-4);
    }
}
