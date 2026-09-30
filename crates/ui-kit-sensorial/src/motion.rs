//! Motion: springs everything moves by, stepped a frame at a time, and the
//! curves the one-off moves follow.
//!
//! What moves in the look moves by one of a few springs ([`HOVER`],
//! [`PRESS`], [`ENTRANCE`], [`REVEAL`]), so a thing reacts the same way
//! wherever it is.

use std::collections::HashMap;

/// How a spring moves: `response` is roughly how long it takes to get
/// there, in seconds, and `damping` how much it may overshoot -- 1 not at
/// all, 0.7 a little, as a thing thrown and caught.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feel {
    pub response: f32,
    pub damping: f32,
}

/// Lighting up under the pointer: quick, with a little bounce.
pub const HOVER: Feel = Feel {
    response: 0.32,
    damping: 0.72,
};
/// Giving under a press: quicker, and not at all.
pub const PRESS: Feel = Feel {
    response: 0.14,
    damping: 1.0,
};
/// Coming in as a screen opens.
pub const ENTRANCE: Feel = Feel {
    response: 0.85,
    damping: 0.86,
};
/// Rising in as it scrolls into view.
pub const REVEAL: Feel = Feel {
    response: 0.9,
    damping: 0.84,
};

/// A value pulled towards a target as by a spring.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
}

impl Spring {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    pub fn step(&mut self, target: f32, dt: f32, response: f32, damping: f32) -> f32 {
        let omega = std::f32::consts::TAU / response.max(1e-3);
        let stiffness = omega * omega;
        let friction = 2.0 * damping * omega;
        // Small steps keep a stiff spring steady whatever the frame rate.
        let steps = (dt / (1.0 / 240.0)).ceil().clamp(1.0, 16.0);
        let h = dt / steps;
        for _ in 0..steps as usize {
            let force = -stiffness * (self.value - target) - friction * self.velocity;
            self.velocity += force * h;
            self.value += self.velocity * h;
        }
        if (self.value - target).abs() < 1e-4 && self.velocity.abs() < 1e-3 {
            self.value = target;
            self.velocity = 0.0;
        }
        self.value
    }

    pub fn is_moving(&self) -> bool {
        self.velocity != 0.0
    }
}

/// Springs by name, each stepped at most once a frame, created at their
/// first target (or at `from`).
#[derive(Default)]
pub struct Springs {
    springs: HashMap<String, (Spring, u64)>,
    frame: u64,
    dt: f32,
}

impl Springs {
    /// Start a frame `dt` seconds after the last.
    pub fn begin(&mut self, dt: f32) {
        self.frame += 1;
        self.dt = dt;
    }

    fn get(&mut self, id: &str, from: f32) -> &mut (Spring, u64) {
        if !self.springs.contains_key(id) {
            self.springs.insert(id.to_owned(), (Spring::new(from), 0));
        }
        self.springs.get_mut(id).expect("inserted above")
    }

    /// The value of the spring `id` pulled towards `target`, starting from
    /// `from` the first time it is asked for.
    pub fn go(&mut self, id: &str, from: f32, target: f32, response: f32, damping: f32) -> f32 {
        let (frame, dt) = (self.frame, self.dt);
        let (spring, stepped) = self.get(id, from);
        if *stepped != frame {
            *stepped = frame;
            spring.step(target, dt, response, damping);
        }
        spring.value
    }

    /// [`go`](Self::go), starting at the target: a value that follows.
    pub fn follow(&mut self, id: &str, target: f32, response: f32, damping: f32) -> f32 {
        self.go(id, target, target, response, damping)
    }

    /// [`go`](Self::go) as `feel` moves.
    pub fn to(&mut self, id: &str, from: f32, target: f32, feel: Feel) -> f32 {
        self.go(id, from, target, feel.response, feel.damping)
    }

    /// [`follow`](Self::follow) as `feel` moves.
    pub fn track(&mut self, id: &str, target: f32, feel: Feel) -> f32 {
        self.follow(id, target, feel.response, feel.damping)
    }

    /// Whether any spring is still on its way, so the next frame should
    /// be drawn soon.
    pub fn moving(&self) -> bool {
        self.springs.values().any(|(spring, _)| spring.is_moving())
    }

    /// Forget every spring but those `keep` says to keep.
    pub fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.springs.retain(|id, _| keep(id));
    }

    pub fn clear(&mut self) {
        self.springs.clear();
    }
}

/// Where `x` is between `a` and `b`, 0 to 1.
pub fn progress(x: f32, a: f32, b: f32) -> f32 {
    ((x - a) / (b - a)).clamp(0.0, 1.0)
}

pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) * 0.5
    }
}

/// Frame-rate independent approach of `current` to `target`: the share of
/// the way left after one second is `remaining`.
pub fn approach(current: f32, target: f32, remaining: f32, dt: f32) -> f32 {
    target + (current - target) * remaining.powf(dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spring_settles_on_its_target() {
        let mut springs = Springs::default();
        for _ in 0..240 {
            springs.begin(1.0 / 60.0);
            springs.track("x", 1.0, HOVER);
            springs.to("y", 0.0, 1.0, ENTRANCE);
        }
        assert_eq!(springs.track("x", 1.0, HOVER), 1.0);
        assert_eq!(springs.to("y", 0.0, 1.0, ENTRANCE), 1.0);
        assert!(!springs.moving());
    }

    #[test]
    fn a_spring_is_stepped_once_a_frame() {
        let mut springs = Springs::default();
        springs.begin(1.0 / 60.0);
        let first = springs.to("x", 0.0, 1.0, HOVER);
        assert_eq!(springs.to("x", 0.0, 1.0, HOVER), first);
        assert!(springs.moving());
    }
}
