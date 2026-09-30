//! Type: Prism Sans, set tight and heavy for headings, with wide-spaced
//! capitals over them, and white at a few strengths for what is less
//! important.

use ui_kit_core::{Color, Material, Rect, Style};
use ui_kit_immediate::{FontFamily, TextAlign, TextOverflow, Ui};

use crate::color::{faded, white, WHITE};

/// Prism Sans: Inter as Prism ships it, the face the brand sets its name in.
/// Load it into the renderer's fonts and prefer it with [`FAMILIES`].
pub const FONT: &[u8] = include_bytes!(
    "../../../../../studio/prism/products/prism/core/app/assets/fonts/PrismSans.ttf"
);

/// The families sans-serif text is set in, in order of preference.
pub const FAMILIES: [&str; 2] = ["Prism Sans", "Inter"];

/// How a run of text is set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    pub size: f32,
    pub weight: u16,
    /// Extra room between letters, in ems.
    pub spacing: f32,
    pub color: Color,
    pub align: TextAlign,
    /// Line height as a multiple of the size, for text that wraps.
    pub leading: f32,
}

impl Type {
    pub const fn new(size: f32, weight: u16) -> Self {
        Self {
            size,
            weight,
            spacing: 0.0,
            color: WHITE,
            align: TextAlign::Left,
            leading: 1.25,
        }
    }

    pub const fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub const fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub const fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub const fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub const fn leading(mut self, leading: f32) -> Self {
        self.leading = leading;
        self
    }

    /// Fade the colour by `alpha`, 0 to 1.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.color = faded(self.color, alpha);
        self
    }

    pub fn line_height(&self) -> f32 {
        (self.size * self.leading).round()
    }

    /// The style this type is drawn in, over `base`.
    pub fn style(&self, base: &Style) -> Style {
        Style {
            fill: [0.0; 4],
            gradient_end: None,
            stroke: [0.0; 4],
            stroke_width: 0.0,
            material: Material::Solid,
            foreground: self.color,
            font_size: self.size,
            font_weight: self.weight,
            text_padding: 0.0,
            text_align: self.align,
            overflow: TextOverflow::Clip,
            font_family: FontFamily::SansSerif,
            italic: false,
            letter_spacing: self.spacing,
            line_height: None,
            corner_radii: None,
            ..base.clone()
        }
    }
}

/// The scale type is set at. Headings come in a wide size and a `narrow`
/// one, for a phone.
pub mod scale {
    use super::*;

    /// A name set as big as the screen allows: a product on its own page.
    pub const fn display(narrow: bool) -> Type {
        Type::new(if narrow { 60.0 } else { 88.0 }, 800)
            .spacing(-0.035)
            .leading(1.0)
    }

    /// A section's heading.
    pub const fn heading(narrow: bool) -> Type {
        Type::new(if narrow { 32.0 } else { 52.0 }, 700)
            .spacing(-0.025)
            .leading(1.08)
    }

    /// The heading of a pane of its own, centred.
    pub const fn pane_heading(narrow: bool) -> Type {
        Type::new(if narrow { 30.0 } else { 46.0 }, 700)
            .spacing(-0.025)
            .align(TextAlign::Center)
            .leading(1.1)
    }

    /// A name at the head of a card.
    pub const fn title(narrow: bool) -> Type {
        Type::new(if narrow { 34.0 } else { 44.0 }, 700).spacing(-0.03)
    }

    /// The line under a hero, centred.
    pub const fn lead(narrow: bool) -> Type {
        Type::new(if narrow { 17.0 } else { 21.0 }, 400)
            .color(white(0.72))
            .align(TextAlign::Center)
            .leading(1.45)
    }

    /// The words under a pane's heading, centred.
    pub const SUB: Type = Type::new(17.0, 400)
        .color(white(0.64))
        .align(TextAlign::Center)
        .leading(1.5);

    /// Running text.
    pub const BODY: Type = Type::new(15.0, 400).color(white(0.6)).leading(1.55);

    /// The label of a button.
    pub const BUTTON: Type = Type::new(15.0, 600).spacing(-0.005);

    /// A link in a bar.
    pub const LINK: Type = Type::new(15.0, 500).color(white(0.72));

    /// Wide-spaced capitals over a heading.
    pub const EYEBROW: Type = Type::new(12.0, 600).spacing(0.24).color(white(0.55));

    /// A caption on a control.
    pub const CAPTION: Type = Type::new(13.0, 600).spacing(0.02).color(white(0.86));

    /// Small print: a footer's lines.
    pub const SMALL: Type = Type::new(13.0, 400).color(white(0.42));
}

/// One line of `text` in `rect`, centred on it vertically.
pub fn text(ui: &mut Ui, rect: Rect, text: &str, kind: Type) {
    if kind.color[3] <= 0.002 || text.is_empty() {
        return;
    }
    let style = kind.style(&ui.style);
    ui.with_style(style, |ui| ui.label(rect, text));
}

pub fn text_width(ui: &Ui, text: &str, kind: Type) -> f32 {
    let style = kind.style(&ui.style);
    ui.text_width(text, &style)
}

/// `text` broken into lines no wider than `width`, only between words: a
/// hyphenated word such as "real-time" stays whole.
pub fn wrap(ui: &Ui, text: &str, kind: Type, width: f32) -> Vec<String> {
    let style = kind.style(&ui.style);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && ui.text_width(&candidate, &style) > width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Lines of `kind`, one under the other from `at`, the way [`wrap`] broke
/// them; returns where they end.
pub fn paragraph(ui: &mut Ui, at: Rect, lines: &[String], kind: Type) -> f32 {
    let lh = kind.line_height();
    for (i, line) in lines.iter().enumerate() {
        text(
            ui,
            Rect::new(at.x, at.y + i as f32 * lh, at.width, lh),
            line,
            kind,
        );
    }
    at.y + lines.len() as f32 * lh
}
