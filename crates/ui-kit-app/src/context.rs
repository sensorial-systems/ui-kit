use ui_kit_core::Rect;
use ui_kit_immediate::Input;

/// The title bar an application draws itself
/// ([`AppConfig::with_custom_title_bar`](crate::AppConfig::with_custom_title_bar)),
/// in the application's coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TitleBar {
    /// The whole bar, across the top of the window.
    pub rect: Rect,
    /// The part of it the window's buttons take, at its trailing end: lay the
    /// bar out to stop short of it.
    pub buttons: Rect,
    pub maximized: bool,
    pub fullscreen: bool,
}

/// The device's safe-area insets, in logical points: the space along each
/// edge the notch/Dynamic Island, the home indicator or the rounded corners
/// take. Zero where there are none (desktop, and phones without them). An
/// application keeps its controls clear of these so nothing hides behind the
/// hardware.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SafeArea {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Environmental information provided to the application every frame.
#[derive(Clone, Debug)]
pub struct FrameContext {
    pub input: Input,
    pub width: f32,
    pub height: f32,
    pub elapsed: f32,
    pub scale: f32,
    pub dark: bool,
    /// The title bar the application draws, when it draws one.
    pub title_bar: Option<TitleBar>,
    /// The device's safe-area insets, in logical points.
    pub safe_area: SafeArea,
}

impl FrameContext {
    pub fn new(
        input: Input,
        width: f32,
        height: f32,
        elapsed: f32,
        scale: f32,
        dark: bool,
    ) -> Self {
        Self {
            input,
            width,
            height,
            elapsed,
            scale,
            dark,
            title_bar: None,
            safe_area: SafeArea::default(),
        }
    }

    /// The same, with a title bar for the application to draw.
    pub fn with_title_bar(mut self, title_bar: Option<TitleBar>) -> Self {
        self.title_bar = title_bar;
        self
    }

    /// The same, with the device's safe-area insets.
    pub fn with_safe_area(mut self, safe_area: SafeArea) -> Self {
        self.safe_area = safe_area;
        self
    }
}
