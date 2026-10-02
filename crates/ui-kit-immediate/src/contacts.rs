//! Every finger and the mouse as contacts, each bound to what it touched
//! down on until it lifts.
//!
//! [`Input`] says where the fingers and the pointer are at each step. A
//! control driven by hand needs more: which contact is which across steps
//! (a mouse press is not a finger), what each one touched down on, and
//! whether lifting it ends a tap or a hold. Worked out per application,
//! these go wrong in the same few ways: a contact counted twice, a release
//! acting on whatever happens to be under the finger by then, a hold read as
//! a tap. [`Contacts`] does it once:
//!
//! - **Identity.** Each finger is its own contact, and the mouse is one
//!   while its button is held and no finger drives the pointer.
//! - **Capture.** A contact is hit-tested once, where it touched down, and
//!   keeps that target until it lifts, wherever it goes meanwhile: a finger
//!   lifted over another control never acts on it, nor does one slid onto it.
//! - **Taps.** A release says whether it ended a tap (short and still,
//!   by [`Tap`]), so a hold, however it ends, is never one.
//! - **Inert contacts.** After [`Contacts::clear`], everything down at that
//!   moment, seen yet or not, does nothing until it lifts: a finger that
//!   opened a screen does not press what is under it on the next.
//!
//! ```
//! # use ui_kit_immediate::{Contacts, ContactEvent, Input, Tap, Point};
//! #[derive(Clone, Copy, PartialEq, Debug)]
//! enum Control { Button }
//! let mut contacts = Contacts::new(Tap::default());
//! let mut input = Input { pointer: Some(Point::new(5.0, 5.0)), down: true, ..Input::default() };
//! input.time = Some(0.0);
//! let hit = |_: Point| Some(Control::Button);
//! assert!(matches!(contacts.update(&input, hit)[..], [ContactEvent::Pressed { .. }]));
//! input.down = false;
//! input.time = Some(0.1);
//! assert!(matches!(contacts.update(&input, hit)[..], [ContactEvent::Released { tap: true, .. }]));
//! ```

use prism_ui::immediate::{Input, Vec2};

/// Which device a contact is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContactId {
    /// The mouse (or a pen), its primary button held.
    Mouse,
    /// A finger, by its [`Touch::id`](prism_ui::immediate::Touch::id).
    Touch(u64),
}

/// What makes a release a tap: down for less than `seconds`, and never
/// further than `travel` from where it touched down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap {
    pub seconds: f64,
    /// In the input's coordinates (logical pixels).
    pub travel: f32,
}

impl Default for Tap {
    /// A third of a second, and a finger's slop.
    fn default() -> Self {
        Self { seconds: 0.3, travel: 14.0 }
    }
}

/// A contact down on a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact<T> {
    pub id: ContactId,
    /// What it touched down on.
    pub target: T,
    /// Where it is now, in the input's coordinates.
    pub position: Vec2,
    /// Where it touched down.
    pub start: Vec2,
    /// When it touched down, in the input's seconds.
    pub since: f64,
    /// How hard a finger presses, where the screen says (see
    /// [`Input::pressure_sensing`]).
    pub force: Option<f32>,
    /// Whether it ever went further than the tap's travel from `start`.
    pub moved: bool,
}

impl<T> Contact<T> {
    /// How long it has been down at `time`.
    pub fn held(&self, time: f64) -> f64 {
        time - self.since
    }
}

/// What a step did to the contacts, in the order to act on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContactEvent<T> {
    /// A contact touched down on `target`.
    Pressed { id: ContactId, target: T, at: Vec2 },
    /// A contact lifted, at `at`, from the `target` it touched down on.
    /// `tap` if it was short and still.
    Released { id: ContactId, target: T, at: Vec2, tap: bool },
}

/// The contacts down now, followed from step to step. See the module.
#[derive(Clone, Debug)]
pub struct Contacts<T> {
    /// Every contact down, its target None where it touched nothing or was
    /// made inert.
    followed: Vec<Contact<Option<T>>>,
    tap: Tap,
    /// Everything down at the next step is inert, seen before or not.
    inert: bool,
}

impl<T: Copy> Contacts<T> {
    pub fn new(tap: Tap) -> Self {
        Self { followed: Vec::new(), tap, inert: false }
    }

    /// Follows one step of input. A contact that appeared is hit-tested at
    /// once, where it touched down (`hit` returns None for nothing there:
    /// it is followed, but does nothing until it lifts); one that lifted is
    /// released. Releases come before presses.
    pub fn update(
        &mut self,
        input: &Input,
        mut hit: impl FnMut(Vec2) -> Option<T>,
    ) -> Vec<ContactEvent<T>> {
        let time = input.time.unwrap_or(0.0);
        let present = current(input);
        let mut events = Vec::new();

        // Lifted: released where they last were.
        let mut kept = Vec::with_capacity(self.followed.len());
        for f in self.followed.drain(..) {
            if present.iter().any(|(id, ..)| *id == f.id) {
                kept.push(f);
                continue;
            }
            if let Some(target) = f.target {
                let c = &f;
                let tap = !c.moved && c.held(time) < self.tap.seconds;
                events.push(ContactEvent::Released { id: c.id, target, at: c.position, tap });
            }
        }
        self.followed = kept;

        // Moved, or new.
        for (id, position, force) in present {
            match self.followed.iter_mut().find(|f| f.id == id) {
                Some(c) => {
                    c.position = position;
                    c.force = force;
                    c.moved |= distance(position, c.start) > self.tap.travel;
                }
                None => {
                    let target = if self.inert { None } else { hit(position) };
                    if let Some(target) = target {
                        events.push(ContactEvent::Pressed { id, target, at: position });
                    }
                    self.followed.push(Contact {
                        id,
                        target,
                        position,
                        start: position,
                        since: time,
                        force,
                        moved: false,
                    });
                }
            }
        }
        self.inert = false;
        events
    }

    /// The contacts down on a target, in the order they touched down.
    pub fn iter(&self) -> impl Iterator<Item = Contact<T>> + '_ {
        self.followed.iter().filter_map(|c| {
            c.target.map(|target| Contact {
                id: c.id,
                target,
                position: c.position,
                start: c.start,
                since: c.since,
                force: c.force,
                moved: c.moved,
            })
        })
    }

    /// Lets go of everything: each contact down now, whether a step has
    /// seen it yet or not, does nothing until it lifts, and lifting it
    /// releases nothing. For when what the contacts were on goes away (a
    /// screen opens over it).
    pub fn clear(&mut self) {
        for c in &mut self.followed {
            c.target = None;
        }
        self.inert = true;
    }
}

impl<T: Copy + PartialEq> Contacts<T> {
    /// Whether any contact holds `target`.
    pub fn holds(&self, target: T) -> bool {
        self.iter().any(|c| c.target == target)
    }
}

/// The contacts in a step: each finger, and the mouse while its button is
/// held and no finger drives the pointer. The host drives `pointer` and
/// `down` from the first finger of a gesture and lists every finger in
/// `touches` for as long as it is down, so a held button with no finger is
/// the mouse's.
fn current(input: &Input) -> Vec<(ContactId, Vec2, Option<f32>)> {
    let mut present: Vec<_> = input
        .touches
        .iter()
        .map(|t| (ContactId::Touch(t.id), t.position, t.force))
        .collect();
    if present.is_empty() && input.down {
        if let Some(p) = input.pointer {
            present.push((ContactId::Mouse, p, None));
        }
    }
    present
}

fn distance(a: Vec2, b: Vec2) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_ui::immediate::Touch;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Control {
        Left,
        Right,
    }

    fn hit(p: Vec2) -> Option<Control> {
        match p.x {
            x if x < 100.0 => Some(Control::Left),
            x if x < 200.0 => Some(Control::Right),
            _ => None,
        }
    }

    fn mouse(at: Option<(f32, f32)>, time: f64) -> Input {
        Input {
            pointer: at.map(|(x, y)| Vec2::new(x, y)),
            down: at.is_some(),
            time: Some(time),
            ..Input::default()
        }
    }

    /// The host's input for the fingers `down`: the first drives the pointer.
    fn fingers(down: &[(u64, f32, f32)], time: f64) -> Input {
        Input {
            pointer: down.first().map(|&(_, x, y)| Vec2::new(x, y)),
            down: !down.is_empty(),
            touches: down
                .iter()
                .map(|&(id, x, y)| Touch { id, position: Vec2::new(x, y), force: None })
                .collect(),
            time: Some(time),
            ..Input::default()
        }
    }

    #[test]
    fn a_quick_still_release_is_a_tap_and_a_long_one_is_not() {
        let mut c = Contacts::new(Tap::default());
        c.update(&fingers(&[(1, 50.0, 0.0)], 0.0), hit);
        let up = c.update(&fingers(&[], 0.1), hit);
        assert!(matches!(up[..], [ContactEvent::Released { tap: true, target: Control::Left, .. }]));

        c.update(&fingers(&[(2, 50.0, 0.0)], 1.0), hit);
        let up = c.update(&fingers(&[], 2.0), hit);
        assert!(matches!(up[..], [ContactEvent::Released { tap: false, .. }]), "a hold is not a tap");

        c.update(&fingers(&[(3, 50.0, 0.0)], 3.0), hit);
        c.update(&fingers(&[(3, 90.0, 0.0)], 3.05), hit);
        c.update(&fingers(&[(3, 50.0, 0.0)], 3.1), hit);
        let up = c.update(&fingers(&[], 3.15), hit);
        assert!(matches!(up[..], [ContactEvent::Released { tap: false, .. }]), "nor is a drag that came back");
    }

    #[test]
    fn a_contact_keeps_what_it_touched_down_on() {
        let mut c = Contacts::new(Tap::default());
        let down = c.update(&fingers(&[(1, 50.0, 0.0)], 0.0), hit);
        assert!(matches!(down[..], [ContactEvent::Pressed { target: Control::Left, .. }]));
        // Dragged onto the other control and lifted there.
        assert!(c.update(&fingers(&[(1, 150.0, 0.0)], 0.5), hit).is_empty(), "sliding presses nothing");
        assert!(c.holds(Control::Left) && !c.holds(Control::Right));
        let up = c.update(&fingers(&[], 1.0), hit);
        assert!(
            matches!(up[..], [ContactEvent::Released { target: Control::Left, .. }]),
            "released from where it began, not pressed where it ended: {up:?}"
        );
    }

    #[test]
    fn one_finger_is_one_contact_and_the_mouse_another() {
        let mut c = Contacts::new(Tap::default());
        let down = c.update(&fingers(&[(1, 50.0, 0.0)], 0.0), hit);
        assert_eq!(down.len(), 1);
        assert_eq!(c.iter().next().map(|k| k.id), Some(ContactId::Touch(1)));
        c.update(&fingers(&[], 0.1), hit);

        let down = c.update(&mouse(Some((150.0, 0.0)), 1.0), hit);
        assert!(matches!(down[..], [ContactEvent::Pressed { id: ContactId::Mouse, target: Control::Right, .. }]));
        let up = c.update(&mouse(None, 1.1), hit);
        assert!(matches!(up[..], [ContactEvent::Released { id: ContactId::Mouse, .. }]));
    }

    #[test]
    fn several_fingers_each_hold_their_own() {
        let mut c = Contacts::new(Tap::default());
        c.update(&fingers(&[(1, 50.0, 0.0)], 0.0), hit);
        let down = c.update(&fingers(&[(1, 50.0, 0.0), (2, 150.0, 0.0)], 0.1), hit);
        assert!(matches!(down[..], [ContactEvent::Pressed { id: ContactId::Touch(2), target: Control::Right, .. }]));
        // The first lifts; the second stays, and is still not the mouse.
        let input = Input { down: false, ..fingers(&[(2, 150.0, 0.0)], 0.2) };
        let up = c.update(&input, hit);
        assert!(matches!(up[..], [ContactEvent::Released { id: ContactId::Touch(1), .. }]));
        assert!(c.holds(Control::Right) && !c.holds(Control::Left));
    }

    #[test]
    fn nothing_under_a_contact_makes_it_do_nothing_until_it_lifts() {
        let mut c = Contacts::new(Tap::default());
        assert!(c.update(&fingers(&[(1, 300.0, 0.0)], 0.0), hit).is_empty());
        assert!(c.update(&fingers(&[(1, 50.0, 0.0)], 0.1), hit).is_empty(), "slid onto a control");
        assert!(c.update(&fingers(&[], 0.2), hit).is_empty());
    }

    #[test]
    fn cleared_contacts_are_inert_seen_or_not() {
        let mut c = Contacts::new(Tap::default());
        c.update(&fingers(&[(1, 50.0, 0.0)], 0.0), hit);
        c.clear();
        // Finger 1 was seen; finger 2 came down before the next step.
        let events = c.update(&fingers(&[(1, 50.0, 0.0), (2, 150.0, 0.0)], 0.1), hit);
        assert!(events.is_empty(), "{events:?}");
        assert!(c.iter().next().is_none());
        assert!(c.update(&fingers(&[], 0.2), hit).is_empty(), "lifting them releases nothing");
        // A finger after that works again.
        assert_eq!(c.update(&fingers(&[(3, 50.0, 0.0)], 0.3), hit).len(), 1);
    }
}
