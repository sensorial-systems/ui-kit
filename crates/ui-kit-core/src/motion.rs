//! Motion as a pure function of time: springs solved in closed form, and
//! values that change target many times without remembering anything
//! between frames.
//!
//! `Ui::animate` in the immediate layer eases from wherever a
//! value was last frame, so it has to be driven frame by frame, in order. A
//! [`SpringTrack`] is a list of target changes known up front, and its value
//! at any time is the sum of one spring step per change: seeking backwards,
//! rendering frames out of order or twice, and splitting a render across
//! machines all give the same pixels.
//!
//! A track can also [loop](SpringTrack::looped): the springs still settling
//! from the end of one lap carry on into the start of the next, so the last
//! frame leads into the first with the same position and speed.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// A damped spring, described the way a designer tunes one.
///
/// `response` is the period of the undamped oscillation, in seconds: roughly
/// how long the spring takes to get where it is going. `damping` is the
/// damping ratio: 1 arrives without passing the target, a little under 1
/// passes it by a hair and comes back, which reads as weight rather than
/// bounce.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spring {
    pub response: f32,
    pub damping: f32,
}

impl Spring {
    pub const fn new(response: f32, damping: f32) -> Self {
        Self { response, damping }
    }

    /// Quick and precise, for things the pointer moves: overshoots by well
    /// under one percent.
    pub const SNAPPY: Self = Self::new(0.3, 0.86);
    /// Unhurried and exact, for the camera and large moves.
    pub const SMOOTH: Self = Self::new(0.5, 1.0);
    /// Fast with no overshoot, for fades and small details.
    pub const QUICK: Self = Self::new(0.18, 1.0);
    /// The edge that follows behind a stretching shape.
    pub const TRAILING: Self = Self::new(0.42, 0.9);
    /// The edge that leads a stretching shape.
    pub const LEADING: Self = Self::new(0.22, 0.88);

    fn omega(self) -> f32 {
        TAU / self.response.max(1e-4)
    }

    /// Where a spring released at rest from 0 towards 1 is, `t` seconds
    /// later. 0 before it starts.
    pub fn step(self, t: f32) -> f32 {
        if t <= 0.0 {
            return 0.0;
        }
        let w = self.omega();
        let z = self.damping.max(0.0);
        if z < 1.0 - 1e-4 {
            let wd = w * (1.0 - z * z).sqrt();
            let decay = (-z * w * t).exp();
            1.0 - decay * ((wd * t).cos() + z * w / wd * (wd * t).sin())
        } else if z <= 1.0 + 1e-4 {
            1.0 - (-w * t).exp() * (1.0 + w * t)
        } else {
            let root = (z * z - 1.0).sqrt();
            let (r1, r2) = (-w * (z - root), -w * (z + root));
            1.0 + (r2 * (r1 * t).exp() - r1 * (r2 * t).exp()) / (r1 - r2)
        }
    }

    /// How fast [`step`](Spring::step) is changing at `t`, per second.
    pub fn step_velocity(self, t: f32) -> f32 {
        if t <= 0.0 {
            return 0.0;
        }
        // The step response's derivative is the impulse response scaled by w².
        let w = self.omega();
        w * w * self.impulse(t)
    }

    /// Where a spring at rest on its target is, `t` seconds after being
    /// pushed with a speed of 1 per second. It swings out and settles back.
    pub fn impulse(self, t: f32) -> f32 {
        if t <= 0.0 {
            return 0.0;
        }
        let w = self.omega();
        let z = self.damping.max(0.0);
        if z < 1.0 - 1e-4 {
            let wd = w * (1.0 - z * z).sqrt();
            (-z * w * t).exp() * (wd * t).sin() / wd
        } else if z <= 1.0 + 1e-4 {
            t * (-w * t).exp()
        } else {
            let root = (z * z - 1.0).sqrt();
            let (r1, r2) = (-w * (z - root), -w * (z + root));
            ((r1 * t).exp() - (r2 * t).exp()) / (r1 - r2)
        }
    }

    /// How fast [`impulse`](Spring::impulse) is changing at `t`.
    pub fn impulse_velocity(self, t: f32) -> f32 {
        if t <= 0.0 {
            return 0.0;
        }
        let w = self.omega();
        let z = self.damping.max(0.0);
        if z < 1.0 - 1e-4 {
            let wd = w * (1.0 - z * z).sqrt();
            let decay = (-z * w * t).exp();
            decay * ((wd * t).cos() - z * w / wd * (wd * t).sin())
        } else if z <= 1.0 + 1e-4 {
            (-w * t).exp() * (1.0 - w * t)
        } else {
            let root = (z * z - 1.0).sqrt();
            let (r1, r2) = (-w * (z - root), -w * (z + root));
            (r1 * (r1 * t).exp() - r2 * (r2 * t).exp()) / (r1 - r2)
        }
    }

    /// How far past its target the spring swings, as a fraction of the move.
    pub fn overshoot(self) -> f32 {
        if self.damping >= 1.0 {
            0.0
        } else {
            let z = self.damping.max(0.0);
            (-std::f32::consts::PI * z / (1.0 - z * z).sqrt()).exp()
        }
    }

    /// Roughly how long until the spring is within a thousandth of its
    /// target and stays there.
    pub fn settle_time(self) -> f32 {
        let z = self.damping.clamp(0.05, 1.0);
        (1000.0f32).ln() / (z * self.omega())
            + if self.damping >= 1.0 {
                self.response
            } else {
                0.0
            }
    }
}

/// What happens at one moment of a [`SpringTrack`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
enum Change {
    /// Spring from the previous target to one `delta` away.
    Spring {
        time: f32,
        delta: f32,
        spring: Spring,
    },
    /// Jump by `delta` at once: where a value that was being dragged is let go.
    Jump { time: f32, delta: f32 },
    /// Push with a speed: a drag let go while moving carries on a little.
    Kick {
        time: f32,
        velocity: f32,
        spring: Spring,
    },
}

impl Change {
    fn time(&self) -> f32 {
        match *self {
            Change::Spring { time, .. } | Change::Jump { time, .. } | Change::Kick { time, .. } => {
                time
            }
        }
    }

    fn value(&self, t: f32) -> f32 {
        match *self {
            Change::Spring {
                time,
                delta,
                spring,
            } => delta * spring.step(t - time),
            Change::Jump { time, delta } => {
                if t >= time {
                    delta
                } else {
                    0.0
                }
            }
            Change::Kick {
                time,
                velocity,
                spring,
            } => velocity * spring.impulse(t - time),
        }
    }

    fn velocity(&self, t: f32) -> f32 {
        match *self {
            Change::Spring {
                time,
                delta,
                spring,
            } => delta * spring.step_velocity(t - time),
            Change::Jump { .. } => 0.0,
            Change::Kick {
                time,
                velocity,
                spring,
            } => velocity * spring.impulse_velocity(t - time),
        }
    }
}

/// A number that springs to a new target each time one is set, as a pure
/// function of time.
///
/// ```
/// use ui_kit_core::motion::{Spring, SpringTrack};
/// let width = SpringTrack::new(100.0)
///     .to(1.0, 240.0, Spring::SNAPPY)
///     .to(2.0, 100.0, Spring::SNAPPY);
/// assert_eq!(width.value(0.5), 100.0);
/// assert!((width.value(1.9) - 240.0).abs() < 1.0);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpringTrack {
    start: f32,
    target: f32,
    changes: Vec<Change>,
    period: Option<f32>,
}

/// How many laps before the current one still move a looped track. Springs
/// in this module settle in well under a lap of any reel worth looping.
const LAPS_BEHIND: usize = 2;

impl SpringTrack {
    pub fn new(start: f32) -> Self {
        Self {
            start,
            target: start,
            changes: vec![],
            period: None,
        }
    }

    /// The target set last, where the track comes to rest.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// The value before anything happens.
    pub fn start(&self) -> f32 {
        self.start
    }

    fn push(&mut self, change: Change) {
        debug_assert!(
            self.changes
                .last()
                .map_or(true, |last| last.time() <= change.time()),
            "changes are added in time order"
        );
        self.changes.push(change);
    }

    /// From `time`, spring towards `target`.
    pub fn to(mut self, time: f32, target: f32, spring: Spring) -> Self {
        self.set_to(time, target, spring);
        self
    }

    /// [`to`](SpringTrack::to), in place.
    pub fn set_to(&mut self, time: f32, target: f32, spring: Spring) {
        let delta = target - self.target;
        self.target = target;
        if delta != 0.0 {
            self.push(Change::Spring {
                time,
                delta,
                spring,
            });
        }
    }

    /// At `time`, be `value` at once. With [`to`](SpringTrack::to) at the
    /// same time, a dragged value let go springs back from where it was.
    pub fn jump(mut self, time: f32, value: f32) -> Self {
        let delta = value - self.target;
        self.target = value;
        if delta != 0.0 {
            self.push(Change::Jump { time, delta });
        }
        self
    }

    /// At `time`, push the value with `velocity` per second; it swings and
    /// settles back on the same target.
    pub fn kick(mut self, time: f32, velocity: f32, spring: Spring) -> Self {
        if velocity != 0.0 {
            self.push(Change::Kick {
                time,
                velocity,
                spring,
            });
        }
        self
    }

    /// Repeat every `period` seconds. The track has to end where it started,
    /// or the loop would jump.
    pub fn looped(mut self, period: f32) -> Self {
        debug_assert!(
            (self.target - self.start).abs() < 1e-3,
            "a looped track ends on its start ({} != {})",
            self.target,
            self.start
        );
        self.period = Some(period);
        self
    }

    /// Whether the track ends where it started, so it can loop.
    pub fn is_closed(&self) -> bool {
        (self.target - self.start).abs() < 1e-3
    }

    fn sum(&self, t: f32, term: impl Fn(&Change, f32) -> f32) -> f32 {
        match self.period {
            None => self.changes.iter().map(|c| term(c, t)).sum(),
            Some(period) => {
                let t = t.rem_euclid(period);
                // Earlier laps: their changes have all started, and once a
                // spring has settled its whole delta counts. Over a closed lap
                // the deltas cancel, so only what is still moving remains.
                (0..=LAPS_BEHIND)
                    .map(|lap| {
                        let t = t + lap as f32 * period;
                        self.changes.iter().map(|c| term(c, t)).sum::<f32>()
                    })
                    .sum()
            }
        }
    }

    /// The value at `t` seconds.
    pub fn value(&self, t: f32) -> f32 {
        self.start + self.sum(t, Change::value)
    }

    /// How fast the value is changing at `t`, per second.
    pub fn velocity(&self, t: f32) -> f32 {
        self.sum(t, Change::velocity)
    }
}

/// Several states a thing can be in, crossfaded by springs: the weight of
/// each at any time, summing to one.
///
/// Every switch springs one weight up and the previous one down with the
/// same spring, so the weights always add up to one and a shape drawn as
/// their weighted mix is a continuous morph from one state's geometry to the
/// next -- even when each state's own geometry is moving.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blend {
    weights: Vec<SpringTrack>,
    current: usize,
}

impl Blend {
    /// `count` states, starting fully in `first`.
    pub fn new(count: usize, first: usize) -> Self {
        Self {
            weights: (0..count)
                .map(|i| SpringTrack::new(if i == first { 1.0 } else { 0.0 }))
                .collect(),
            current: first,
        }
    }

    /// From `time`, move into `state`.
    pub fn to(mut self, time: f32, state: usize, spring: Spring) -> Self {
        if state != self.current {
            self.weights[self.current].set_to(time, 0.0, spring);
            self.weights[state].set_to(time, 1.0, spring);
            self.current = state;
        }
        self
    }

    pub fn looped(mut self, period: f32) -> Self {
        self.weights = self.weights.into_iter().map(|w| w.looped(period)).collect();
        self
    }

    /// Each state's weight at `t`. The weights sum to one.
    pub fn weights(&self, t: f32) -> Vec<f32> {
        self.weights.iter().map(|w| w.value(t)).collect()
    }

    /// The weight of `state` at `t`.
    pub fn weight(&self, state: usize, t: f32) -> f32 {
        self.weights[state].value(t)
    }
}

/// Beats per minute and where the first downbeat falls, for placing things
/// on a song's grid.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeatGrid {
    pub bpm: f32,
    /// Seconds from the start to the first downbeat.
    pub offset: f32,
    pub beats_per_bar: u32,
}

impl Default for BeatGrid {
    fn default() -> Self {
        Self::new(120.0)
    }
}

impl BeatGrid {
    pub fn new(bpm: f32) -> Self {
        Self {
            bpm,
            offset: 0.0,
            beats_per_bar: 4,
        }
    }

    pub fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    /// Seconds per beat.
    pub fn beat_length(&self) -> f32 {
        60.0 / self.bpm
    }

    /// The time of `beat`, counted from the first downbeat; fractions fall
    /// between beats.
    pub fn time(&self, beat: f32) -> f32 {
        self.offset + beat * self.beat_length()
    }

    /// The beat at `time`, with its fraction.
    pub fn beat(&self, time: f32) -> f32 {
        (time - self.offset) / self.beat_length()
    }

    /// How long `beats` last.
    pub fn duration(&self, beats: f32) -> f32 {
        beats * self.beat_length()
    }
}

/// How far a value dragged `excess` past its limit is let go, resisting
/// more the further it is pulled: never more than `limit`.
pub fn rubber_band(excess: f32, limit: f32) -> f32 {
    if excess <= 0.0 || limit <= 0.0 {
        return 0.0;
    }
    (1.0 - 1.0 / (excess * 0.55 / limit + 1.0)) * limit
}

/// `a` to `b` by `t`.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn a_step_starts_at_rest_and_arrives() {
        for spring in [
            Spring::SNAPPY,
            Spring::SMOOTH,
            Spring::QUICK,
            Spring::new(0.4, 1.6),
        ] {
            assert_eq!(spring.step(0.0), 0.0);
            assert_eq!(spring.step_velocity(0.0), 0.0);
            assert!(close(spring.step(1e-3), 0.0, 1e-3));
            assert!(
                close(spring.step(spring.settle_time() + 1.0), 1.0, 1e-3),
                "{spring:?}"
            );
        }
    }

    #[test]
    fn the_velocity_is_the_slope_of_the_step() {
        for spring in [Spring::SNAPPY, Spring::SMOOTH, Spring::new(0.4, 1.6)] {
            for t in [0.02, 0.1, 0.25, 0.6] {
                let h = 1e-3;
                let slope = (spring.step(t + h) - spring.step(t - h)) / (2.0 * h);
                assert!(close(
                    slope,
                    spring.step_velocity(t),
                    0.05 * slope.abs().max(1.0)
                ));
                let slope = (spring.impulse(t + h) - spring.impulse(t - h)) / (2.0 * h);
                assert!(close(
                    slope,
                    spring.impulse_velocity(t),
                    0.05 * slope.abs().max(1.0)
                ));
            }
        }
    }

    #[test]
    fn the_house_springs_overshoot_by_a_hair_at_most() {
        for spring in [Spring::SNAPPY, Spring::LEADING, Spring::TRAILING] {
            let peak = (0..2000)
                .map(|i| spring.step(i as f32 * 0.001))
                .fold(0.0, f32::max);
            assert!(peak - 1.0 < 0.02, "{spring:?} overshoots by {}", peak - 1.0);
            assert!(close(peak - 1.0, spring.overshoot(), 0.002));
        }
    }

    #[test]
    fn a_track_is_a_pure_function_of_time() {
        let track =
            SpringTrack::new(0.0)
                .to(0.5, 10.0, Spring::SNAPPY)
                .to(0.7, -4.0, Spring::SMOOTH);
        let forwards: Vec<f32> = (0..100).map(|i| track.value(i as f32 * 0.02)).collect();
        let backwards: Vec<f32> = (0..100)
            .rev()
            .map(|i| track.value(i as f32 * 0.02))
            .collect();
        assert!(forwards.iter().eq(backwards.iter().rev()));
        assert_eq!(track.value(0.4), 0.0);
        assert!(close(track.value(5.0), -4.0, 1e-3));
    }

    #[test]
    fn a_retarget_mid_flight_keeps_position_and_speed() {
        let track =
            SpringTrack::new(0.0)
                .to(0.0, 100.0, Spring::SNAPPY)
                .to(0.1, 0.0, Spring::SNAPPY);
        // Both change fast here, so compare across a small step.
        let h = 1e-4;
        assert!(close(track.value(0.1 - h), track.value(0.1 + h), 0.5));
        assert!(close(
            track.velocity(0.1 - h),
            track.velocity(0.1 + h),
            15.0
        ));
    }

    #[test]
    fn a_released_drag_springs_back_from_where_it_was_let_go() {
        let released_at = 40.0;
        let track = SpringTrack::new(0.0)
            .jump(1.0, released_at)
            .to(1.0, 0.0, Spring::SNAPPY);
        assert_eq!(track.value(0.99), 0.0);
        assert!(close(track.value(1.0), released_at, 1e-3));
        assert!(track.value(1.1) < released_at && track.value(1.1) > 0.0);
        assert!(close(track.value(3.0), 0.0, 0.05));
    }

    #[test]
    fn a_loop_meets_itself_in_value_and_speed() {
        let period = 4.0;
        let track = SpringTrack::new(0.0)
            .to(0.5, 1.0, Spring::SNAPPY)
            .to(3.8, 0.0, Spring::SMOOTH)
            .looped(period);
        // The spring started at 3.8 is still settling at the seam.
        let h = 1e-3;
        assert!(close(track.value(period - h), track.value(0.0), 0.01));
        assert!(close(track.velocity(period - h), track.velocity(0.0), 0.05));
        assert!(
            track.value(0.0).abs() > 1e-3,
            "the previous lap still moves the start"
        );
        assert!(close(track.value(0.3), track.value(0.3 + period), 1e-4));
    }

    #[test]
    fn blend_weights_always_sum_to_one() {
        let blend = Blend::new(3, 0)
            .to(0.2, 1, Spring::SNAPPY)
            .to(0.3, 2, Spring::SNAPPY)
            .to(0.9, 0, Spring::SMOOTH);
        for i in 0..60 {
            let sum: f32 = blend.weights(i as f32 * 0.03).iter().sum();
            assert!(close(sum, 1.0, 1e-4));
        }
    }

    #[test]
    fn a_rubber_band_resists_and_never_passes_its_limit() {
        assert_eq!(rubber_band(-5.0, 40.0), 0.0);
        let a = rubber_band(20.0, 40.0);
        let b = rubber_band(80.0, 40.0);
        assert!(a > 0.0 && a < 20.0 && b > a && b < 40.0);
    }

    #[test]
    fn a_beat_grid_places_beats_after_the_downbeat() {
        let grid = BeatGrid::new(120.0).with_offset(0.25);
        assert_eq!(grid.beat_length(), 0.5);
        assert_eq!(grid.time(4.0), 2.25);
        assert_eq!(grid.beat(2.25), 4.0);
    }
}
