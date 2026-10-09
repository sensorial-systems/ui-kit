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
        }
    }

    /// The same, with a title bar for the application to draw.
    pub fn with_title_bar(mut self, title_bar: Option<TitleBar>) -> Self {
        self.title_bar = title_bar;
        self
    }

}
