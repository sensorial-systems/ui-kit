//! Values that change over time: keyframes, the curves between them, and the
//! tracks a timeline is made of.
//!
//! Nothing here draws. A [`Track`] is a sorted run of [`Keyframe`]s and the
//! one question worth asking of it -- [`Track::value_at`] -- so the thing that
//! renders the animation can read it without a UI, and the curve editor in
//! `ui-kit-immediate` is only a way of writing the same numbers by hand.
//!
//! Time is whatever the caller counts in, as long as it is the same
//! throughout: seconds for a video, beats for a sequencer, metres for a
//! profile along a road. Values are `f32`, one number to a track; something
//! with three of them -- a position, a colour -- is three tracks, which is
//! what lets a track be added, removed and shaped on its own.

use crate::Color;
use serde::{Deserialize, Serialize};

/// How a [`Keyframe`] reaches the one after it.
///
/// The mode belongs to the key the segment starts from, so a curve reads left
/// to right: this key holds, or runs straight, or eases into the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpolation {
    /// Nothing happens until the next key, then the value jumps to it.
    Hold,
    /// A straight line to the next key.
    Linear,
    /// A curve that passes smoothly through the neighbouring keys, with
    /// tangents worked out from them rather than set by hand.
    #[default]
    Smooth,
    /// A cubic curve shaped by this key's outgoing handle and the next key's
    /// incoming one.
    Bezier,
}

impl Interpolation {
    /// The four, in the order a toolbar offers them.
    pub const ALL: [Self; 4] = [Self::Hold, Self::Linear, Self::Smooth, Self::Bezier];

    pub fn name(self) -> &'static str {
        match self {
            Self::Hold => "Hold",
            Self::Linear => "Linear",
            Self::Smooth => "Smooth",
            Self::Bezier => "Bezier",
        }
    }
}

/// One end of a [`Keyframe`]'s tangent, as an offset from the key itself in
/// the track's own time and value units.
///
/// A handle only shapes the curve while the segment it belongs to is
/// [`Interpolation::Bezier`]; the other modes work the tangents out
/// themselves and leave the handles where they were, so switching a key to
/// Bezier and back does not lose the shape that was drawn by hand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Handle {
    pub time: f32,
    pub value: f32,
}

impl Handle {
    pub fn new(time: f32, value: f32) -> Self {
        Self { time, value }
    }
}

/// A value the track is known to have at a time, and how it leaves it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    /// How the segment from here to the next key is shaped.
    pub interpolation: Interpolation,
    /// Towards the previous key. Shapes the segment arriving here when that
    /// segment is a Bezier.
    pub incoming: Handle,
    /// Towards the next key.
    pub outgoing: Handle,
}

impl Keyframe {
    pub fn new(time: f32, value: f32) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::default(),
            incoming: Handle::default(),
            outgoing: Handle::default(),
        }
    }

    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }
}

/// How far along a segment a handle reaches when it is first given one: a
/// third of the way, which is the tangent a straight line would have.
const HANDLE_REACH: f32 = 1.0 / 3.0;
/// Two keys closer together than this in time are the same key.
pub const SAME_TIME: f32 = 1e-4;

/// One animated number over time.
///
/// The keys are kept sorted by time by everything that adds to them, so
/// [`Track::value_at`] can walk them and a curve can be drawn straight from
/// the list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// What the track drives, as the application names it. Unique within an
    /// [`Animation`].
    pub id: String,
    /// What the track is called on screen.
    pub name: String,
    pub keys: Vec<Keyframe>,
    /// The value a track with no keys has, and where a new key starts.
    pub default: f32,
    /// How far the value is allowed to be dragged, and the vertical extent a
    /// curve editor fits the track into.
    pub range: (f32, f32),
    /// What the numbers are in, shown after them: `"\u{b0}"`, `"x"`, `"px"`.
    pub unit: String,
    /// How many decimals the value is shown to.
    pub decimals: u8,
    pub color: Color,
    /// Whether the curve is drawn and can be caught. A hidden track still
    /// drives its value.
    pub visible: bool,
    /// Whether the keys can be moved. A locked track is drawn faintly.
    pub locked: bool,
}

impl Track {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            keys: Vec::new(),
            default: 0.0,
            range: (0.0, 1.0),
            unit: String::new(),
            decimals: 2,
            color: [0.231, 0.510, 0.965, 1.0],
            visible: true,
            locked: false,
        }
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.range = if min <= max { (min, max) } else { (max, min) };
        self
    }

    pub fn with_default(mut self, value: f32) -> Self {
        self.default = value;
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    pub fn with_decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_keys(mut self, keys: impl IntoIterator<Item = Keyframe>) -> Self {
        for key in keys {
            self.set(key);
        }
        self
    }

    /// Clamp `value` to what the track allows.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.range.0, self.range.1)
    }

    /// The index of the key at `time`, within [`SAME_TIME`] of it.
    pub fn index_at(&self, time: f32) -> Option<usize> {
        self.keys
            .iter()
            .position(|key| (key.time - time).abs() <= SAME_TIME)
    }

    /// Put `key` in, replacing whatever was at its time. Gives back where it
    /// landed.
    ///
    /// Everything that adds keys goes through here, which is what keeps the
    /// list sorted.
    pub fn set(&mut self, key: Keyframe) -> usize {
        let key = Keyframe {
            value: self.clamp(key.value),
            ..key
        };
        if let Some(index) = self.index_at(key.time) {
            self.keys[index] = key;
            return index;
        }
        let index = self
            .keys
            .iter()
            .position(|existing| existing.time > key.time)
            .unwrap_or(self.keys.len());
        self.keys.insert(index, key);
        index
    }

    /// Key the track to `value` at `time`, keeping the shape of a key that is
    /// already there. Gives back where it landed.
    pub fn set_value(&mut self, time: f32, value: f32) -> usize {
        match self.index_at(time) {
            Some(index) => {
                self.keys[index].value = self.clamp(value);
                index
            }
            None => {
                // A new key carries on however the curve was shaped where it
                // was put, rather than resetting the segment it splits.
                let interpolation = self
                    .segment_at(time)
                    .map_or(Interpolation::default(), |index| {
                        self.keys[index].interpolation
                    });
                self.set(Keyframe::new(time, value).with_interpolation(interpolation))
            }
        }
    }

    /// Key the track to the value it already has at `time`, so the curve is
    /// pinned there without changing it.
    pub fn pin(&mut self, time: f32) -> usize {
        self.set_value(time, self.value_at(time))
    }

    pub fn remove(&mut self, index: usize) -> Option<Keyframe> {
        (index < self.keys.len()).then(|| self.keys.remove(index))
    }

    /// Move the key at `index` to a new time, sliding it past its neighbours
    /// if it is dragged over them. Gives back where it ended up.
    pub fn move_key(&mut self, index: usize, time: f32, value: f32) -> usize {
        let Some(key) = self.keys.get(index).copied() else {
            return index;
        };
        // Dropping a key exactly on another one would replace it, which is
        // not what dragging over a neighbour means. Nudge clear instead.
        let blocked = self
            .keys
            .iter()
            .enumerate()
            .any(|(i, other)| i != index && (other.time - time).abs() <= SAME_TIME);
        let time = if blocked {
            time + SAME_TIME * 2.0
        } else {
            time
        };
        self.keys.remove(index);
        self.set(Keyframe {
            time,
            value,
            ..key
        })
    }

    /// The index of the key the segment covering `time` starts from: the last
    /// one at or before it. None before the first key.
    pub fn segment_at(&self, time: f32) -> Option<usize> {
        self.keys
            .iter()
            .rposition(|key| key.time <= time + SAME_TIME)
            .filter(|index| *index + 1 < self.keys.len())
    }

    /// What the track is worth at `time`.
    ///
    /// Before the first key and after the last it holds that key's value, so
    /// a track is never undefined and a single key is a constant.
    pub fn value_at(&self, time: f32) -> f32 {
        let keys = &self.keys;
        match keys.first() {
            None => self.default,
            Some(first) if time <= first.time => first.value,
            Some(_) => {
                let last = keys[keys.len() - 1];
                if time >= last.time {
                    return last.value;
                }
                let index = keys
                    .iter()
                    .rposition(|key| key.time <= time)
                    .unwrap_or_default();
                self.segment(index).value_at(time)
            }
        }
    }

    /// The curve from the key at `index` to the one after it.
    ///
    /// Panics if there is no key after `index`; [`Track::segment_at`] only
    /// ever gives indices that have one.
    pub fn segment(&self, index: usize) -> Segment {
        let a = self.keys[index];
        let b = self.keys[index + 1];
        let reach = (b.time - a.time) * HANDLE_REACH;
        // Smooth aims each tangent at the key beyond the segment, so the
        // curve runs through the keys without a corner at them. Bezier takes
        // the handles as they were drawn, falling back to the straight line
        // while they have never been placed.
        let (out, incoming) = match a.interpolation {
            Interpolation::Smooth => (
                tangent(
                    slope(index.checked_sub(1).map_or(a, |i| self.keys[i]), b),
                    reach,
                    a.value,
                    b.value,
                ),
                tangent(
                    slope(a, self.keys.get(index + 2).copied().unwrap_or(b)),
                    -reach,
                    b.value,
                    a.value,
                ),
            ),
            Interpolation::Bezier => {
                let level = slope(a, b);
                (
                    placed(a.outgoing, level, reach),
                    placed(b.incoming, level, -reach),
                )
            }
            _ => (Handle::default(), Handle::default()),
        };
        Segment {
            from: a,
            to: b,
            out,
            incoming,
            mode: a.interpolation,
        }
    }

    /// Every segment's curve, sampled into `per_segment` points a piece, for
    /// drawing. A track with fewer than two keys gives nothing.
    pub fn sample(&self, per_segment: usize) -> Vec<(f32, f32)> {
        let steps = per_segment.max(1);
        let mut points = Vec::new();
        for index in 0..self.keys.len().saturating_sub(1) {
            let segment = self.segment(index);
            let span = segment.to.time - segment.from.time;
            for step in 0..=steps {
                let time = segment.from.time + span * step as f32 / steps as f32;
                points.push((time, segment.value_at(time)));
            }
        }
        points
    }

    /// The smallest and largest value the keys reach, which is what a curve
    /// editor fits a track to. None without keys.
    pub fn extent(&self) -> Option<(f32, f32)> {
        let mut bounds: Option<(f32, f32)> = None;
        for key in &self.keys {
            let (low, high) = bounds.unwrap_or((key.value, key.value));
            bounds = Some((low.min(key.value), high.max(key.value)));
        }
        bounds
    }
}

/// How steeply the line through two keys rises, per unit of time.
fn slope(before: Keyframe, after: Keyframe) -> f32 {
    let run = after.time - before.time;
    if run.abs() < SAME_TIME {
        0.0
    } else {
        (after.value - before.value) / run
    }
}

/// A tangent handle at a key worth `from`, leaving at `slope` and reaching
/// `reach` in time -- backwards, where that is negative.
///
/// Its control point is held between the two keys of the segment, which is
/// what keeps a smooth curve from swinging past the keys it joins.
fn tangent(slope: f32, reach: f32, from: f32, to: f32) -> Handle {
    let value = (from + slope * reach).clamp(from.min(to), from.max(to)) - from;
    Handle::new(reach, value)
}

/// A handle as it was drawn, or the straight line where it never was.
fn placed(handle: Handle, level: f32, reach: f32) -> Handle {
    if handle == Handle::default() {
        Handle::new(reach, level * reach)
    } else {
        handle
    }
}

/// The curve between two keys, ready to be asked for a value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub from: Keyframe,
    pub to: Keyframe,
    /// The first key's outgoing tangent, as the mode works it out.
    pub out: Handle,
    /// The second key's incoming tangent.
    pub incoming: Handle,
    pub mode: Interpolation,
}

impl Segment {
    /// The two control points of the cubic, in track coordinates.
    pub fn controls(&self) -> [(f32, f32); 2] {
        [
            (
                self.from.time + self.out.time,
                self.from.value + self.out.value,
            ),
            (
                self.to.time + self.incoming.time,
                self.to.value + self.incoming.value,
            ),
        ]
    }

    /// What the curve is worth at `time`, which is taken to be inside the
    /// segment; outside it the ends hold.
    pub fn value_at(&self, time: f32) -> f32 {
        let span = self.to.time - self.from.time;
        if span.abs() < SAME_TIME {
            return self.to.value;
        }
        let t = ((time - self.from.time) / span).clamp(0.0, 1.0);
        match self.mode {
            Interpolation::Hold => self.from.value,
            Interpolation::Linear => self.from.value + (self.to.value - self.from.value) * t,
            _ => {
                let [c0, c1] = self.controls();
                let x = [self.from.time, c0.0, c1.0, self.to.time];
                let y = [self.from.value, c0.1, c1.1, self.to.value];
                cubic(y, solve(x, time))
            }
        }
    }
}

/// A cubic Bezier through `p`, at parameter `t`.
fn cubic(p: [f32; 4], t: f32) -> f32 {
    let u = 1.0 - t;
    u * u * u * p[0] + 3.0 * u * u * t * p[1] + 3.0 * u * t * t * p[2] + t * t * t * p[3]
}

/// The parameter at which the cubic through `x` reaches `time`.
///
/// A Bezier is a curve in both axes, so the value at a time is not the value
/// at that fraction of the segment: the parameter has to be found first.
/// Bisection, because the control points can be dragged anywhere, including
/// places where the curve doubles back and Newton's method would wander off.
fn solve(x: [f32; 4], time: f32) -> f32 {
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let middle = 0.5 * (low + high);
        if cubic(x, middle) < time {
            low = middle;
        } else {
            high = middle;
        }
    }
    0.5 * (low + high)
}

/// Every track of one thing being animated, and how long it runs for.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub tracks: Vec<Track>,
}

impl Animation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn track(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    pub fn track_mut(&mut self, id: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|track| track.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.tracks.iter().position(|track| track.id == id)
    }

    /// Add `track`, replacing one with the same id.
    pub fn add(&mut self, track: Track) -> usize {
        match self.index_of(&track.id) {
            Some(index) => {
                self.tracks[index] = track;
                index
            }
            None => {
                self.tracks.push(track);
                self.tracks.len() - 1
            }
        }
    }

    pub fn remove(&mut self, id: &str) -> Option<Track> {
        self.index_of(id).map(|index| self.tracks.remove(index))
    }

    /// What the track `id` is worth at `time`, or None where nothing drives
    /// it and whatever holds the value should keep it.
    pub fn value_at(&self, id: &str, time: f32) -> Option<f32> {
        self.track(id).map(|track| track.value_at(time))
    }

    /// The last time any track has a key at. Nothing keyed gives zero.
    pub fn end(&self) -> f32 {
        self.tracks
            .iter()
            .filter_map(|track| track.keys.last())
            .fold(0.0, |end, key| end.max(key.time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(keys: &[(f32, f32)], interpolation: Interpolation) -> Track {
        Track::new("t", "T").with_range(-100.0, 100.0).with_keys(
            keys.iter()
                .map(|&(time, value)| Keyframe::new(time, value).with_interpolation(interpolation)),
        )
    }

    #[test]
    fn a_track_with_nothing_keyed_is_its_default() {
        let track = Track::new("t", "T").with_default(7.0);
        assert_eq!(track.value_at(0.0), 7.0);
        assert_eq!(track.value_at(100.0), 7.0);
    }

    #[test]
    fn one_key_holds_the_same_value_for_ever() {
        let track = track(&[(2.0, 5.0)], Interpolation::Linear);
        assert_eq!(track.value_at(-10.0), 5.0);
        assert_eq!(track.value_at(2.0), 5.0);
        assert_eq!(track.value_at(10.0), 5.0);
    }

    #[test]
    fn a_straight_segment_is_the_line_between_its_keys() {
        let track = track(&[(0.0, 0.0), (10.0, 100.0)], Interpolation::Linear);
        assert!((track.value_at(2.5) - 25.0).abs() < 1e-3);
        assert!((track.value_at(7.5) - 75.0).abs() < 1e-3);
    }

    #[test]
    fn a_held_segment_does_not_move_until_the_next_key() {
        let track = track(&[(0.0, 0.0), (10.0, 100.0)], Interpolation::Hold);
        assert_eq!(track.value_at(9.99), 0.0);
        assert_eq!(track.value_at(10.0), 100.0);
    }

    #[test]
    fn every_curve_starts_and_ends_on_its_keys() {
        for mode in Interpolation::ALL {
            let track = track(&[(0.0, 10.0), (4.0, -30.0), (9.0, 60.0)], mode);
            for (time, value) in [(0.0, 10.0), (4.0, -30.0), (9.0, 60.0)] {
                let got = track.value_at(time);
                assert!((got - value).abs() < 1e-3, "{} at {time}: {got}", mode.name());
            }
        }
    }

    #[test]
    fn a_smooth_curve_eases_out_of_a_key_rather_than_leaving_it_straight() {
        let straight = track(&[(0.0, 0.0), (10.0, 100.0), (20.0, 0.0)], Interpolation::Linear);
        let smooth = track(&[(0.0, 0.0), (10.0, 100.0), (20.0, 0.0)], Interpolation::Smooth);
        // Coming up to a peak, easing is above the straight line.
        assert!(smooth.value_at(8.0) > straight.value_at(8.0));
        // And it does not overshoot past the peak itself.
        assert!(smooth.value_at(9.9) <= 100.0 + 1e-3);
    }

    #[test]
    fn a_bezier_segment_follows_the_handles_it_is_given() {
        let mut track = track(&[(0.0, 0.0), (10.0, 100.0)], Interpolation::Bezier);
        // Both handles flat: the value leaves and arrives level, so half way
        // through it is still half, but a quarter in it has barely moved.
        track.keys[0].outgoing = Handle::new(5.0, 0.0);
        track.keys[1].incoming = Handle::new(-5.0, 0.0);
        assert!((track.value_at(5.0) - 50.0).abs() < 1.0);
        assert!(track.value_at(2.5) < 20.0);
        assert!(track.value_at(7.5) > 80.0);
    }

    #[test]
    fn keys_stay_in_time_order_however_they_are_put_in() {
        let mut track = Track::new("t", "T").with_range(0.0, 10.0);
        for time in [5.0, 1.0, 9.0, 3.0] {
            track.set(Keyframe::new(time, 1.0));
        }
        let times: Vec<f32> = track.keys.iter().map(|key| key.time).collect();
        assert_eq!(times, vec![1.0, 3.0, 5.0, 9.0]);
    }

    #[test]
    fn keying_the_same_time_twice_replaces_rather_than_doubles() {
        let mut track = Track::new("t", "T").with_range(0.0, 10.0);
        track.set_value(2.0, 3.0);
        track.set_value(2.0, 8.0);
        assert_eq!(track.keys.len(), 1);
        assert_eq!(track.keys[0].value, 8.0);
    }

    #[test]
    fn a_key_is_never_worth_more_than_the_track_allows() {
        let mut track = Track::new("t", "T").with_range(0.0, 1.0);
        track.set_value(0.0, 40.0);
        track.set_value(1.0, -40.0);
        assert_eq!(track.keys[0].value, 1.0);
        assert_eq!(track.keys[1].value, 0.0);
    }

    #[test]
    fn a_key_dragged_past_its_neighbour_takes_its_place_in_the_order() {
        let mut track = track(&[(0.0, 0.0), (5.0, 50.0), (10.0, 100.0)], Interpolation::Linear);
        let landed = track.move_key(0, 7.0, 70.0);
        assert_eq!(landed, 1);
        let times: Vec<f32> = track.keys.iter().map(|key| key.time).collect();
        assert_eq!(times, vec![5.0, 7.0, 10.0]);
    }

    #[test]
    fn a_key_dropped_on_another_one_does_not_swallow_it() {
        let mut track = track(&[(0.0, 0.0), (5.0, 50.0)], Interpolation::Linear);
        track.move_key(0, 5.0, 10.0);
        assert_eq!(track.keys.len(), 2);
    }

    #[test]
    fn pinning_leaves_the_curve_exactly_where_it_was() {
        let track = track(&[(0.0, 0.0), (10.0, 100.0)], Interpolation::Smooth);
        let before: Vec<f32> = (0..=20).map(|i| track.value_at(i as f32 * 0.5)).collect();
        let mut pinned = track.clone();
        pinned.pin(3.0);
        assert_eq!(pinned.keys.len(), 3);
        for (i, was) in before.into_iter().enumerate() {
            let now = pinned.value_at(i as f32 * 0.5);
            assert!((now - was).abs() < 2.0, "at {}: {was} became {now}", i as f32 * 0.5);
        }
    }

    #[test]
    fn a_sampled_curve_runs_from_the_first_key_to_the_last() {
        let track = track(&[(1.0, 0.0), (4.0, 10.0), (6.0, 2.0)], Interpolation::Smooth);
        let points = track.sample(8);
        assert_eq!(points.first().map(|p| p.0), Some(1.0));
        assert_eq!(points.last().map(|p| p.0), Some(6.0));
        assert!(points.windows(2).all(|pair| pair[1].0 >= pair[0].0));
    }

    #[test]
    fn an_animation_answers_only_for_the_tracks_it_has() {
        let mut animation = Animation::new();
        animation.add(track(&[(0.0, 1.0), (1.0, 2.0)], Interpolation::Linear));
        assert_eq!(animation.value_at("t", 0.0), Some(1.0));
        assert_eq!(animation.value_at("missing", 0.0), None);
        assert_eq!(animation.end(), 1.0);
    }

    #[test]
    fn adding_a_track_twice_replaces_it_rather_than_leaving_two() {
        let mut animation = Animation::new();
        animation.add(Track::new("t", "First"));
        animation.add(Track::new("t", "Second"));
        assert_eq!(animation.tracks.len(), 1);
        assert_eq!(animation.tracks[0].name, "Second");
        assert!(animation.remove("t").is_some());
        assert!(animation.is_empty());
    }
}
