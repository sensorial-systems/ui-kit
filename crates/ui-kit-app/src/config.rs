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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "UI Kit Application".into(),
            width: 1200.0,
            height: 850.0,
            min_width: Some(320.0),
            min_height: Some(240.0),
            transparent: true,
            resizable: true,
            dark: true,
            background: [0.07, 0.07, 0.08, 1.0],
            fps: 60,
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
}
