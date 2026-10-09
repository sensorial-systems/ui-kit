//! The look's own controls, moving by its springs: a pill that is white
//! and solid when it is the one thing to press and glass when it is not, a
//! link underlined in the spectrum, a glass card that lifts and lights up
//! under the pointer, and the wide-spaced capitals over a heading.
//!
//! They keep what moves them in a [`Springs`] the application owns and
//! steps once a frame, and return the [`Response`] for the application to
//! act on.

use ui_kit_core::{Point, Rect};
use ui_kit_immediate::{CursorIcon, Response, Ui};

use crate::color::{faded, INK, PAPER, WHITE};
use crate::glass::GlassLook;
use crate::motion::{Springs, HOVER, PRESS};
use crate::paint::{self, arrow_down, arrow_out, arrow_right, spectral_line};
use crate::typography::{self, scale, Type};

/// Scale `rect` about its centre.
pub fn grow(rect: Rect, by: f32) -> Rect {
    let (w, h) = (rect.width * by, rect.height * by);
    Rect::new(
        rect.x + (rect.width - w) * 0.5,
        rect.y + (rect.height - h) * 0.5,
        w,
        h,
    )
}

/// Where `pointer` is over `rect`, 0 to 1 each way.
pub fn pointer_in(pointer: Option<Point>, rect: Rect) -> Option<[f32; 2]> {
    let p = pointer?;
    Some([
        (p.x - rect.x) / rect.width.max(1.0),
        (p.y - rect.y) / rect.height.max(1.0),
    ])
}

/// How hovered `id` is, 0 to 1, springing as the look's things do; the
/// pointer turns into a hand over it.
pub fn hover(ui: &mut Ui, springs: &mut Springs, id: &str, response: Response) -> f32 {
    if response.hovered {
        ui.set_cursor(CursorIcon::Pointer);
    }
    springs.track(
        &format!("hover.{id}"),
        if response.hovered { 1.0 } else { 0.0 },
        HOVER,
    )
}

/// The pointer, unless it is a finger: glass lights up where a mouse is,
/// not where a finger was.
fn hovering(ui: &Ui) -> Option<Point> {
    let input = ui.input();
    input.pointer.filter(|_| input.touches.is_empty())
}

/// What a pill or link shows beside its label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Glyph {
    #[default]
    None,
    /// On to somewhere.
    Right,
    /// Further down the page.
    Down,
    /// Out of the page, to another site.
    Out,
}

impl Glyph {
    fn paint(self, ui: &mut Ui, at: Point, size: f32, color: ui_kit_core::Color, nudge: f32) {
        let width = 1.7;
        match self {
            Glyph::None => {}
            Glyph::Right => arrow_right(ui, Point::new(at.x + nudge, at.y), size, color, width),
            Glyph::Down => arrow_down(ui, Point::new(at.x, at.y + nudge), size, color, width),
            Glyph::Out => arrow_out(
                ui,
                Point::new(at.x + nudge * 0.7, at.y - nudge * 0.7),
                size,
                color,
                width,
            ),
        }
    }
}

/// A pill with a label: white and solid when it is primary, glass when it
/// is not. It swells a little under the pointer and gives under a press.
#[derive(Clone, Copy, Debug)]
pub struct Pill<'a> {
    label: &'a str,
    primary: bool,
    glyph: Glyph,
    alpha: f32,
}

impl<'a> Pill<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            primary: false,
            glyph: Glyph::None,
            alpha: 1.0,
        }
    }

    /// The one thing to press: solid white.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    pub fn glyph(mut self, glyph: Glyph) -> Self {
        self.glyph = glyph;
        self
    }

    /// How faded in it is, 0 to 1.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// How wide the pill is for its label.
    pub fn width(&self, ui: &Ui) -> f32 {
        let text = typography::text_width(ui, self.label, scale::BUTTON);
        text + if self.glyph == Glyph::None { 0.0 } else { 26.0 } + 48.0
    }

    pub fn show(&self, ui: &mut Ui, springs: &mut Springs, id: &str, rect: Rect) -> Response {
        let alpha = self.alpha;
        if alpha <= 0.01 {
            return Response::default();
        }
        let response = ui.interact(id, rect);
        let hover = hover(ui, springs, id, response);
        let press = springs.track(
            &format!("press.{id}"),
            if response.active { 1.0 } else { 0.0 },
            PRESS,
        );
        let shape = grow(rect, 1.0 + 0.03 * hover - 0.04 * press);
        let radius = shape.height * 0.5;
        let kind = scale::BUTTON
            .color(if self.primary { INK } else { WHITE })
            .alpha(alpha);
        if self.primary {
            paint::shadow(
                ui,
                Rect::new(shape.x, shape.y + 8.0, shape.width, shape.height),
                radius,
                24.0,
                0.35 * alpha,
            );
            paint::fill(ui, shape, radius, faded(PAPER, alpha));
            if hover > 0.01 {
                // A band of the spectrum passing under the surface.
                spectral_line(
                    ui,
                    Point::new(shape.x + radius * 0.6, shape.y + shape.height - 1.5),
                    shape.width - radius * 1.2,
                    0.9 * hover * alpha,
                    1.5,
                );
            }
        } else {
            GlassLook::control(hover)
                .highlight_at(pointer_in(ui.input().pointer, shape))
                .opacity(alpha)
                .paint(ui, shape, radius);
        }
        let icon = if self.glyph == Glyph::None { 0.0 } else { 16.0 };
        let gap = if self.glyph == Glyph::None { 0.0 } else { 10.0 };
        let text_w = typography::text_width(ui, self.label, kind);
        let start = shape.x + (shape.width - text_w - gap - icon) * 0.5;
        typography::text(
            ui,
            Rect::new(start, shape.y, text_w + 4.0, shape.height),
            self.label,
            kind,
        );
        let icon_at = Point::new(start + text_w + gap, shape.y + (shape.height - icon) * 0.5);
        self.glyph.paint(ui, icon_at, icon, kind.color, 3.0 * hover);
        response
    }
}

/// A link in text that lights up under the pointer, underlined in the
/// spectrum while it is, or while it is the `current` one.
#[derive(Clone, Copy, Debug)]
pub struct Link<'a> {
    label: &'a str,
    kind: Type,
    current: bool,
}

impl<'a> Link<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            kind: scale::LINK,
            current: false,
        }
    }

    pub fn kind(mut self, kind: Type) -> Self {
        self.kind = kind;
        self
    }

    /// Where the page is now: lit and underlined.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    pub fn width(&self, ui: &Ui) -> f32 {
        typography::text_width(ui, self.label, self.kind)
    }

    /// Show it with its text's top left corner at `at`.
    pub fn show(&self, ui: &mut Ui, springs: &mut Springs, id: &str, at: Point) -> Response {
        let kind = self.kind;
        let width = self.width(ui);
        let height = kind.line_height().max(20.0);
        let rect = Rect::new(at.x - 6.0, at.y - 4.0, width + 12.0, height + 8.0);
        let response = ui.interact(id, rect);
        let hover = hover(ui, springs, id, response);
        let base = kind.color[3];
        let lit = if self.current {
            1.0
        } else {
            base + (1.0 - base) * hover
        };
        let shown = Type {
            color: [kind.color[0], kind.color[1], kind.color[2], lit],
            ..kind
        };
        typography::text(
            ui,
            Rect::new(at.x, at.y, width + 2.0, height),
            self.label,
            shown,
        );
        let underline = if self.current { 1.0 } else { hover };
        if underline > 0.01 && base > 0.01 {
            spectral_line(
                ui,
                Point::new(at.x + width * 0.5 * (1.0 - underline), at.y + height + 1.0),
                width * underline,
                0.9 * base.max(0.5),
                1.2,
            );
        }
        response
    }
}

/// A glass card that lifts towards the pointer and lights up where it is,
/// over a soft shadow.
#[derive(Clone, Copy, Debug)]
pub struct GlassCard {
    radius: f32,
    alpha: f32,
    strong: bool,
    speed: f32,
    pointer: Option<Option<Point>>,
}

/// A card as [`GlassCard::show`] drew it.
#[derive(Clone, Copy, Debug)]
pub struct Shown {
    /// Where the card is, lifted.
    pub rect: Rect,
    pub response: Response,
    /// How hovered it is, 0 to 1: for what is on it to light up with it.
    pub hover: f32,
}

impl GlassCard {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            alpha: 1.0,
            strong: false,
            speed: 0.0,
            pointer: None,
        }
    }

    /// How faded in it is, 0 to 1.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// A card that leads: lifted higher, with a wider rim and shadow.
    pub fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// How fast the page under it moves, in CSS pixels a second: glass
    /// bends further while it does.
    pub fn moving(mut self, velocity: f32) -> Self {
        self.speed = (velocity.abs() / 2500.0).min(1.0);
        self
    }

    /// Where the pointer that lights it is, in place of the frame's own
    /// (`None` for none, as after a finger lifted).
    pub fn pointer(mut self, pointer: Option<Point>) -> Self {
        self.pointer = Some(pointer);
        self
    }

    pub fn show(&self, ui: &mut Ui, springs: &mut Springs, id: &str, rect: Rect) -> Shown {
        let a = self.alpha;
        let response = ui.interact(id, rect);
        let hover = hover(ui, springs, id, response) * a;
        let lift = if self.strong { 6.0 } else { 4.0 } * hover;
        let rect = Rect::new(rect.x, rect.y - lift, rect.width, rect.height);
        paint::shadow(
            ui,
            Rect::new(
                rect.x + 12.0,
                rect.y + 24.0 + lift,
                rect.width - 24.0,
                rect.height - 18.0,
            ),
            self.radius,
            if self.strong { 48.0 } else { 36.0 },
            (0.32 + 0.15 * hover) * a,
        );
        let pointer = self.pointer.unwrap_or_else(|| hovering(ui));
        let mut look = GlassLook::card(hover).moving(self.speed);
        if self.strong {
            look = look.strong();
        }
        look.highlight_at(pointer_in(pointer, rect))
            .opacity(a)
            .paint(ui, rect, self.radius);
        Shown {
            rect,
            response,
            hover,
        }
    }
}

/// Wide-spaced capitals with a stroke of the spectrum before them -- and
/// after them too, when centred in `center_in` -- drawn in as `a` goes 0
/// to 1. `x` and `y` are its top left, or the left of the width it is
/// centred in.
pub fn eyebrow(ui: &mut Ui, x: f32, y: f32, label: &str, a: f32, center_in: Option<f32>) {
    let line_len = 28.0 * a;
    let kind = scale::EYEBROW;
    match center_in {
        Some(width) => {
            let w = typography::text_width(ui, label, kind);
            let start = x + (width - w) * 0.5;
            spectral_line(
                ui,
                Point::new(start - 14.0 - line_len, y + 8.0),
                line_len,
                a,
                1.5,
            );
            typography::text(ui, Rect::new(start, y, w + 4.0, 16.0), label, kind.alpha(a));
            spectral_line(ui, Point::new(start + w + 14.0, y + 8.0), line_len, a, 1.5);
        }
        None => {
            spectral_line(ui, Point::new(x, y + 8.0), line_len, a, 1.5);
            // As wide as the label: a long one is not cut short.
            let w = typography::text_width(ui, label, kind).max(300.0);
            typography::text(
                ui,
                Rect::new(x + 42.0, y, w + 4.0, 16.0),
                label,
                kind.alpha(a),
            );
        }
    }
}
