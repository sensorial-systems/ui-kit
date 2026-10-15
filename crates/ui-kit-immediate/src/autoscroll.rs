//! Scrolling with the middle button, as Windows does it.
//!
//! - **Click:** press and release the middle button without moving, and an
//!   anchor appears where it was pressed. The content then scrolls on its
//!   own, the further the pointer is from the anchor the faster, until any
//!   button is pressed, Escape, or the wheel turns. The press that stops it
//!   reaches nothing under the pointer.
//! - **Hold:** press the middle button and move away, and it scrolls the
//!   same way only while the button is held.
//!
//! Near the anchor (within [`Autoscroll::DEAD_ZONE`]) nothing moves, so a
//! hand that shakes doesn't scroll. [`Autoscroll::update`] reads each step
//! of input before the UI does and says how far to scroll it;
//! [`Autoscroll::paint`] draws the anchor and the cursor after the content.
//!
//! ```
//! # use ui_kit_immediate::{Autoscroll, Input, Point, Rect};
//! let mut autoscroll = Autoscroll::new();
//! let area = Rect::new(0.0, 0.0, 800.0, 600.0);
//! let mut step = |pointer: Point, middle: bool, time: f64| {
//!     let mut input = Input { pointer: Some(pointer), middle_down: middle, time: Some(time), ..Input::default() };
//!     autoscroll.update(&mut input, area, true)
//! };
//! step(Point::new(400.0, 300.0), true, 0.0); // press the middle button
//! step(Point::new(400.0, 300.0), false, 0.1); // and let go: an anchor
//! assert!(step(Point::new(400.0, 400.0), false, 0.2) > 0.0); // below it, the content scrolls down
//! ```

use crate::components::theme::with_alpha;
use crate::{vector, CursorIcon, Input, Key, Layer, Point, Rect, Ui};

/// What the middle button is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Idle,
    /// Held since it was pressed at `anchor`; `moved` once it left there.
    Held { anchor: Point, moved: bool },
    /// Let go without moving: scrolling until something stops it.
    Free { anchor: Point },
}

/// Middle-button scrolling for one scrolled area; see the module.
#[derive(Clone, Debug)]
pub struct Autoscroll {
    state: State,
    middle: bool,
    primary: bool,
    secondary: bool,
    /// Buttons whose press stopped autoscroll: hidden from the UI until
    /// they are let go.
    swallow: [bool; 3],
    time: Option<f64>,
    pointer: Option<Point>,
}

impl Default for Autoscroll {
    fn default() -> Self {
        Self::new()
    }
}

impl Autoscroll {
    /// How far from the anchor the pointer can be without scrolling.
    pub const DEAD_ZONE: f32 = 12.0;
    /// How far the pointer moves from the anchor before a held button
    /// counts as dragging (and letting it go stops).
    const DRAG: f32 = 6.0;

    pub fn new() -> Self {
        Self {
            state: State::Idle,
            middle: false,
            primary: false,
            secondary: false,
            swallow: [false; 3],
            time: None,
            pointer: None,
        }
    }

    /// Whether it is scrolling (an anchor is showing).
    pub fn is_active(&self) -> bool {
        self.state != State::Idle
    }

    /// Stops it, as if the reader had clicked.
    pub fn stop(&mut self) {
        self.state = State::Idle;
    }

    /// Reads a step of input before the UI does. A press of the middle
    /// button in `area` starts autoscroll when there is something to scroll
    /// (`can_scroll`); a press that stops it is taken out of `input`, and
    /// the middle button never reaches the UI while it scrolls. Returns how
    /// far to scroll for this step, in pixels, down positive.
    pub fn update(&mut self, input: &mut Input, area: Rect, can_scroll: bool) -> f32 {
        let dt = match (self.time, input.time) {
            (Some(before), Some(now)) => (now - before).clamp(0.0, 0.1) as f32,
            (_, None) => 1.0 / 60.0,
            _ => 0.0,
        };
        self.time = input.time.or(self.time);
        if input.pointer.is_some() {
            self.pointer = input.pointer;
        }
        let middle_pressed = input.middle_down && !self.middle;
        let middle_released = !input.middle_down && self.middle;
        let primary_pressed = input.down && !self.primary;
        let secondary_pressed = input.secondary_down && !self.secondary;
        self.middle = input.middle_down;
        self.primary = input.down;
        self.secondary = input.secondary_down;

        match self.state {
            State::Idle => {
                let at = input.pointer.filter(|at| area.contains(*at));
                if let (true, true, Some(at)) = (middle_pressed, can_scroll, at) {
                    self.state = State::Held {
                        anchor: at,
                        moved: false,
                    };
                    self.swallow[2] = true;
                }
            }
            State::Held { anchor, moved } => {
                let moved = moved
                    || self
                        .pointer
                        .is_some_and(|p| (p - anchor).length() > Self::DRAG);
                self.state = if middle_released {
                    if moved {
                        State::Idle
                    } else {
                        State::Free { anchor }
                    }
                } else {
                    State::Held { anchor, moved }
                };
            }
            State::Free { .. } => {
                let escape = input.keys.contains(&Key::Escape);
                if primary_pressed || secondary_pressed || middle_pressed {
                    self.state = State::Idle;
                    self.swallow = [
                        self.swallow[0] || primary_pressed,
                        self.swallow[1] || secondary_pressed,
                        self.swallow[2] || middle_pressed,
                    ];
                } else if escape {
                    self.state = State::Idle;
                    input.keys.retain(|k| *k != Key::Escape);
                } else if input.scroll.y != 0.0 {
                    self.state = State::Idle;
                }
            }
        }

        // What stopped (or started) it reaches nothing under the pointer.
        for (swallowed, held) in self.swallow.iter_mut().zip([
            &mut input.down,
            &mut input.secondary_down,
            &mut input.middle_down,
        ]) {
            if *swallowed {
                if *held {
                    *held = false;
                } else {
                    *swallowed = false;
                }
            }
        }

        match (self.anchor(), self.pointer) {
            (Some(anchor), Some(pointer)) => speed(pointer.y - anchor.y) * dt,
            _ => 0.0,
        }
    }

    fn anchor(&self) -> Option<Point> {
        match self.state {
            State::Held { anchor, .. } | State::Free { anchor } => Some(anchor),
            State::Idle => None,
        }
    }

    /// Draws the anchor over everything, with the scrolling cursor, and asks
    /// for the next frame while it scrolls. Call it after the content.
    pub fn paint(&self, ui: &mut Ui) {
        let Some(anchor) = self.anchor() else {
            return;
        };
        let palette = ui.theme.palette;
        let offset = self.pointer.map_or(0.0, |p| p.y - anchor.y);
        let lit = |on: bool| if on { palette.text } else { with_alpha(palette.muted, 0.7) };
        ui.with_layer(Layer::TOOLTIP, |ui| {
            vector::circle(ui, anchor, 16.0, palette.border);
            vector::circle(ui, anchor, 15.0, palette.surface);
            let (x, y) = (anchor.x, anchor.y);
            let up = [Point::new(x, y - 11.0), Point::new(x - 5.0, y - 5.0), Point::new(x + 5.0, y - 5.0)];
            let down = [Point::new(x, y + 11.0), Point::new(x + 5.0, y + 5.0), Point::new(x - 5.0, y + 5.0)];
            vector::fill_triangles(ui, &up, lit(offset < -Self::DEAD_ZONE));
            vector::fill_triangles(ui, &down, lit(offset > Self::DEAD_ZONE));
            vector::circle(ui, anchor, 2.0, palette.text);
        });
        ui.set_cursor(CursorIcon::ResizeVertical);
        ui.output_mut().repaint_within(0.0);
    }
}

/// How fast to scroll, in pixels a second, for the pointer this far below
/// the anchor (above: negative): nothing within the dead zone, then faster
/// the further it goes.
fn speed(offset: f32) -> f32 {
    let beyond = offset.abs() - Autoscroll::DEAD_ZONE;
    if beyond <= 0.0 {
        return 0.0;
    }
    offset.signum() * (4.0 * beyond + 0.04 * beyond * beyond)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };

    struct Steps {
        autoscroll: Autoscroll,
        time: f64,
    }

    impl Steps {
        fn new() -> Self {
            Self {
                autoscroll: Autoscroll::new(),
                time: 0.0,
            }
        }

        /// A step 0.1 s after the last: what it scrolled, and the input as
        /// the UI gets it.
        fn step(&mut self, at: (f32, f32), buttons: [bool; 3]) -> (f32, Input) {
            self.time += 0.1;
            let mut input = Input {
                pointer: Some(Point::new(at.0, at.1)),
                down: buttons[0],
                secondary_down: buttons[1],
                middle_down: buttons[2],
                time: Some(self.time),
                ..Input::default()
            };
            let scrolled = self.autoscroll.update(&mut input, AREA, true);
            (scrolled, input)
        }
    }

    const NONE: [bool; 3] = [false; 3];
    const MIDDLE: [bool; 3] = [false, false, true];
    const PRIMARY: [bool; 3] = [true, false, false];

    #[test]
    fn a_click_leaves_an_anchor_that_scrolls_until_the_next_click() {
        let mut s = Steps::new();
        let (_, input) = s.step((400.0, 300.0), MIDDLE);
        assert!(!input.middle_down, "the middle press is the autoscroll's");
        s.step((400.0, 300.0), NONE);
        assert!(s.autoscroll.is_active(), "a still click leaves it scrolling");
        let (down, _) = s.step((400.0, 400.0), NONE);
        let (up, _) = s.step((400.0, 200.0), NONE);
        assert!(down > 0.0 && up < 0.0, "towards the pointer: {down} {up}");
        let (farther, _) = s.step((400.0, 500.0), NONE);
        assert!(farther > down, "faster further away");
        let (still, _) = s.step((400.0, 308.0), NONE);
        assert_eq!(still, 0.0, "nothing near the anchor");
        // A click stops it, and the button it pressed reaches nothing.
        let (_, input) = s.step((400.0, 300.0), PRIMARY);
        assert!(!s.autoscroll.is_active());
        assert!(!input.down, "the click that stopped it is taken");
        let (_, input) = s.step((400.0, 300.0), PRIMARY);
        assert!(!input.down, "still taken while held");
        let (_, input) = s.step((400.0, 300.0), NONE);
        let (_, input2) = s.step((400.0, 300.0), PRIMARY);
        assert!(!input.down && input2.down, "the next click is the UI's again");
    }

    #[test]
    fn a_held_drag_scrolls_until_let_go() {
        let mut s = Steps::new();
        s.step((400.0, 300.0), MIDDLE);
        let (scrolled, _) = s.step((400.0, 450.0), MIDDLE);
        assert!(scrolled > 0.0);
        s.step((400.0, 450.0), NONE);
        assert!(!s.autoscroll.is_active(), "letting go after a drag stops it");
    }

    #[test]
    fn escape_and_the_wheel_stop_it_and_nothing_to_scroll_never_starts_it() {
        let mut s = Steps::new();
        s.step((400.0, 300.0), MIDDLE);
        s.step((400.0, 300.0), NONE);
        let mut input = Input {
            pointer: Some(Point::new(400.0, 300.0)),
            keys: vec![Key::Escape],
            time: Some(10.0),
            ..Input::default()
        };
        s.autoscroll.update(&mut input, AREA, true);
        assert!(!s.autoscroll.is_active() && input.keys.is_empty(), "Escape is taken");

        s.step((400.0, 300.0), MIDDLE);
        s.step((400.0, 300.0), NONE);
        let mut input = Input {
            pointer: Some(Point::new(400.0, 300.0)),
            scroll: Point::new(0.0, 40.0),
            time: Some(20.0),
            ..Input::default()
        };
        s.autoscroll.update(&mut input, AREA, true);
        assert!(!s.autoscroll.is_active(), "the wheel stops it");

        let mut autoscroll = Autoscroll::new();
        let mut input = Input {
            pointer: Some(Point::new(400.0, 300.0)),
            middle_down: true,
            ..Input::default()
        };
        autoscroll.update(&mut input, AREA, false);
        assert!(!autoscroll.is_active(), "nothing to scroll");
        let mut input = Input {
            pointer: Some(Point::new(900.0, 300.0)),
            middle_down: true,
            ..Input::default()
        };
        Autoscroll::new().update(&mut input, AREA, true);
    }
}
