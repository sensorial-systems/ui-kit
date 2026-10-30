//! Scrolling that feels like the platform's: one state for a list or a page
//! that is taller than where it shows.
//!
//! - **Drag the content** (a finger, or the mouse): it follows, past either
//!   end only reluctantly (a rubber band), and let go moving it coasts on,
//!   slowing as it goes, and springs back if it ran past an end. A press on
//!   a list still coasting catches it, and is not a tap.
//! - **The scrollbar:** grab the thumb and drag it; press the track and the
//!   thumb jumps there, under the pointer, and drags on from there.
//! - **The middle button:** [`Autoscroll`], as Windows has it.
//! - **The wheel, keys and pages** ([`Scroller::scroll_by`],
//!   [`Scroller::scroll_to`]) glide there rather than jump; keys held
//!   ([`Scroller::move_by`]) move it as they go.
//! - **Snapping** ([`Scroller::snapping`]): a list of rows comes to rest
//!   with a row at the top.
//!
//! [`Scroller::update`] reads each step of input before the content does,
//! and says where the thumb is and whether the press is a drag (so the
//! content does not take it as a tap). The content is then drawn
//! [`Scroller::offset`] up, clipped to its view; the scrollbar however it
//! likes, at [`Scrolled::thumb`].
//!
//! ```
//! # use ui_kit_immediate::{Input, Point, Rect, Scroller};
//! let mut scroller = Scroller::new();
//! let view = Rect::new(0.0, 0.0, 300.0, 400.0);
//! let step = |scroller: &mut Scroller, y: f32, down: bool| {
//!     let mut input = Input { pointer: Some(Point::new(150.0, y)), down, ..Input::default() };
//!     scroller.update(&mut input, view, None, 2000.0, 1.0 / 60.0);
//! };
//! step(&mut scroller, 300.0, true);
//! for y in [280.0, 250.0, 210.0, 160.0] {
//!     step(&mut scroller, y, true); // flicked up
//! }
//! step(&mut scroller, 160.0, false); // and let go
//! let let_go = scroller.offset();
//! for _ in 0..30 {
//!     step(&mut scroller, 160.0, false);
//! }
//! assert!(scroller.offset() > let_go, "it coasts on");
//! ```

use crate::{Autoscroll, Input, Rect, Ui};

/// What the pointer's press is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Grab {
    None,
    /// Pressed on the content at height `from`, the offset `start` then; a
    /// drag once it has moved [`Scroller::SLOP`] (or caught it coasting).
    Content { from: f32, start: f32, dragging: bool },
    /// The thumb, held `at` below its top.
    Thumb { at: f32 },
    /// Pressed somewhere else: nothing to the scroller until let go.
    Elsewhere,
}

/// What [`Scroller::update`] made of a step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scrolled {
    /// The content is being dragged: the press is not a tap on what is
    /// under it.
    pub dragging: bool,
    /// The scrollbar has the press (its thumb, or its track).
    pub held: bool,
    /// Where the thumb is, in the track given, while there is anything to
    /// scroll.
    pub thumb: Option<Rect>,
    /// The pointer is over the thumb, or holding it.
    pub thumb_hovered: bool,
}

/// One scrolled view's state; see the module.
#[derive(Clone, Debug)]
pub struct Scroller {
    offset: f32,
    /// Canvas units a second, down positive, while it coasts or springs.
    velocity: f32,
    /// Where the wheel, the keys or a page are taking it: it glides there.
    target: Option<f32>,
    grab: Grab,
    /// The pointer's height while dragging the content, and when, the
    /// newest last: how fast it was going when let go.
    samples: Vec<(f64, f32)>,
    autoscroll: Autoscroll,
    /// Seconds, by the steps' `dt`.
    clock: f64,
    was_down: bool,
    /// The furthest it scrolls, and the view's height, as of the last step.
    max: f32,
    view: f32,
    snap: Option<f32>,
    /// Wheel travel not yet half a row, where it snaps.
    wheel: f32,
    min_thumb: f32,
}

impl Default for Scroller {
    fn default() -> Self {
        Self::new()
    }
}

impl Scroller {
    /// How far a press on the content moves before it is a drag.
    pub const SLOP: f32 = 8.0;
    /// How fast coasting slows: its speed falls by e every 1/FRICTION
    /// seconds (iOS's normal deceleration is 2).
    pub const FRICTION: f32 = 2.4;
    /// How stiff the spring back from past an end is (critically damped).
    const STIFFNESS: f32 = 400.0;
    /// How quickly a glide (the wheel, a page) closes in: e times nearer
    /// every 1/GLIDE seconds.
    const GLIDE: f32 = 18.0;
    /// Coasting slower than this (units a second) stops, or snaps.
    const REST: f32 = 10.0;
    const SNAP_SPEED: f32 = 160.0;
    /// The fastest a flick sends it.
    const MAX_SPEED: f32 = 7000.0;
    /// How far back the pointer's path is read for its speed when let go.
    const VELOCITY_WINDOW: f64 = 0.1;
    /// How far either side of the track a press still takes the thumb.
    const REACH: f32 = 10.0;

    pub fn new() -> Self {
        Self {
            offset: 0.0,
            velocity: 0.0,
            target: None,
            grab: Grab::None,
            samples: Vec::new(),
            autoscroll: Autoscroll::new(),
            clock: 0.0,
            was_down: false,
            max: 0.0,
            view: 0.0,
            snap: None,
            wheel: 0.0,
            min_thumb: 24.0,
        }
    }

    /// Comes to rest only on a multiple of `pitch` (or the end): a list of
    /// rows `pitch` apart rests with a row at the top.
    pub fn snapping(mut self, pitch: f32) -> Self {
        self.snap = (pitch > 0.0).then_some(pitch);
        self
    }

    /// The shortest the thumb gets, however long the content.
    pub fn min_thumb(mut self, height: f32) -> Self {
        self.min_thumb = height;
        self
    }

    /// How far down it is scrolled; past an end (below 0, above
    /// [`Scroller::max`]) while it is pulled or springs back.
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// The furthest it scrolls, as of the last step.
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Where it is going to rest: where a glide ends, coasting would stop
    /// (on a row, if it snaps), or where it is.
    pub fn destination(&self) -> f32 {
        let headed = match self.target {
            Some(target) => target,
            None => self.offset + self.velocity / Self::FRICTION,
        };
        self.rest(headed)
    }

    /// Says how tall the view and the content are before it first shows
    /// (each step says so again), so a scroll asked for meanwhile (a
    /// [`Scroller::reveal`]) knows where the end is.
    pub fn measure(&mut self, view: f32, content: f32) {
        self.view = view;
        self.max = (content - view).max(0.0);
    }

    /// Puts it at `offset` at once, still.
    pub fn set_offset(&mut self, offset: f32) {
        self.offset = offset;
        self.stop();
    }

    /// Stops it where it is: no coasting, gliding or autoscroll.
    pub fn stop(&mut self) {
        self.velocity = 0.0;
        self.target = None;
        self.autoscroll.stop();
    }

    /// The wheel, or a page: `delta` further down (up negative), glided to.
    /// It stops what it was doing, autoscroll too. Where it snaps, each
    /// notch moves at least a row on from where it was going (however
    /// short a notch is beside a row), and a trackpad's small steps add up
    /// to half a row before it moves one.
    pub fn scroll_by(&mut self, delta: f32) {
        let from = self.target.unwrap_or(self.offset.clamp(0.0, self.limit()));
        self.stop();
        let to = match self.snap {
            Some(pitch) => {
                if self.wheel * delta < 0.0 {
                    self.wheel = 0.0;
                }
                self.wheel += delta;
                if self.wheel.abs() < 0.5 * pitch {
                    self.target = Some(from);
                    return;
                }
                let rows = (self.wheel.abs() / pitch).round().max(1.0) * self.wheel.signum();
                self.wheel = 0.0;
                self.rest(from) + rows * pitch
            }
            None => from + delta,
        };
        self.target = Some(to.clamp(0.0, self.limit()));
    }

    /// Glides to `offset` (clamped: `f32::INFINITY` is the end).
    pub fn scroll_to(&mut self, offset: f32) {
        self.stop();
        self.target = Some(offset.clamp(0.0, self.limit()));
    }

    /// Moves it `delta` at once, as a key held does each step.
    pub fn move_by(&mut self, delta: f32) {
        if delta != 0.0 {
            let from = self.target.unwrap_or(self.offset);
            self.stop();
            self.offset = (from + delta).clamp(0.0, self.limit());
        }
    }

    /// The furthest it scrolls, or before its first step (not yet known)
    /// no limit: a scroll asked for before it shows is kept.
    fn limit(&self) -> f32 {
        if self.view > 0.0 { self.max } else { f32::INFINITY }
    }

    /// Glides just far enough to show the content from `top` to `bottom`
    /// (as the focus moves through a list).
    pub fn reveal(&mut self, top: f32, bottom: f32) {
        let at = self.target.unwrap_or(self.offset);
        let to = if top < at {
            top
        } else if bottom > at + self.view {
            bottom - self.view
        } else {
            return;
        };
        self.scroll_to(to);
    }

    /// Whether it moves on its own: coasting, springing, gliding or
    /// autoscrolling (the content wants drawing again).
    pub fn is_moving(&self) -> bool {
        self.velocity != 0.0
            || self.target.is_some()
            || self.autoscroll.is_active()
            || self.offset < 0.0
            || self.offset > self.max
    }

    /// Whether the middle button is scrolling it.
    pub fn is_autoscrolling(&self) -> bool {
        self.autoscroll.is_active()
    }

    /// Draws the autoscroll's anchor, after the content.
    pub fn paint_autoscroll(&self, ui: &mut Ui) {
        self.autoscroll.paint(ui);
    }

    /// Reads a step of input, `dt` seconds on from the last, before the
    /// content does: the content `content` tall shows in `view`, and the
    /// scrollbar runs down `track` (`None`: no scrollbar to grab). A press
    /// the middle button's autoscroll takes, or that stops it, is taken out
    /// of `input` (see [`Autoscroll::update`]); the scroller keeps its own
    /// time, by `dt`.
    pub fn update(
        &mut self,
        input: &mut Input,
        view: Rect,
        track: Option<Rect>,
        content: f32,
        dt: f32,
    ) -> Scrolled {
        let dt = dt.clamp(0.0, 0.1);
        self.clock += f64::from(dt);
        self.max = (content - view.height).max(0.0);
        self.view = view.height;
        let can_scroll = self.max > 0.0;
        let track = track.filter(|t| can_scroll && t.height > 0.0);

        // The middle button, by the scroller's clock.
        let time = input.time.replace(self.clock);
        let area = track.map_or(view, |t| union(view, t));
        let auto = self.autoscroll.update(input, area, can_scroll);
        input.time = time;
        if self.autoscroll.is_active() {
            self.velocity = 0.0;
            self.target = None;
            self.offset = (self.offset + auto).clamp(0.0, self.max);
        }

        let down = input.down;
        let pressed = down && !self.was_down;
        let released = !down && self.was_down;
        self.was_down = down;
        let pointer = input.pointer;
        let thumb = track.map(|t| self.thumb(t, content));
        let on_bar = |p: crate::Point| {
            track.is_some_and(|t| {
                p.y >= t.y && p.y <= t.bottom() && p.x >= t.x - Self::REACH && p.x <= t.right() + Self::REACH
            })
        };

        if pressed {
            // A press catches it coasting: that is all the press does.
            let caught = self.velocity.abs() > Self::SNAP_SPEED;
            self.velocity = 0.0;
            self.target = None;
            self.samples.clear();
            self.grab = match (pointer, track, thumb) {
                (Some(p), Some(t), Some(th)) if on_bar(p) => {
                    let at = if p.y >= th.y && p.y <= th.bottom() {
                        p.y - th.y
                    } else {
                        // The track: the thumb jumps under the pointer.
                        let at = 0.5 * th.height;
                        self.offset = self.thumb_offset(p.y - at, t, th.height);
                        at
                    };
                    Grab::Thumb { at }
                }
                (Some(p), _, _) if view.contains(p) => {
                    self.samples.push((self.clock, p.y));
                    Grab::Content { from: p.y, start: self.offset, dragging: caught }
                }
                _ => Grab::Elsewhere,
            };
        }

        if down {
            match (self.grab, pointer) {
                (Grab::Content { from, start, dragging }, Some(p)) => {
                    let mut grab = Grab::Content { from, start, dragging };
                    if !dragging && (p.y - from).abs() > Self::SLOP {
                        // Taken up from the slop's edge, so it does not jump.
                        let from = from + Self::SLOP * (p.y - from).signum();
                        grab = Grab::Content { from, start: self.offset, dragging: true };
                    }
                    if let Grab::Content { from, start, dragging: true } = grab {
                        self.offset = self.pulled(start + from - p.y);
                    }
                    self.grab = grab;
                    self.samples.push((self.clock, p.y));
                    let newest = self.clock;
                    self.samples.retain(|(t, _)| newest - t <= 2.0 * Self::VELOCITY_WINDOW);
                }
                (Grab::Thumb { at }, Some(p)) => {
                    if let (Some(t), Some(th)) = (track, thumb) {
                        self.offset = self.thumb_offset(p.y - at, t, th.height);
                    }
                }
                _ => {}
            }
        }

        let mut scrolled = Scrolled {
            dragging: matches!(self.grab, Grab::Content { dragging: true, .. }),
            held: matches!(self.grab, Grab::Thumb { .. }),
            ..Scrolled::default()
        };

        if released {
            if let Grab::Content { dragging: true, .. } = self.grab {
                self.velocity = self.release_speed();
            }
            self.grab = Grab::None;
            self.samples.clear();
        }

        let holding = matches!(self.grab, Grab::Content { .. } | Grab::Thumb { .. });
        if !holding && !self.autoscroll.is_active() {
            self.coast(dt);
        }

        if let Some(t) = track {
            let th = self.thumb(t, content);
            scrolled.thumb = Some(th);
            scrolled.thumb_hovered = scrolled.held
                || pointer.is_some_and(|p| on_bar(p) && p.y >= th.y && p.y <= th.bottom());
        }
        scrolled
    }

    /// Where `raw` is, pulled past an end: less the further it goes.
    fn pulled(&self, raw: f32) -> f32 {
        let band = |past: f32| {
            let d = self.view.max(1.0);
            (1.0 - 1.0 / (past * 0.55 / d + 1.0)) * d
        };
        if raw < 0.0 {
            -band(-raw)
        } else if raw > self.max {
            self.max + band(raw - self.max)
        } else {
            raw
        }
    }

    /// How fast the content went with the pointer just before it let go:
    /// nothing if it was held still.
    fn release_speed(&self) -> f32 {
        let Some(&(newest, y)) = self.samples.last() else {
            return 0.0;
        };
        let Some(&(oldest, y0)) = self.samples.iter().find(|(t, _)| newest - t <= Self::VELOCITY_WINDOW) else {
            return 0.0;
        };
        let span = (newest - oldest) as f32;
        if span < 1e-3 {
            return 0.0;
        }
        ((y0 - y) / span).clamp(-Self::MAX_SPEED, Self::MAX_SPEED)
    }

    /// Where it rests near `offset`: on a row, or the end.
    fn rest(&self, offset: f32) -> f32 {
        let offset = offset.clamp(0.0, self.max);
        match self.snap {
            Some(pitch) if self.max - offset < 0.5 * pitch => self.max,
            Some(pitch) => ((offset / pitch).round() * pitch).min(self.max),
            None => offset,
        }
    }

    /// Lets it move on its own for `dt`: gliding to a target, coasting
    /// (slowing), or springing back from past an end.
    fn coast(&mut self, dt: f32) {
        if let Some(target) = self.target {
            let to = self.rest(target);
            self.offset += (to - self.offset) * (1.0 - (-Self::GLIDE * dt).exp());
            if (to - self.offset).abs() < 0.5 {
                self.offset = to;
                self.target = None;
            }
            return;
        }
        let steps = (dt * 240.0).ceil().max(1.0);
        let h = dt / steps;
        let omega = Self::STIFFNESS.sqrt();
        for _ in 0..steps as usize {
            let bound = self.offset.clamp(0.0, self.max);
            let past = self.offset - bound;
            if past != 0.0 {
                // Back from past the end, a spring that does not overshoot.
                self.velocity += (-Self::STIFFNESS * past - 2.0 * omega * self.velocity) * h;
                self.offset += self.velocity * h;
                let now_past = self.offset - bound;
                if now_past.signum() != past.signum() || (now_past.abs() < 0.5 && self.velocity.abs() < 20.0) {
                    self.offset = bound;
                    self.velocity = 0.0;
                }
            } else if self.velocity != 0.0 {
                let decay = (-Self::FRICTION * h).exp();
                self.offset += self.velocity * (1.0 - decay) / Self::FRICTION;
                self.velocity *= decay;
            }
        }
        let inside = self.offset >= 0.0 && self.offset <= self.max;
        if inside && self.velocity.abs() < Self::REST {
            self.velocity = 0.0;
        }
        if inside && self.snap.is_some() && self.velocity.abs() < Self::SNAP_SPEED {
            // Slow enough: on to the row it is heading for.
            let to = self.rest(self.offset + 0.12 * self.velocity);
            self.velocity = 0.0;
            if (to - self.offset).abs() >= 0.01 {
                self.target = Some(to);
            }
        }
    }

    /// The thumb in `track`, for content `content` tall: as long as the
    /// view is of the content, shorter as it is pulled past an end.
    fn thumb(&self, track: Rect, content: f32) -> Rect {
        let full = (track.height * self.view / content.max(1.0))
            .clamp(self.min_thumb.min(track.height), track.height);
        let past = if self.offset < 0.0 {
            -self.offset
        } else {
            (self.offset - self.max).max(0.0)
        };
        let height = (full - 2.0 * past * full / self.view.max(1.0)).max(0.4 * full);
        let fraction = if self.max > 0.0 { (self.offset / self.max).clamp(0.0, 1.0) } else { 0.0 };
        let y = track.y + fraction * (track.height - height);
        Rect::new(track.x, y, track.width, height)
    }

    /// The offset with the thumb's top at `top`.
    fn thumb_offset(&self, top: f32, track: Rect, thumb: f32) -> f32 {
        let travel = track.height - thumb;
        if travel <= 0.0 {
            return 0.0;
        }
        ((top - track.y) / travel).clamp(0.0, 1.0) * self.max
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;

    const VIEW: Rect = Rect { x: 0.0, y: 0.0, width: 300.0, height: 400.0 };
    const TRACK: Rect = Rect { x: 290.0, y: 0.0, width: 4.0, height: 400.0 };
    const CONTENT: f32 = 2000.0;
    const DT: f32 = 1.0 / 60.0;

    /// A step with the primary button (or a finger) at `at`, `down`.
    fn step(s: &mut Scroller, at: (f32, f32), down: bool) -> Scrolled {
        step_with(s, at, [down, false])
    }

    /// A step with the primary and middle buttons as given.
    fn step_with(s: &mut Scroller, at: (f32, f32), [down, middle]: [bool; 2]) -> Scrolled {
        let mut input = Input { pointer: Some(Point::new(at.0, at.1)), down, middle_down: middle, ..Input::default() };
        s.update(&mut input, VIEW, Some(TRACK), CONTENT, DT)
    }

    fn settle(s: &mut Scroller, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            step(s, (150.0, 200.0), false);
        }
    }

    #[test]
    fn the_thumb_drags_and_the_track_jumps_under_the_pointer() {
        let mut s = Scroller::new();
        let thumb = step(&mut s, (150.0, 200.0), false).thumb.unwrap();
        assert_eq!(thumb.height, 80.0, "as long as the view is of the content");
        // Grabbed a little off the track, by its middle, and dragged half
        // the travel down.
        let grab = (TRACK.x + 8.0, thumb.y + 40.0);
        let r = step(&mut s, grab, true);
        assert!(r.held && r.thumb_hovered && !r.dragging);
        assert_eq!(s.offset(), 0.0, "grabbing does not move it");
        let r = step(&mut s, (grab.0, grab.1 + 160.0), true);
        assert_eq!(s.offset(), 800.0, "half the travel, half the way");
        assert_eq!(r.thumb.unwrap().y, 160.0);
        // Dragged past the end, it stops at the end; let go, it stays.
        step(&mut s, (grab.0, 900.0), true);
        assert_eq!(s.offset(), 1600.0);
        step(&mut s, (grab.0, 900.0), false);
        settle(&mut s, 0.5);
        assert_eq!(s.offset(), 1600.0, "no momentum from the thumb");

        // A press on the track above the thumb: the thumb's middle jumps
        // there, and drags on.
        let r = step(&mut s, (TRACK.x + 2.0, 100.0), true);
        assert!(r.held);
        assert_eq!(r.thumb.unwrap().y, 60.0, "centred under the pointer");
        assert_eq!(s.offset(), 300.0);
        step(&mut s, (TRACK.x + 2.0, 140.0), true);
        assert_eq!(s.offset(), 500.0);
        step(&mut s, (TRACK.x + 2.0, 140.0), false);
        // Nothing to scroll: no thumb, and the track is no different.
        let mut short = Scroller::new();
        let mut input = Input { pointer: Some(Point::new(292.0, 100.0)), down: true, ..Input::default() };
        let r = short.update(&mut input, VIEW, Some(TRACK), 300.0, DT);
        assert!(r.thumb.is_none() && !r.held);
    }

    #[test]
    fn a_flick_coasts_slowing_down_and_a_slow_drag_does_not() {
        let mut s = Scroller::new();
        step(&mut s, (150.0, 350.0), true);
        // Up 20 a step: 1200 units a second.
        for k in 1..=6 {
            let r = step(&mut s, (150.0, 350.0 - 20.0 * k as f32), true);
            assert!(r.dragging || k == 1, "a drag once past the slop");
        }
        let held = s.offset();
        assert!((held - (120.0 - Scroller::SLOP)).abs() < 1e-3, "it follows the finger from past the slop: {held}");
        step(&mut s, (150.0, 230.0), false);
        let let_go = 1200.0 * (-Scroller::FRICTION * DT).exp();
        assert!((s.velocity - let_go).abs() < 1.0, "as fast as the finger: {}", s.velocity);
        // It goes on, each step less far than the last.
        let mut last = s.offset();
        let mut gains = vec![];
        for _ in 0..30 {
            step(&mut s, (150.0, 230.0), false);
            gains.push(s.offset() - last);
            last = s.offset();
        }
        assert!(gains.iter().all(|g| *g > 0.0), "coasting on");
        assert!(gains.windows(2).all(|w| w[1] < w[0]), "slowing: {gains:?}");
        // Speed falls by e every 1/FRICTION seconds.
        let half_second = 1200.0 * (-Scroller::FRICTION * 31.0 * DT).exp();
        assert!((s.velocity - half_second).abs() < 0.02 * half_second, "{} {half_second}", s.velocity);
        settle(&mut s, 5.0);
        assert_eq!(s.velocity, 0.0, "and comes to rest");
        // Coasting, it would have gone v / FRICTION.
        assert!((s.offset() - (held + 1200.0 / Scroller::FRICTION)).abs() < 10.0, "{}", s.offset());

        // Moved, then held still before letting go: no coast.
        let mut s = Scroller::new();
        step(&mut s, (150.0, 350.0), true);
        step(&mut s, (150.0, 250.0), true);
        for _ in 0..10 {
            step(&mut s, (150.0, 250.0), true);
        }
        step(&mut s, (150.0, 250.0), false);
        let at = s.offset();
        settle(&mut s, 1.0);
        assert_eq!(s.offset(), at, "held still, it stays");
    }

    #[test]
    fn past_an_end_it_pulls_reluctantly_and_springs_back() {
        let mut s = Scroller::new();
        step(&mut s, (150.0, 100.0), true);
        step(&mut s, (150.0, 110.0), true);
        step(&mut s, (150.0, 310.0), true);
        assert!(s.offset() < 0.0 && s.offset() > -200.0, "pulled less far than the finger: {}", s.offset());
        for _ in 0..10 {
            step(&mut s, (150.0, 310.0), true);
        }
        step(&mut s, (150.0, 310.0), false);
        settle(&mut s, 1.0);
        assert_eq!(s.offset(), 0.0, "back to the top");

        // A flick towards the end runs past it a little, and comes back.
        let mut s = Scroller::new();
        s.set_offset(1500.0);
        step(&mut s, (150.0, 350.0), true);
        for k in 1..=5 {
            step(&mut s, (150.0, 350.0 - 40.0 * k as f32), true);
        }
        step(&mut s, (150.0, 150.0), false);
        let mut furthest = 0.0f32;
        for _ in 0..120 {
            step(&mut s, (150.0, 150.0), false);
            furthest = furthest.max(s.offset());
        }
        assert!(furthest > 1600.0 && furthest < 1700.0, "ran past a little: {furthest}");
        assert_eq!(s.offset(), 1600.0, "and back to the end");
        assert!(!s.is_moving());
    }

    #[test]
    fn a_press_on_it_coasting_catches_it_and_is_no_tap() {
        let mut s = Scroller::new();
        step(&mut s, (150.0, 350.0), true);
        for k in 1..=5 {
            step(&mut s, (150.0, 350.0 - 40.0 * k as f32), true);
        }
        step(&mut s, (150.0, 150.0), false);
        step(&mut s, (150.0, 150.0), false);
        let r = step(&mut s, (150.0, 150.0), true);
        assert!(r.dragging, "the press that catches it is not a tap");
        let caught = s.offset();
        step(&mut s, (150.0, 150.0), true);
        assert_eq!(s.offset(), caught, "it stopped");
    }

    #[test]
    fn the_middle_button_scrolls_faster_further_from_where_it_was_pressed() {
        let mut s = Scroller::new();
        step_with(&mut s, (150.0, 200.0), [false, true]);
        step_with(&mut s, (150.0, 200.0), [false, false]);
        assert!(s.is_autoscrolling());
        let speed = |s: &mut Scroller, y: f32| {
            let before = s.offset();
            step_with(s, (150.0, y), [false, false]);
            (s.offset() - before) / DT
        };
        assert_eq!(speed(&mut s, 205.0), 0.0, "still near the anchor");
        let near = speed(&mut s, 250.0);
        let far = speed(&mut s, 350.0);
        assert!(near > 0.0 && far > 2.0 * near, "{near} {far}");
        let up = speed(&mut s, 150.0);
        assert!((up + near).abs() < 0.05, "up as fast as down: {up} {near}");
        // A click stops it, and is no tap or drag on the content.
        let mut input = Input { pointer: Some(Point::new(150.0, 200.0)), down: true, ..Input::default() };
        let r = s.update(&mut input, VIEW, Some(TRACK), CONTENT, DT);
        assert!(!s.is_autoscrolling() && !input.down && !r.dragging);
        // Nor does it scroll past the ends.
        let mut s = Scroller::new();
        step_with(&mut s, (150.0, 200.0), [false, true]);
        step_with(&mut s, (150.0, 200.0), [false, false]);
        for _ in 0..30 {
            step_with(&mut s, (150.0, 20.0), [false, false]);
        }
        assert_eq!(s.offset(), 0.0);
    }

    #[test]
    fn the_wheel_glides_and_a_list_of_rows_rests_on_a_row() {
        let mut s = Scroller::new().snapping(50.0);
        step(&mut s, (150.0, 200.0), false);
        s.scroll_by(40.0);
        step(&mut s, (150.0, 200.0), false);
        assert!(s.offset() > 0.0 && s.offset() < 50.0, "gliding: {}", s.offset());
        settle(&mut s, 0.5);
        assert_eq!(s.offset(), 50.0, "on to the row");
        // A flick rests on a row too.
        step(&mut s, (150.0, 350.0), true);
        for k in 1..=4 {
            step(&mut s, (150.0, 350.0 - 23.0 * k as f32), true);
        }
        step(&mut s, (150.0, 258.0), false);
        settle(&mut s, 4.0);
        assert!(s.offset() > 150.0 && s.offset() % 50.0 == 0.0, "{}", s.offset());
        // Each notch of a wheel a row, however quickly they come and
        // however short a notch is beside a row; small steps add up.
        let mut w = Scroller::new().snapping(86.0);
        step(&mut w, (150.0, 200.0), false);
        for notches in 1..=3 {
            w.scroll_by(48.0);
            step(&mut w, (150.0, 200.0), false);
            assert_eq!(w.destination(), notches as f32 * 86.0);
        }
        w.scroll_by(-48.0);
        assert_eq!(w.destination(), 2.0 * 86.0, "and back up");
        settle(&mut w, 1.0);
        for _ in 0..8 {
            w.scroll_by(5.0);
        }
        assert_eq!(w.destination(), 2.0 * 86.0, "not yet half a row");
        w.scroll_by(5.0);
        assert_eq!(w.destination(), 3.0 * 86.0);
        w.scroll_by(500.0);
        assert_eq!(w.destination(), 9.0 * 86.0, "a long swipe, rows as far");
        // Revealing a row below the view brings it just into view.
        s.set_offset(0.0);
        s.reveal(450.0, 500.0);
        settle(&mut s, 1.0);
        assert_eq!(s.offset(), 100.0);
        // Pages and ends.
        s.scroll_to(f32::INFINITY);
        settle(&mut s, 1.0);
        assert_eq!(s.offset(), 1600.0);
        s.move_by(-30.0);
        assert_eq!(s.offset(), 1570.0, "a key held moves it at once");
        // Asked before it first shows, it goes there once it does.
        let mut s = Scroller::new();
        s.scroll_to(f32::INFINITY);
        settle(&mut s, 1.0);
        assert_eq!(s.offset(), 1600.0);
    }
}
