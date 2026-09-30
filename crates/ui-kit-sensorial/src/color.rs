//! The colours: black, white in a few strengths, and the spectrum light
//! splits into -- and the palette and theme built from them, which every
//! component draws with.

use ui_kit_core::{Color, Material, Style};
use ui_kit_immediate::{FontFamily, Palette, TextOverflow, Theme};

use crate::glass::GlassLook;

/// Text, and anything else drawn in light.
pub const WHITE: Color = [0.96, 0.97, 0.98, 1.0];
/// The white of the one solid thing on a screen: a primary pill.
pub const PAPER: Color = [0.97, 0.975, 0.985, 1.0];
/// What is written on [`PAPER`].
pub const INK: Color = [0.02, 0.02, 0.03, 1.0];
/// Where nothing is lit.
pub const BLACK: Color = [0.0, 0.0, 0.0, 1.0];

/// White at `alpha`: how the look draws everything secondary, so it takes
/// on the colour of the light behind it.
pub const fn white(alpha: f32) -> Color {
    [1.0, 1.0, 1.0, alpha]
}

/// `color` with its opacity scaled by `alpha`, 0 to 1: for fading a thing
/// in and out.
pub fn faded(color: Color, alpha: f32) -> Color {
    [
        color[0],
        color[1],
        color[2],
        color[3] * alpha.clamp(0.0, 1.0),
    ]
}

/// A spectrum, 0 red through 1 violet: the colours the light splits into.
pub fn spectrum(x: f32) -> Color {
    let h = x.clamp(0.0, 1.0) * 0.8;
    let channel = |offset: f32| {
        let v = (((h + offset).fract() * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0);
        v * v * (3.0 - 2.0 * v)
    };
    [channel(0.0), channel(2.0 / 3.0), channel(1.0 / 3.0), 1.0]
}

/// The palette. Surfaces are faint white, the strength glass tints to: a
/// fill in [`Palette::surface`] or [`Palette::raised`] is drawn as glass
/// once the frame goes through [`lower`](crate::lower). The accent is
/// [`PAPER`], so a primary control is the one solid white thing, with
/// [`INK`] on it.
pub fn palette() -> Palette {
    Palette {
        background: BLACK,
        surface: white(0.045),
        raised: white(0.075),
        border: white(0.1),
        text: WHITE,
        muted: white(0.56),
        accent: PAPER,
        danger: [1.0, 0.36, 0.33, 1.0],
        scrim: [0.0, 0.0, 0.0, 0.55],
        shadow: [0.0, 0.0, 0.0, 0.35],
    }
}

/// The theme: [`palette`], with glass for the panes, cards, popups, fields
/// and buttons of the built-in widgets, pills for buttons, and type set as
/// the look sets it.
pub fn theme() -> Theme {
    let p = palette();
    let mut theme = Theme::from_palette(p);
    let glass = |style: &mut Style, look: GlassLook, radius: f32| {
        style.material = look.material();
        style.radius = radius;
        style.stroke = [0.0; 4];
        style.stroke_width = 0.0;
    };
    let pill = 999.0;
    for style in [
        &mut theme.text,
        &mut theme.muted,
        &mut theme.heading,
        &mut theme.panel,
        &mut theme.card,
        &mut theme.popup,
        &mut theme.tooltip,
        &mut theme.field,
        &mut theme.row,
        &mut theme.row_selected,
        &mut theme.primary,
        &mut theme.normal,
        &mut theme.ghost,
        &mut theme.selected,
        &mut theme.danger,
    ] {
        style.font_family = FontFamily::SansSerif;
        style.font_size = style.font_size.max(14.0);
    }
    theme.heading.font_size = 17.0;
    theme.heading.font_weight = 700;
    theme.heading.letter_spacing = -0.01;
    theme.muted.font_size = 13.0;
    glass(&mut theme.panel, GlassLook::card(0.0), 28.0);
    glass(&mut theme.card, GlassLook::card(0.0), 24.0);
    glass(&mut theme.popup, GlassLook::popup(), 16.0);
    glass(&mut theme.field, GlassLook::control(0.0), 12.0);
    glass(&mut theme.normal, GlassLook::control(0.0), pill);
    theme.tooltip = Style {
        fill: [0.03, 0.03, 0.04, 0.92],
        stroke: white(0.12),
        radius: 10.0,
        overflow: TextOverflow::Wrap,
        ..theme.tooltip
    };
    theme.row_selected.fill = white(0.1);
    theme.primary = Style {
        fill: PAPER,
        stroke: [0.0; 4],
        stroke_width: 0.0,
        foreground: INK,
        font_weight: 600,
        radius: pill,
        material: Material::Solid,
        ..theme.primary
    };
    theme.ghost.radius = pill;
    theme.selected = Style {
        fill: white(0.14),
        stroke: white(0.3),
        foreground: WHITE,
        radius: pill,
        ..theme.selected
    };
    theme.danger.radius = pill;
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spectrum_runs_red_to_violet() {
        let red = spectrum(0.0);
        assert!(red[0] > 0.99 && red[1] < 0.01 && red[2] < 0.01);
        let violet = spectrum(1.0);
        assert!(violet[2] > 0.99 && violet[1] < 0.01);
    }

    #[test]
    fn the_theme_is_dark_and_made_of_glass() {
        let theme = theme();
        assert!(ui_kit_immediate::components::theme::is_dark(&theme.palette));
        assert!(GlassLook::of(&theme.card.material).is_some());
        assert!(GlassLook::of(&theme.primary.material).is_none());
    }
}
