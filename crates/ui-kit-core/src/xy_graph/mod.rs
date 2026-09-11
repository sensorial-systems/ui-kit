//! Generic, flexible XY graph data model and coordinate mapping.

pub mod axis;
pub mod handle;
pub mod plot;
pub mod point;
pub mod range;
pub mod series;
pub mod shaded_region;
pub mod tick;

pub use axis::XyAxis;
pub use handle::XyHandle;
pub use plot::XyPlot;
pub use point::XyPoint;
pub use range::XyRange;
pub use series::XySeries;
pub use shaded_region::XyShadedRegion;
pub use tick::XyTick;
