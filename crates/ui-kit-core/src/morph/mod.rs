//! One shape that morphs through a run of UI states, described without a
//! renderer.
//!
//! A [`Scene`] is what a moment looks like: a stack of [`Layer`]s of simple
//! [`Item`]s -- rounded rectangles, one line of text, stroked and filled
//! paths -- in the coordinates of a square stage. The immediate layer turns
//! it into draw commands and the Dioxus host into SVG, so both draw the same
//! frame from the same numbers.
//!
//! [`Reel`] is the choreography: a button that becomes a loader, a check, a
//! music player, a slider, a toggle, tabs, a chart, a command palette and a
//! toast, on a beat grid, driven by a cursor. [`Reel::seek`] is a pure
//! function of time, so it can be scrubbed, rendered out of order and looped.

mod reel;

pub use reel::{Cue, Reel, ReelState, Sound, REEL_BEATS, STAGE};

use crate::{Color, Point, Rect, TextAlign};
use serde::{Deserialize, Serialize};

/// How a path's line is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

/// One thing a scene draws.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    /// A soft shadow under a rounded rectangle.
    Shadow {
        rect: Rect,
        radius: f32,
        blur: f32,
        color: Color,
    },
    /// A rounded rectangle; a circle is one as round as it is small.
    Rect {
        rect: Rect,
        radius: f32,
        fill: Color,
        stroke: Option<Stroke>,
    },
    /// One line of text, its middle on `at.y`, starting at, centred on or
    /// ending at `at.x`.
    Text {
        at: Point,
        text: String,
        size: f32,
        weight: u16,
        color: Color,
        align: TextAlign,
        /// A text caret after the last character, in this colour.
        caret: Option<Color>,
    },
    /// Connected lines with round ends and joins.
    Path {
        points: Vec<Point>,
        stroke: Stroke,
        closed: bool,
    },
    /// A filled outline, which may be concave.
    Polygon {
        points: Vec<Point>,
        fill: Color,
        stroke: Option<Stroke>,
    },
}

impl Item {
    /// The item moved and scaled: `p * scale + offset`.
    pub fn transformed(&self, scale: f32, offset: Point) -> Item {
        let point = |p: &Point| Point::new(p.x * scale + offset.x, p.y * scale + offset.y);
        let rect = |r: &Rect| {
            Rect::new(
                r.x * scale + offset.x,
                r.y * scale + offset.y,
                r.width * scale,
                r.height * scale,
            )
        };
        let stroke = |s: &Stroke| Stroke {
            width: s.width * scale,
            color: s.color,
        };
        match self {
            Item::Shadow {
                rect: r,
                radius,
                blur,
                color,
            } => Item::Shadow {
                rect: rect(r),
                radius: radius * scale,
                blur: blur * scale,
                color: *color,
            },
            Item::Rect {
                rect: r,
                radius,
                fill,
                stroke: s,
            } => Item::Rect {
                rect: rect(r),
                radius: radius * scale,
                fill: *fill,
                stroke: s.as_ref().map(stroke),
            },
            Item::Text {
                at,
                text,
                size,
                weight,
                color,
                align,
                caret,
            } => Item::Text {
                at: point(at),
                text: text.clone(),
                size: size * scale,
                weight: *weight,
                color: *color,
                align: *align,
                caret: *caret,
            },
            Item::Path {
                points,
                stroke: s,
                closed,
            } => Item::Path {
                points: points.iter().map(point).collect(),
                stroke: stroke(s),
                closed: *closed,
            },
            Item::Polygon {
                points,
                fill,
                stroke: s,
            } => Item::Polygon {
                points: points.iter().map(point).collect(),
                fill: *fill,
                stroke: s.as_ref().map(stroke),
            },
        }
    }
}

/// Items drawn together, faded and blurred as one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    /// Multiplies every item's opacity.
    pub opacity: f32,
    /// Gaussian blur radius, in the same units as the items.
    pub blur: f32,
    pub items: Vec<Item>,
}

impl Layer {
    pub fn new(opacity: f32, blur: f32) -> Self {
        Self {
            opacity,
            blur,
            items: vec![],
        }
    }

    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }
}

/// What the stage looks like at one moment.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    /// The stage is a square this many units across.
    pub size: f32,
    pub background: Color,
    /// Bottom first.
    pub layers: Vec<Layer>,
}

impl Scene {
    /// The scale and offset that fit the stage in `target`, centred.
    pub fn fit(&self, target: Rect) -> (f32, Point) {
        let scale = (target.width.min(target.height) / self.size).max(0.0);
        let offset = Point::new(
            target.x + (target.width - self.size * scale) * 0.5,
            target.y + (target.height - self.size * scale) * 0.5,
        );
        (scale, offset)
    }

    /// The scene moved and scaled so the stage fills `target`.
    pub fn placed(&self, target: Rect) -> Scene {
        let (scale, offset) = self.fit(target);
        Scene {
            size: self.size * scale,
            background: self.background,
            layers: self
                .layers
                .iter()
                .map(|layer| Layer {
                    opacity: layer.opacity,
                    blur: layer.blur * scale,
                    items: layer
                        .items
                        .iter()
                        .map(|item| item.transformed(scale, offset))
                        .collect(),
                })
                .collect(),
        }
    }
}

/// `a` to `b` by `t`, per channel.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

/// `color` with its opacity multiplied by `alpha`.
pub fn fade(color: Color, alpha: f32) -> Color {
    [color[0], color[1], color[2], color[3] * alpha]
}

/// A colour from `0xRRGGBB`.
pub const fn hex(value: u32) -> Color {
    [
        ((value >> 16) & 255) as f32 / 255.0,
        ((value >> 8) & 255) as f32 / 255.0,
        (value & 255) as f32 / 255.0,
        1.0,
    ]
}

/// The points of a circular arc, clockwise from `start` radians through
/// `sweep` radians.
pub fn arc(center: Point, radius: f32, start: f32, sweep: f32) -> Vec<Point> {
    let steps = ((sweep.abs() * radius.max(1.0)).sqrt() * 4.0)
        .ceil()
        .clamp(2.0, 96.0) as usize;
    (0..=steps)
        .map(|i| {
            let a = start + sweep * i as f32 / steps as f32;
            Point::new(center.x + a.cos() * radius, center.y + a.sin() * radius)
        })
        .collect()
}

/// The first `fraction` of a path's length.
pub fn trim(points: &[Point], fraction: f32) -> Vec<Point> {
    if fraction >= 1.0 || points.len() < 2 {
        return points.to_vec();
    }
    if fraction <= 0.0 {
        return vec![];
    }
    let length = |a: &Point, b: &Point| (b.x - a.x).hypot(b.y - a.y);
    let total: f32 = points.windows(2).map(|w| length(&w[0], &w[1])).sum();
    let mut left = total * fraction;
    let mut out = vec![points[0]];
    for w in points.windows(2) {
        let l = length(&w[0], &w[1]);
        if l >= left {
            let t = if l > 0.0 { left / l } else { 0.0 };
            out.push(Point::new(
                w[0].x + (w[1].x - w[0].x) * t,
                w[0].y + (w[1].y - w[0].y) * t,
            ));
            return out;
        }
        left -= l;
        out.push(w[1]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placing_a_scene_scales_everything_about_the_target() {
        let scene = Scene {
            size: 100.0,
            background: [1.0; 4],
            layers: vec![Layer {
                opacity: 1.0,
                blur: 4.0,
                items: vec![Item::Rect {
                    rect: Rect::new(10.0, 10.0, 20.0, 20.0),
                    radius: 5.0,
                    fill: [0.0; 4],
                    stroke: None,
                }],
            }],
        };
        let placed = scene.placed(Rect::new(0.0, 50.0, 200.0, 300.0));
        assert_eq!(placed.size, 200.0);
        assert_eq!(placed.layers[0].blur, 8.0);
        assert_eq!(
            placed.layers[0].items[0],
            Item::Rect {
                rect: Rect::new(20.0, 120.0, 40.0, 40.0),
                radius: 10.0,
                fill: [0.0; 4],
                stroke: None
            }
        );
    }

    #[test]
    fn trimming_a_path_keeps_the_first_part_of_its_length() {
        let path = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ];
        assert_eq!(
            trim(&path, 0.25),
            vec![Point::new(0.0, 0.0), Point::new(5.0, 0.0)]
        );
        assert_eq!(trim(&path, 0.75).last(), Some(&Point::new(10.0, 5.0)));
        assert!(trim(&path, 0.0).is_empty());
    }
}
