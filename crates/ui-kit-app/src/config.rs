use ui_kit_core::Color;

/// Configuration for an application window and its display settings.
#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub transparent: bool,
    pub resizable: bool,
    pub dark: bool,
    pub background: Color,
    pub fps: u32,
    /// Whether the window is shown when the application starts. An
    /// application that starts in the system tray sets this to false and
    /// shows the window with [`WindowCommand::Show`](crate::WindowCommand::Show).
    pub visible: bool,
    /// Draw ui-kit's own title bar and window controls around the content.
    /// Off on Android, where the application owns the whole screen.
    pub chrome: bool,
    /// The application draws the title bar itself, this tall: its content
    /// runs to the top of the window, under the bar, and only the window's
    /// buttons are painted over it (see [`with_custom_title_bar`](Self::with_custom_title_bar)).
    pub custom_title_bar: Option<f32>,
    /// Put a button on the bar that takes the whole screen and gives it back,
    /// before the others.
    pub fullscreen_button: bool,
    /// Take the whole screen: on Android, hide the status and navigation bars
    /// (a swipe from the edge shows them for a moment); on desktop, a
    /// borderless fullscreen window.
    pub fullscreen: bool,
    /// In a browser, the id of the `<canvas>` to draw into. Without one, a
    /// canvas filling the page is added to its body. Unused natively.
    pub canvas: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "UI Kit Application".into(),
            width: 1200.0,
            height: 850.0,
            min_width: Some(320.0),
            min_height: Some(240.0),
            // A page shows through a transparent canvas only where the
            // browser composites it that way; draw the background instead.
            transparent: !cfg!(target_arch = "wasm32"),
            resizable: true,
            dark: true,
            background: [0.07, 0.07, 0.08, 1.0],
            fps: 60,
            visible: true,
            // Android owns the screen, and a page is already inside the
            // browser's window.
            chrome: !cfg!(any(target_os = "android", target_arch = "wasm32")),
            custom_title_bar: None,
            fullscreen_button: false,
            fullscreen: false,
            canvas: None,
        }
    }
}

impl AppConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_min_size(mut self, width: f32, height: f32) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn with_dark(mut self, dark: bool) -> Self {
        self.dark = dark;
        if dark {
            self.background = [0.07, 0.07, 0.08, 1.0];
        } else {
            self.background = [1.0, 1.0, 1.0, 1.0];
        }
        self
    }

    pub fn with_background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps.clamp(1, 240);
        self
    }

    /// Draw ui-kit's title bar and window controls, or leave the whole
    /// window to the application.
    pub fn with_chrome(mut self, chrome: bool) -> Self {
        self.chrome = chrome;
        self
    }

    /// Draw the title bar in the application, `height` tall, and leave the
    /// window only its buttons, over the bar's trailing end -- as a store, a
    /// browser or an editor with tabs in its bar does.
    ///
    /// Each frame's [`FrameContext::title_bar`](crate::FrameContext::title_bar)
    /// says where the bar is and where the buttons are; the application lays
    /// its bar out around them and says, through
    /// [`App::title_bar`](crate::App::title_bar), where its own controls on it
    /// are, so the rest of the bar still moves the window.
    pub fn with_custom_title_bar(mut self, height: f32) -> Self {
        self.custom_title_bar = Some(height);
        self
    }

    /// Put a full-screen button on the bar (see [`AppConfig::fullscreen_button`]).
    pub fn with_fullscreen_button(mut self, shown: bool) -> Self {
        self.fullscreen_button = shown;
        self
    }

    /// Take the whole screen (see [`AppConfig::fullscreen`]).

    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// Start with the window hidden or shown.
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// In a browser, draw into the `<canvas>` with this id.
    pub fn with_canvas(mut self, id: impl Into<String>) -> Self {
        self.canvas = Some(id.into());
        self
    }
}
