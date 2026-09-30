//! The colours the components draw with.
//!
//! Components read the [`Palette`] of the [`Ui`]'s theme, so an application
//! restyles all of them by setting `ui.theme`. [`theme`] builds the kit's own
//! light and dark looks; status colours ([`SUCCESS`], [`WARNING`], ...) are the
//! same in both.
use crate::{Palette, Theme, Ui};
use ui_kit_core::{Color, Style, TextAlign};

pub const INFO: Color = [0.231, 0.510, 0.965, 1.0];
pub const SUCCESS: Color = [0.063, 0.725, 0.506, 1.0];
pub const WARNING: Color = [0.961, 0.620, 0.043, 1.0];
pub const DANGER: Color = [0.937, 0.267, 0.267, 1.0];

/// A colour from `0xRRGGBB`.
pub const fn rgb(value: u32) -> Color {
    [
        ((value >> 16) & 255) as f32 / 255.0,
        ((value >> 8) & 255) as f32 / 255.0,
        (value & 255) as f32 / 255.0,
        1.0,
    ]
}

/// `color` with its opacity replaced.
pub const fn with_alpha(color: Color, alpha: f32) -> Color {
    [color[0], color[1], color[2], alpha]
}

/// What reads on `color`: white on a dark or saturated fill, near-black on
/// a light one -- the label of a button filled in the accent, say.
pub fn on(color: Color) -> Color {
    let [r, g, b, _] = color;
    if 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.6 {
        [0.02, 0.02, 0.03, 1.0]
    } else {
        [1.0; 4]
    }
}

/// `color` with its opacity scaled, as a disabled control draws.
pub fn faded(color: Color, factor: f32) -> Color {
    [color[0], color[1], color[2], color[3] * factor]
}

/// The kit's palette: a white or near-black page with blue accents.
pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            background: rgb(0x121214),
            surface: rgb(0x1a1a1e),
            raised: rgb(0x27272a),
            border: rgb(0x2e2e33),
            text: rgb(0xf4f4f5),
            muted: rgb(0x8b8b93),
            accent: INFO,
            danger: DANGER,
            scrim: [0.0, 0.0, 0.0, 0.45],
            shadow: [0.0, 0.0, 0.0, 0.25],
        }
    } else {
        Palette {
            background: rgb(0xffffff),
            surface: rgb(0xffffff),
            raised: rgb(0xf4f4f5),
            border: rgb(0xe4e4e7),
            text: rgb(0x09090b),
            muted: rgb(0x71717a),
            accent: INFO,
            danger: DANGER,
            scrim: [0.0, 0.0, 0.0, 0.45],
            shadow: [0.06, 0.09, 0.18, 0.06],
        }
    }
}

/// The kit's theme, light or dark.
pub fn theme(dark: bool) -> Theme {
    Theme::from_palette(palette(dark))
}

/// Whether `palette` is a dark one, judged by its background.
pub fn is_dark(palette: &Palette) -> bool {
    let [r, g, b, _] = palette.background;
    0.2126 * r + 0.7152 * g + 0.0722 * b < 0.5
}

/// The style every component starts from: a bordered surface with body text.
pub fn base(ui: &Ui) -> Style {
    let p = ui.theme.palette;
    Style {
        fill: p.surface,
        stroke: p.border,
        foreground: p.text,
        accent: p.accent,
        radius: 8.0,
        font_size: 14.0,
        font_weight: 400,
        text_padding: 12.0,
        text_align: TextAlign::Left,
        ..Style::default()
    }
}
