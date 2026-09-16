use super::theme::base;
use crate::Ui;
use ui_kit_core::{Color, Rect, Style, TextAlign};

/// Text in a rect: body text by default, or muted, a heading, a caption.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label {
    size: f32,
    weight: u16,
    color: Option<Color>,
    muted: bool,
    align: TextAlign,
}

impl Default for Label {
    fn default() -> Self {
        Self {
            size: 14.0,
            weight: 400,
            color: None,
            muted: false,
            align: TextAlign::Left,
        }
    }
}

impl Label {
    pub fn new() -> Self {
        Self::default()
    }

    /// Secondary text in the palette's muted colour.
    pub fn muted() -> Self {
        Self {
            muted: true,
            ..Self::default()
        }
    }

    /// A section or card title: larger and semibold.
    pub fn heading(size: f32) -> Self {
        Self {
            size,
            weight: 600,
            ..Self::default()
        }
    }

    /// The muted heading over a group of controls.
    pub fn subheading() -> Self {
        Self {
            weight: 500,
            muted: true,
            ..Self::default()
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn centered(self) -> Self {
        self.align(TextAlign::Center)
    }

    pub fn right(self) -> Self {
        self.align(TextAlign::Right)
    }

    /// The style the text is drawn in.
    pub fn style(&self, ui: &Ui) -> Style {
        let p = ui.theme.palette;
        Style {
            foreground: self
                .color
                .unwrap_or(if self.muted { p.muted } else { p.text }),
            font_size: self.size,
            font_weight: self.weight,
            text_padding: 0.0,
            text_align: self.align,
            ..base(ui)
        }
    }

    /// How wide `text` is in this label's style.
    pub fn width(&self, ui: &Ui, text: &str) -> f32 {
        ui.text_width(text, &self.style(ui))
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect, text: &str) {
        let style = self.style(ui);
        ui.with_style(style, |ui| ui.label(rect, text));
    }
}
