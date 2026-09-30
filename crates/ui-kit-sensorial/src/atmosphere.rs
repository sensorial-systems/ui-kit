//! The world behind everything: black, as the brand's intro is, lit by
//! light split as through a prism -- a fan of faint spectral rays from
//! above, pools of coloured light drifting far away, a haze where the
//! subject of the screen hangs, and dust at three depths that streaks and
//! splits into its colours when the page moves fast.
//!
//! It is alive: it drifts with time, rises as the page scrolls, leans
//! towards the pointer and flashes on a press. The application says how
//! with an [`Atmosphere`], written every frame and read by
//! [`SensorialRenderer`](crate::SensorialRenderer), and paints it with
//! [`paint_backdrop`] before anything else, so the glass over it has
//! something to bend.

use std::cell::RefCell;
use std::rc::Rc;

use ui_kit_core::{Point, Rect};
use ui_kit_immediate::{Response, Ui};

use crate::motion::approach;

/// The command kind the world is painted as.
pub const BACKDROP: &str = "sensorial.backdrop";

/// What the world looks like this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atmosphere {
    /// Seconds, for everything that drifts.
    pub time: f32,
    /// How far the page has scrolled, and how fast, in CSS pixels.
    pub scroll: f32,
    pub velocity: f32,
    /// How much of the world is lit: 0 is all black.
    pub lit: f32,
    /// Where the world leans to, 0 to 1 across and down the screen: the
    /// pointer, eased.
    pub pointer: [f32; 2],
    /// Where the pointer is, in CSS pixels, while there is one that
    /// hovers: glass that follows the pointer lights up there.
    pub cursor: Option<[f32; 2]>,
    /// The hue the light leans to, 0 red to 1 violet: a brand's own.
    pub hue: f32,
    /// A flash of light through the haze, 0 to 1.
    pub flash: f32,
    /// Where the haze is (0 to 1 down the screen) and how wide.
    pub haze: [f32; 2],
    /// Pixels per CSS pixel.
    pub scale: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            time: 0.0,
            scroll: 0.0,
            velocity: 0.0,
            lit: 1.0,
            pointer: [0.5, 0.5],
            cursor: None,
            hue: 0.5,
            flash: 0.0,
            haze: [0.45, 1.2],
            scale: 1.0,
        }
    }
}

impl Atmosphere {
    /// An atmosphere whose light leans to `hue`.
    pub fn new(hue: f32) -> Self {
        Self {
            hue,
            ..Self::default()
        }
    }

    /// Move on to a frame `dt` seconds after the last, on a `viewport` of
    /// CSS pixels at `scale` pixels each, with the pointer at `pointer`
    /// (`None` when there is none, or it is a finger). The world leans
    /// towards the pointer slowly, and a flash fades.
    pub fn advance(&mut self, dt: f32, viewport: [f32; 2], scale: f32, pointer: Option<Point>) {
        self.time += dt;
        self.scale = scale;
        self.cursor = pointer.map(|p| [p.x, p.y]);
        let target = pointer.map_or([0.5, 0.5], |p| {
            [
                (p.x / viewport[0].max(1.0)).clamp(0.0, 1.0),
                (p.y / viewport[1].max(1.0)).clamp(0.0, 1.0),
            ]
        });
        for (value, target) in self.pointer.iter_mut().zip(target) {
            *value = approach(*value, target, 0.2, dt);
        }
        self.flash = approach(self.flash, 0.0, 0.05, dt);
    }

    /// Where the page is: scrolled `scroll` CSS pixels, moving at
    /// `velocity` a second.
    pub fn scrolled(&mut self, scroll: f32, velocity: f32) {
        self.scroll = scroll;
        self.velocity = velocity;
    }

    /// Light up the haze, as on a press.
    pub fn flash(&mut self) {
        self.flash = 1.0;
    }
}

/// An [`Atmosphere`] the application writes and the renderer reads.
pub type SharedAtmosphere = Rc<RefCell<Atmosphere>>;

pub fn shared(atmosphere: Atmosphere) -> SharedAtmosphere {
    Rc::new(RefCell::new(atmosphere))
}

/// Paint the world over `rect`, the whole screen. Paint it first: it
/// covers everything drawn before it.
pub fn paint_backdrop(ui: &mut Ui, rect: Rect) {
    ui.paint(BACKDROP, "", rect, 0.0, Response::default(), false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_world_leans_towards_the_pointer() {
        let mut atmosphere = Atmosphere::default();
        for _ in 0..600 {
            atmosphere.advance(
                1.0 / 60.0,
                [1000.0, 500.0],
                2.0,
                Some(Point::new(1000.0, 0.0)),
            );
        }
        assert!((atmosphere.pointer[0] - 1.0).abs() < 1e-3);
        assert!(atmosphere.pointer[1].abs() < 1e-3);
        assert_eq!(atmosphere.cursor, Some([1000.0, 0.0]));
        assert_eq!(atmosphere.scale, 2.0);
        assert!((atmosphere.time - 10.0).abs() < 1e-3);
    }
}
