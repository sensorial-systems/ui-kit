//! Reusable custom window chrome, with optional rendering and native adapters.
mod chrome;
pub use chrome::{ResizeEdge, WindowAction, WindowChrome, WindowControl, WindowRegion};
#[cfg(feature = "immediate")]
pub mod immediate;
#[cfg(feature = "immediate")]
pub use immediate::{window_chrome, WindowResponse, WindowStyle};
#[cfg(feature = "dioxus")]
mod dioxus;
#[cfg(feature = "dioxus")]
pub use dioxus::WindowChromeView;
#[cfg(feature = "winit")]
mod native;
#[cfg(feature = "winit")]
pub use native::{ChromeEvent, WinitWindow};
