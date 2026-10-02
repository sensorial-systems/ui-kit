//! Turning winit's events into the immediate host's [`Input`], and its
//! [`Output`] back into what the window shows.

use std::time::Instant;
use ui_kit_core::{Point, Rect};
use ui_kit_immediate::{
    AccessAction, CursorIcon, Input, Key, Modifiers, Output, TextNavigation, Touch,
};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::{Key as WinitKey, ModifiersState, NamedKey},
    window::{CursorGrabMode, CursorIcon as WinitCursor, Window},
};

/// How far one notch of a wheel scrolls, in logical pixels.
const LINE: f32 = 48.0;
/// Change in reported force under one finger that shows the screen senses
/// pressure rather than reporting a constant.
const PRESSURE_EPSILON: f32 = 0.01;

/// Collects a window's events between frames into [`Input`]s.
///
/// Feed it every [`WindowEvent`], take the input once per frame, and hand
/// the frame's [`Output`] back with [`InputCollector::apply`] so the cursor,
/// the input method and the clipboard follow what the UI asked for.
///
/// Where the pointer is and which buttons are held carry over from frame to
/// frame; keys, text, wheel movement and clipboard requests belong to the
/// frame they happened before.
///
/// The input between two frames is cut at every button change, as the web
/// host cuts it: a press and a release are each seen where the pointer was
/// when they happened, however slow the frame and however quickly the
/// pointer moved on. A click let go over a button, the pointer then moved
/// off it before the next frame, is still a click on the button.
pub struct InputCollector {
    input: Input,
    /// What came before a button change, closed into inputs of their own.
    queue: Vec<Input>,
    pointer: Option<Point>,
    down: bool,
    secondary_down: bool,
    middle_down: bool,
    modifiers: ModifiersState,
    composing: bool,
    /// Fingers down, in the order they touched.
    touches: Vec<Touch>,
    /// The finger driving the pointer: the first of a gesture, until it lifts.
    primary_touch: Option<u64>,
    /// The force each finger touched down with, to tell a screen that senses
    /// pressure from one that reports a constant.
    first_force: Vec<(u64, f32)>,
    pressure_sensing: bool,
    held: Vec<Key>,
    start: Instant,
    /// Whether the window holds the pointer for the UI.
    pointer_locked: bool,
    /// Whether a frame's ask for the pointer was acted on, until frames stop
    /// asking: a lock lost with the focus is not taken again unasked.
    lock_asked: bool,
    #[cfg(feature = "clipboard")]
    clipboard: Option<arboard::Clipboard>,
}

impl Default for InputCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl InputCollector {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            queue: Vec::new(),
            pointer: None,
            down: false,
            secondary_down: false,
            middle_down: false,
            modifiers: ModifiersState::default(),
            composing: false,
            touches: Vec::new(),
            primary_touch: None,
            first_force: Vec::new(),
            pressure_sensing: false,
            held: Vec::new(),
            start: Instant::now(),
            pointer_locked: false,
            lock_asked: false,
            #[cfg(feature = "clipboard")]
            clipboard: arboard::Clipboard::new().ok(),
        }
    }

    /// Where the pointer is, in logical pixels, if it is over the window.
    pub fn pointer(&self) -> Option<Point> {
        self.pointer
    }

    pub fn modifiers(&self) -> Modifiers {
        modifiers(self.modifiers)
    }

    /// Take in one event. Returns whether it is something the UI reacts to,
    /// so the host knows to draw another frame.
    pub fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let scale = window.scale_factor();
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                // Held, the pointer stays put and only its motion is news.
                if !self.pointer_locked {
                    let p = position.to_logical::<f32>(scale);
                    self.pointer = Some(Point::new(p.x, p.y));
                }
            }
            WindowEvent::CursorLeft { .. } => {
                // A drag that leaves the window still ends where it is let go.
                if !(self.down || self.secondary_down || self.middle_down) {
                    self.pointer = None;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left | MouseButton::Right | MouseButton::Middle => {
                        self.button(*button, pressed)
                    }
                    _ => return false,
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (-x * LINE, -y * LINE),
                    MouseScrollDelta::PixelDelta(p) => {
                        let p = p.to_logical::<f32>(scale);
                        (-p.x, -p.y)
                    }
                };
                self.input.scroll.x += x;
                self.input.scroll.y += y;
            }
            WindowEvent::ModifiersChanged(state) => {
                self.modifiers = state.state();
                self.input.extend_selection = self.modifiers.shift_key();
                return false;
            }
            WindowEvent::Focused(false) => {
                // The system gives a held pointer back with the focus, as a
                // browser does.
                if self.pointer_locked {
                    self.release_pointer(window);
                }
                self.down = false;
                self.secondary_down = false;
                self.middle_down = false;
                self.pointer = None;
                self.modifiers = ModifiersState::default();
                self.touches.clear();
                self.primary_touch = None;
                self.held.clear();
            }
            WindowEvent::Touch(touch) => {
                let p = touch.location.to_logical::<f32>(scale);
                let force = touch.force.map(|f| f.normalized() as f32);
                self.touch(touch.id, Point::new(p.x, p.y), force, touch.phase);
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => self.composing = false,
                Ime::Disabled => {
                    self.composing = false;
                    self.input.preedit = None;
                }
                Ime::Preedit(text, _) => {
                    self.composing = !text.is_empty();
                    self.input.preedit = (!text.is_empty()).then(|| text.clone());
                }
                Ime::Commit(text) => {
                    self.composing = false;
                    self.input.preedit = None;
                    self.input.text.push_str(text);
                }
            },
            WindowEvent::KeyboardInput { event, .. } => {
                let key = key_of(&event.logical_key);
                if event.state != ElementState::Pressed {
                    self.held.retain(|k| Some(*k) != key);
                    return false;
                }
                if let Some(key) = key.filter(|k| !self.held.contains(k)) {
                    self.held.push(key);
                }
                self.key(&event.logical_key, event.text.as_deref());
            }
            _ => return false,
        }
        true
    }

    /// A finger went down, moved or lifted. The first finger of a gesture
    /// is the pointer, pressed while it is down, so every widget answers to
    /// touch; the others are only in [`Input::touches`], and a finger landing
    /// while another is down does not move the pointer.
    fn touch(&mut self, id: u64, position: Point, force: Option<f32>, phase: TouchPhase) {
        if let Some(f) = force {
            match self.first_force.iter().find(|(i, _)| *i == id) {
                // A force that changes under one finger is a pressure sensor.
                Some((_, first)) if (f - first).abs() > PRESSURE_EPSILON => self.pressure_sensing = true,
                Some(_) => {}
                None => self.first_force.push((id, f)),
            }
        }
        // The first finger and the press it makes come and go together: no
        // step has the pointer pressed by a finger that is not in `touches`,
        // or that finger without the press, which would read as the mouse.
        match phase {
            TouchPhase::Started => {
                self.touches.retain(|t| t.id != id);
                if self.primary_touch.is_none() && !self.down {
                    self.primary_touch = Some(id);
                    self.segment();
                    self.touches.push(Touch { id, position, force });
                    self.pointer = Some(position);
                    self.down = true;
                    self.segment();
                } else {
                    self.touches.push(Touch { id, position, force });
                }
            }
            TouchPhase::Moved => {
                if let Some(t) = self.touches.iter_mut().find(|t| t.id == id) {
                    t.position = position;
                    t.force = force;
                }
                if self.primary_touch == Some(id) {
                    self.pointer = Some(position);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.primary_touch == Some(id) && self.down {
                    // Seen down where it lifts, then gone with the press.
                    if let Some(t) = self.touches.iter_mut().find(|t| t.id == id) {
                        t.position = position;
                    }
                    self.pointer = Some(position);
                    self.segment();
                    self.touches.retain(|t| t.id != id);
                    self.down = false;
                    self.segment();
                } else {
                    self.touches.retain(|t| t.id != id);
                }
                self.first_force.retain(|(i, _)| *i != id);
                if self.touches.is_empty() {
                    self.primary_touch = None;
                }
            }
        }
    }

    /// `button` went down or up. What was gathered before is closed into an
    /// input of its own with the buttons as they were, and the change into
    /// another with the pointer where it is now: each is a step of the next
    /// frame, so a press lands where it was made and a release where it let
    /// go, before the pointer moves on.
    fn button(&mut self, button: MouseButton, pressed: bool) {
        let held = match button {
            MouseButton::Left => self.down,
            MouseButton::Right => self.secondary_down,
            MouseButton::Middle => self.middle_down,
            _ => return,
        };
        if held == pressed {
            return;
        }
        self.segment();
        match button {
            MouseButton::Left => self.down = pressed,
            MouseButton::Right => self.secondary_down = pressed,
            _ => self.middle_down = pressed,
        }
        self.segment();
    }

    /// Close the input gathered so far into a step of its own, with what
    /// lasts across frames as it is now.
    fn segment(&mut self) {
        let mut input = std::mem::take(&mut self.input);
        input.pointer = self.pointer;
        input.down = self.down;
        input.secondary_down = self.secondary_down;
        input.middle_down = self.middle_down;
        input.touches = self.touches.clone();
        input.pressure_sensing = self.pressure_sensing;
        input.held = self.held.clone();
        input.modifiers = modifiers(self.modifiers);
        input.extend_selection = self.modifiers.shift_key();
        input.pointer_locked = self.pointer_locked;
        // A composition in progress stays on screen until it is committed.
        self.input.preedit = input.preedit.clone();
        self.queue.push(input);
    }

    fn key(&mut self, key: &WinitKey, text: Option<&str>) {
        let command = self.modifiers.control_key() || self.modifiers.super_key();
        let pressed = key_of(key);
        // While an input method composes, the keys are its to interpret.
        if self.composing {
            return;
        }
        // What a key types is typed whether or not the key has a name here:
        // characters sent by on-screen keyboards and automation arrive as
        // keys with no identity but with their text.
        if !command {
            if let Some(text) = text.filter(|t| t.chars().all(|c| !c.is_control())) {
                self.input.text.push_str(text);
            }
        }
        let Some(pressed) = pressed else {
            return;
        };
        self.input.keys.push(pressed);
        match pressed {
            Key::Tab => self.input.tab = true,
            Key::Backspace => self.input.backspace = true,
            Key::Delete => self.input.delete = true,
            Key::Enter => self.input.activate = true,
            Key::Left => {
                self.input.step = -1;
                self.input.navigation = Some(TextNavigation::Left);
            }
            Key::Right => {
                self.input.step = 1;
                self.input.navigation = Some(TextNavigation::Right);
            }
            Key::Up => {
                self.input.step = 1;
                self.input.navigation = Some(TextNavigation::Up);
            }
            Key::Down => {
                self.input.step = -1;
                self.input.navigation = Some(TextNavigation::Down);
            }
            Key::Home => self.input.navigation = Some(TextNavigation::Home),
            Key::End => self.input.navigation = Some(TextNavigation::End),
            Key::Character(c) if command => match c {
                'a' => self.input.select_all = true,
                'c' => self.input.copy = true,
                'x' => self.input.cut = true,
                'v' => self.input.paste = self.read_clipboard(),
                'z' if self.modifiers.shift_key() => self.input.redo = true,
                'z' => self.input.undo = true,
                'y' => self.input.redo = true,
                's' => self.input.save = true,
                _ => {}
            },
            _ => {}
        }
    }

    #[cfg(feature = "clipboard")]
    fn read_clipboard(&mut self) -> Option<String> {
        self.clipboard.as_mut()?.get_text().ok()
    }

    #[cfg(not(feature = "clipboard"))]
    fn read_clipboard(&mut self) -> Option<String> {
        None
    }

    /// Queue requests from assistive technology for the next frame.
    pub fn access_actions(&mut self, actions: impl IntoIterator<Item = AccessAction>) {
        self.input.access_actions.extend(actions);
    }

    /// The input since the last frame laid out over `viewport`: one input
    /// for each stretch between button changes, in the order they came, for
    /// the UI to step through. The last is the one to draw. Never empty.
    pub fn take(&mut self, viewport: Rect) -> Vec<Input> {
        self.segment();
        let time = self.start.elapsed().as_secs_f64();
        let mut inputs = std::mem::take(&mut self.queue);
        for input in &mut inputs {
            input.time = Some(time);
            input.viewport = Some(viewport);
        }
        inputs
    }

    /// Make the window follow what the frame asked for: its cursor, where
    /// and whether the input method opens, and the clipboard. The output's
    /// rectangles are in logical pixels.
    pub fn apply(&mut self, window: &Window, output: &Output) {
        window.set_cursor(cursor(output.cursor));
        if output.pointer_lock && !self.lock_asked {
            self.lock_asked = true;
            // Locked where the system can, kept in the window where it only
            // confines, as winit offers on each.
            let grabbed = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
            if grabbed.is_ok() {
                window.set_cursor_visible(false);
                self.pointer_locked = true;
            }
        } else if !output.pointer_lock && self.lock_asked {
            self.lock_asked = false;
            if self.pointer_locked {
                self.release_pointer(window);
            }
        }
        window.set_ime_allowed(output.wants_keyboard);
        if let Some(area) = output.ime_area {
            window.set_ime_cursor_area(
                LogicalPosition::new(area.x, area.y),
                LogicalSize::new(area.width, area.height),
            );
        }
        if let Some(text) = &output.clipboard {
            self.write_clipboard(text);
        }
        if output.repaint_after.is_some() {
            window.request_redraw();
        }
    }

    /// The mouse moved by `delta` device units, whatever the pointer did:
    /// from `DeviceEvent::MouseMotion`, which goes on while it is held.
    pub fn motion(&mut self, window: &Window, delta: (f64, f64)) {
        let scale = window.scale_factor();
        self.input.pointer_motion.x += (delta.0 / scale) as f32;
        self.input.pointer_motion.y += (delta.1 / scale) as f32;
    }

    fn release_pointer(&mut self, window: &Window) {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        self.pointer_locked = false;
    }

    #[cfg(feature = "clipboard")]
    fn write_clipboard(&mut self, text: &str) {
        if let Some(clipboard) = self.clipboard.as_mut() {
            let _ = clipboard.set_text(text);
        }
    }

    #[cfg(not(feature = "clipboard"))]
    fn write_clipboard(&mut self, _: &str) {}
}

fn modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        shift: state.shift_key(),
        control: state.control_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}

/// The UI key for a winit key; characters are lower case, so a shortcut is
/// the same key with or without shift.
fn key_of(key: &WinitKey) -> Option<Key> {
    let named = match key {
        WinitKey::Named(named) => Some(*named),
        _ => None,
    };
    match (key, named) {
        (_, Some(NamedKey::Escape)) => Some(Key::Escape),
        (_, Some(NamedKey::Enter)) => Some(Key::Enter),
        (_, Some(NamedKey::Tab)) => Some(Key::Tab),
        (_, Some(NamedKey::Space)) => Some(Key::Space),
        (_, Some(NamedKey::Backspace)) => Some(Key::Backspace),
        (_, Some(NamedKey::Delete)) => Some(Key::Delete),
        (_, Some(NamedKey::ArrowLeft)) => Some(Key::Left),
        (_, Some(NamedKey::ArrowRight)) => Some(Key::Right),
        (_, Some(NamedKey::ArrowUp)) => Some(Key::Up),
        (_, Some(NamedKey::ArrowDown)) => Some(Key::Down),
        (_, Some(NamedKey::Home)) => Some(Key::Home),
        (_, Some(NamedKey::End)) => Some(Key::End),
        (_, Some(NamedKey::PageUp)) => Some(Key::PageUp),
        (_, Some(NamedKey::PageDown)) => Some(Key::PageDown),
        (WinitKey::Named(named), _) => function_key(*named).map(Key::F),
        (WinitKey::Character(c), _) => c
            .chars()
            .next()
            .map(|c| Key::Character(c.to_ascii_lowercase())),
        _ => None,
    }
}

fn function_key(key: NamedKey) -> Option<u8> {
    use NamedKey::*;
    Some(match key {
        F1 => 1,
        F2 => 2,
        F3 => 3,
        F4 => 4,
        F5 => 5,
        F6 => 6,
        F7 => 7,
        F8 => 8,
        F9 => 9,
        F10 => 10,
        F11 => 11,
        F12 => 12,
        _ => return None,
    })
}

/// The winit cursor for a UI cursor.
pub fn cursor(icon: CursorIcon) -> WinitCursor {
    match icon {
        CursorIcon::Default => WinitCursor::Default,
        CursorIcon::Pointer => WinitCursor::Pointer,
        CursorIcon::Text => WinitCursor::Text,
        CursorIcon::ResizeHorizontal => WinitCursor::EwResize,
        CursorIcon::ResizeVertical => WinitCursor::NsResize,
        CursorIcon::Grab => WinitCursor::Grab,
        CursorIcon::Grabbing => WinitCursor::Grabbing,
        CursorIcon::NotAllowed => WinitCursor::NotAllowed,
        CursorIcon::Crosshair => WinitCursor::Crosshair,
        CursorIcon::ResizeDiagonal => WinitCursor::NwseResize,
        CursorIcon::Help => WinitCursor::Help,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl InputCollector {
        /// The input a frame draws with: its last step.
        fn last(&mut self, viewport: Rect) -> Input {
            self.take(viewport).pop().expect("a frame always has input")
        }
    }

    fn press(c: &mut InputCollector, at: Point, pressed: bool) {
        c.pointer = Some(at);
        c.button(MouseButton::Left, pressed);
    }

    #[test]
    fn a_click_between_frames_is_seen_where_it_happened() {
        let mut c = InputCollector::new();
        let button = Point::new(10.0, 10.0);
        c.pointer = Some(Point::new(200.0, 200.0));
        press(&mut c, button, true);
        press(&mut c, button, false);
        // The pointer moved on before the frame was made.
        c.pointer = Some(Point::new(300.0, 300.0));
        let steps = c.take(Rect::default());
        let seen: Vec<(bool, Option<Point>)> = steps.iter().map(|i| (i.down, i.pointer)).collect();
        let pressed_at = seen.iter().position(|(down, _)| *down).expect("the press is seen");
        assert_eq!(seen[pressed_at].1, Some(button), "pressed where it was pressed");
        let released = seen[pressed_at..].iter().find(|(down, _)| !*down);
        assert_eq!(released, Some(&(false, Some(button))), "let go where it was let go");
        assert_eq!(steps.last().unwrap().pointer, Some(Point::new(300.0, 300.0)), "and drawn where it is now");
    }

    #[test]
    fn a_held_button_carries_over_frames() {
        let mut c = InputCollector::new();
        press(&mut c, Point::new(1.0, 1.0), true);
        assert!(c.last(Rect::default()).down);
        let next = c.take(Rect::default());
        assert_eq!(next.len(), 1, "no change, one step");
        assert!(next[0].down);
        press(&mut c, Point::new(1.0, 1.0), false);
        assert!(!c.last(Rect::default()).down);
    }

    #[test]
    fn keys_become_shortcuts_navigation_and_text() {
        let mut collector = InputCollector::new();
        collector.key(&WinitKey::Named(NamedKey::ArrowLeft), None);
        collector.key(&WinitKey::Character("a".into()), Some("a"));
        collector.modifiers = ModifiersState::CONTROL;
        collector.key(&WinitKey::Character("C".into()), Some("\u{3}"));
        let input = collector.last(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(
            input.keys,
            vec![Key::Left, Key::Character('a'), Key::Character('c')]
        );
        assert!(matches!(input.navigation, Some(TextNavigation::Left)));
        assert_eq!(input.text, "a", "a shortcut types nothing");
        assert!(input.copy && input.modifiers.control);
        assert!(input.time.is_some());

        let next = collector.last(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(
            next.keys.is_empty() && !next.copy,
            "keys belong to one frame"
        );
    }

    #[test]
    fn first_finger_is_the_pointer_and_all_fingers_are_touches() {
        let mut c = InputCollector::new();
        c.touch(7, Point::new(10.0, 20.0), None, TouchPhase::Started);
        c.touch(9, Point::new(300.0, 40.0), None, TouchPhase::Started);
        c.touch(9, Point::new(310.0, 50.0), None, TouchPhase::Moved);
        let input = c.last(Rect::default());
        assert!(input.down);
        assert_eq!(
            input.pointer,
            Some(Point::new(10.0, 20.0)),
            "a second finger does not move the pointer"
        );
        assert_eq!(input.touches.len(), 2);
        assert_eq!(input.touches[1].position, Point::new(310.0, 50.0));

        // The first finger lifts: the press ends, and the other finger stays
        // a touch without taking over the pointer mid-gesture.
        c.touch(7, Point::new(12.0, 22.0), None, TouchPhase::Ended);
        let input = c.last(Rect::default());
        assert!(!input.down);
        assert_eq!(input.touches.iter().map(|t| t.id).collect::<Vec<_>>(), vec![9]);
        c.touch(9, Point::new(320.0, 50.0), None, TouchPhase::Moved);
        assert_eq!(c.last(Rect::default()).pointer, Some(Point::new(12.0, 22.0)));

        // Once every finger is up, the next one leads a new gesture.
        c.touch(9, Point::new(320.0, 50.0), None, TouchPhase::Ended);
        c.touch(3, Point::new(5.0, 5.0), None, TouchPhase::Started);
        let input = c.last(Rect::default());
        assert!(input.down);
        assert_eq!(input.pointer, Some(Point::new(5.0, 5.0)));
    }

    /// No step has the pointer pressed by a finger that is not listed, nor
    /// a lone finger listed without its press: either reads as the mouse,
    /// and a tap becomes two.
    #[test]
    fn a_finger_and_its_press_come_and_go_together() {
        use ui_kit_immediate::{ContactEvent, ContactId, Contacts, Tap};
        let mut c = InputCollector::new();
        let mut contacts = Contacts::new(Tap::default());
        let mut events = Vec::new();
        let mut follow = |c: &mut InputCollector, events: &mut Vec<ContactEvent<()>>| {
            for step in c.take(Rect::default()) {
                assert_eq!(step.down, !step.touches.is_empty(), "one finger: pressed exactly while listed");
                events.extend(contacts.update(&step, |_| Some(())));
            }
        };
        // A tap, all between two frames.
        c.touch(4, Point::new(10.0, 10.0), None, TouchPhase::Started);
        c.touch(4, Point::new(11.0, 10.0), None, TouchPhase::Ended);
        follow(&mut c, &mut events);
        // A drag lifted far from where it began, over several frames.
        c.touch(5, Point::new(100.0, 10.0), None, TouchPhase::Started);
        follow(&mut c, &mut events);
        c.touch(5, Point::new(20.0, 10.0), None, TouchPhase::Moved);
        follow(&mut c, &mut events);
        c.touch(5, Point::new(5.0, 10.0), None, TouchPhase::Ended);
        follow(&mut c, &mut events);
        let seen: Vec<(bool, ContactId)> = events
            .iter()
            .map(|e| match e {
                ContactEvent::Pressed { id, .. } => (true, *id),
                ContactEvent::Released { id, .. } => (false, *id),
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                (true, ContactId::Touch(4)),
                (false, ContactId::Touch(4)),
                (true, ContactId::Touch(5)),
                (false, ContactId::Touch(5)),
            ],
            "each finger is one press and one release, and never the mouse"
        );
    }

    #[test]
    fn pressure_sensing_needs_a_force_that_changes() {
        let mut c = InputCollector::new();
        // A screen that reports a constant force.
        c.touch(1, Point::new(0.0, 0.0), Some(1.0), TouchPhase::Started);
        c.touch(1, Point::new(1.0, 0.0), Some(1.0), TouchPhase::Moved);
        c.touch(1, Point::new(1.0, 0.0), Some(1.0), TouchPhase::Ended);
        assert!(!c.last(Rect::default()).pressure_sensing);
        // A finger pressing harder.
        c.touch(2, Point::new(0.0, 0.0), Some(0.2), TouchPhase::Started);
        c.touch(2, Point::new(0.0, 0.0), Some(0.6), TouchPhase::Moved);
        let input = c.last(Rect::default());
        assert!(input.pressure_sensing);
        assert_eq!(input.touches[0].force, Some(0.6));
    }

    #[test]
    fn held_keys_last_until_released() {
        let mut c = InputCollector::new();
        c.held.push(Key::Up);
        let input = c.last(Rect::default());
        assert_eq!(input.held, vec![Key::Up]);
        assert_eq!(c.last(Rect::default()).held, vec![Key::Up], "held keys carry over frames");
    }

    #[test]
    fn text_from_a_key_with_no_name_is_still_typed() {
        let mut collector = InputCollector::new();
        collector.key(&WinitKey::Unidentified(winit::keyboard::NativeKey::Unidentified), Some("v"));
        let input = collector.last(Rect::default());
        assert_eq!(input.text, "v");
        assert!(input.keys.is_empty());
    }

    #[test]
    fn keys_wait_while_an_input_method_composes() {
        let mut collector = InputCollector::new();
        collector.composing = true;
        collector.input.preedit = Some("\u{304b}".into());
        collector.key(&WinitKey::Named(NamedKey::Enter), None);
        let input = collector.last(Rect::default());
        assert!(input.keys.is_empty() && !input.activate);
        assert_eq!(input.preedit.as_deref(), Some("\u{304b}"));
        assert_eq!(
            collector.last(Rect::default()).preedit.as_deref(),
            Some("\u{304b}")
        );
    }
}
