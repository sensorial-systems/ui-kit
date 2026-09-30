//! Glass: what is behind a panel, frosted, bent at its rounded rim as
//! through a thick lens and split into colours there, with a rim of light
//! round the edge and a highlight where the pointer is.
//!
//! A [`GlassLook`] travels as the [`GLASS`] material of a command;
//! [`SensorialRenderer`](crate::SensorialRenderer) draws it from what was
//! drawn before it, so glass bends whatever is under it.

use ui_kit_core::{Material, Rect};
use ui_kit_immediate::{Response, Ui};

/// The material a glass surface is painted in; its parameters are a
/// [`GlassLook`].
pub const GLASS: &str = "sensorial.glass";
/// The command kind of a bare pane of glass, with nothing on it.
pub const GLASS_PANEL: &str = "sensorial.glass-panel";

/// Where [`GlassLook::highlight`] follows the pointer, wherever it is on
/// the screen, rather than sitting at a place on the panel.
pub const AT_POINTER: [f32; 2] = [f32::NAN, f32::NAN];

/// How a glass panel looks. Lengths are in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassLook {
    /// How wide the rounded rim is, that bends light.
    pub bevel: f32,
    /// How far the rim bends light.
    pub refraction: f32,
    /// How much further red bends than blue: 0 none, 0.1 a lot.
    pub dispersion: f32,
    /// How far what is behind is blurred.
    pub blur: f32,
    /// The tint and how much of the view it covers.
    pub tint: [f32; 3],
    pub tint_amount: f32,
    /// How bright the rim of light round the edge is.
    pub rim: f32,
    /// How much of the rim is split into a spectrum.
    pub spectral: f32,
    /// How opaque the whole panel is, for fading it in and out.
    pub opacity: f32,
    /// Where the highlight is, 0 to 1 across and down the panel (outside
    /// it for light from beyond the edge), or [`AT_POINTER`]; how bright
    /// and how wide.
    pub highlight: [f32; 2],
    pub highlight_strength: f32,
    pub highlight_radius: f32,
    /// How much the face magnifies what is behind.
    pub lens: f32,
    /// How much brighter than what is behind the panel is.
    pub brightness: f32,
}

impl Default for GlassLook {
    fn default() -> Self {
        Self {
            bevel: 16.0,
            refraction: 14.0,
            dispersion: 0.06,
            blur: 22.0,
            tint: [1.0, 1.0, 1.0],
            tint_amount: 0.035,
            rim: 0.32,
            spectral: 0.22,
            opacity: 1.0,
            highlight: [0.5, 0.0],
            highlight_strength: 0.0,
            highlight_radius: 220.0,
            lens: 0.012,
            brightness: 1.08,
        }
    }
}

impl GlassLook {
    const LEN: usize = 18;

    /// A control: a pill of glass a button is made of, `hover` 0 to 1.
    pub fn control(hover: f32) -> Self {
        Self {
            bevel: 12.0,
            refraction: 9.0 + 6.0 * hover,
            dispersion: 0.08,
            blur: 16.0,
            tint_amount: 0.06 + 0.06 * hover,
            rim: 0.36 + 0.2 * hover,
            spectral: 0.3 + 0.35 * hover,
            highlight: [0.5, -0.4],
            highlight_strength: 0.03 + 0.1 * hover,
            highlight_radius: 70.0,
            lens: 0.0,
            brightness: 1.12 + 0.06 * hover,
            ..Self::default()
        }
    }

    /// A card, lit and bent further under the pointer, `hover` 0 to 1.
    pub fn card(hover: f32) -> Self {
        Self {
            bevel: 22.0,
            refraction: 18.0 + 14.0 * hover,
            dispersion: 0.07 + 0.05 * hover,
            blur: 26.0,
            tint_amount: 0.035 + 0.02 * hover,
            rim: 0.28 + 0.16 * hover,
            spectral: 0.2 + 0.35 * hover,
            highlight: [0.2, -0.3],
            highlight_strength: 0.035 + 0.075 * hover,
            highlight_radius: 200.0,
            lens: 0.01,
            brightness: 1.06 + 0.05 * hover,
            ..Self::default()
        }
    }

    /// A big card, one that leads: a wider rim and highlight.
    pub fn strong(mut self) -> Self {
        self.bevel += 4.0;
        self.highlight_radius += 60.0;
        self
    }

    /// Bent and split further while the page moves at `speed`, 0 still to
    /// 1 fast.
    pub fn moving(mut self, speed: f32) -> Self {
        let speed = speed.clamp(0.0, 1.0);
        self.refraction += 12.0 * speed;
        self.dispersion += 0.08 * speed;
        self
    }

    /// A pane that holds a whole section, `hover` 0 to 1.
    pub fn pane(hover: f32) -> Self {
        Self {
            bevel: 34.0,
            refraction: 26.0,
            dispersion: 0.1,
            blur: 30.0,
            tint_amount: 0.03,
            rim: 0.26,
            spectral: 0.3 + 0.2 * hover,
            highlight: [0.5, -0.2],
            highlight_strength: 0.03 + 0.05 * hover,
            highlight_radius: 360.0,
            lens: 0.008,
            brightness: 1.05,
            ..Self::default()
        }
    }

    /// A bar floating over the page, such as the navigation: darker and
    /// more frosted once the page has `settled` under it, 0 to 1.
    pub fn bar(settled: f32) -> Self {
        Self {
            bevel: 20.0,
            refraction: 16.0,
            dispersion: 0.09,
            blur: 22.0 + 8.0 * settled,
            tint_amount: 0.03 + 0.05 * settled,
            tint: [0.06, 0.065, 0.08],
            rim: 0.3,
            spectral: 0.28,
            highlight: [0.3, -1.0],
            highlight_strength: 0.05,
            highlight_radius: 160.0,
            lens: 0.0,
            brightness: 1.1,
            ..Self::default()
        }
    }

    /// A small chip of thick glass, `hover` 0 to 1: a toggle floating on
    /// its own.
    pub fn chip(hover: f32) -> Self {
        Self {
            bevel: 14.0,
            refraction: 10.0,
            dispersion: 0.12,
            blur: 12.0,
            tint_amount: 0.06 + 0.06 * hover,
            rim: 0.4 + 0.2 * hover,
            spectral: 0.45,
            highlight: [0.5, -0.5],
            highlight_strength: 0.04,
            lens: 0.0,
            brightness: 1.2,
            ..Self::default()
        }
    }

    /// A menu, a dropdown or a dialog: dark enough to read over whatever
    /// it covers.
    pub fn popup() -> Self {
        Self {
            bevel: 14.0,
            refraction: 10.0,
            dispersion: 0.08,
            blur: 30.0,
            tint: [0.05, 0.055, 0.07],
            tint_amount: 0.62,
            rim: 0.3,
            spectral: 0.25,
            highlight: AT_POINTER,
            highlight_strength: 0.04,
            highlight_radius: 160.0,
            lens: 0.0,
            brightness: 1.0,
            ..Self::default()
        }
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// The highlight at `at` (0 to 1 over the panel) when there is a
    /// pointer over it, and where the look puts it when there is not.
    pub fn highlight_at(mut self, at: Option<[f32; 2]>) -> Self {
        if let Some(at) = at {
            self.highlight = at;
        }
        self
    }

    fn encode(&self) -> Vec<f32> {
        vec![
            self.bevel,
            self.refraction,
            self.dispersion,
            self.blur,
            self.tint[0],
            self.tint[1],
            self.tint[2],
            self.tint_amount,
            self.rim,
            self.spectral,
            self.opacity,
            self.highlight[0],
            self.highlight[1],
            self.highlight_strength,
            self.highlight_radius,
            self.lens,
            self.brightness,
            0.0,
        ]
    }

    pub fn decode(p: &[f32]) -> Self {
        let mut v = [0.0; Self::LEN];
        for (slot, value) in v.iter_mut().zip(p) {
            *slot = *value;
        }
        Self {
            bevel: v[0],
            refraction: v[1],
            dispersion: v[2],
            blur: v[3],
            tint: [v[4], v[5], v[6]],
            tint_amount: v[7],
            rim: v[8],
            spectral: v[9],
            opacity: v[10],
            highlight: [v[11], v[12]],
            highlight_strength: v[13],
            highlight_radius: v[14],
            lens: v[15],
            brightness: v[16],
        }
    }

    /// This look as a material, for a [`Style`](ui_kit_core::Style).
    pub fn material(&self) -> Material {
        Material::Custom {
            name: GLASS.into(),
            parameters: self.encode(),
        }
    }

    /// The look `material` is, if it is glass.
    pub fn of(material: &Material) -> Option<Self> {
        match material {
            Material::Custom { name, parameters } if name == GLASS => {
                Some(Self::decode(parameters))
            }
            _ => None,
        }
    }

    /// Paint a pane of this glass over `rect`, with corners of `radius`.
    pub fn paint(&self, ui: &mut Ui, rect: Rect, radius: f32) {
        if self.opacity <= 0.001 || rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        let mut style = ui.style.clone();
        style.radius = radius;
        style.corner_radii = None;
        style.material = self.material();
        ui.with_style(style, |ui| {
            ui.paint(GLASS_PANEL, "", rect, 0.0, Response::default(), false)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_survives_its_material() {
        let look = GlassLook::card(0.4).moving(0.5).opacity(0.7);
        assert_eq!(GlassLook::of(&look.material()), Some(look));
    }

    #[test]
    fn following_the_pointer_survives_too() {
        let look = GlassLook::popup();
        let back = GlassLook::of(&look.material()).unwrap();
        assert!(back.highlight[0].is_nan() && back.highlight[1].is_nan());
    }
}
