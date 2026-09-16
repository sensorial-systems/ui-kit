//! Turning winit's events into the immediate host's [`Input`], and its
//! [`Output`] back into what the window shows.

use std::time::Instant;
use ui_kit_core::{Point, Rect};
use ui_kit_immediate::{AccessAction, CursorIcon, Input, Key, Modifiers, Output, TextNavigation};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key as WinitKey, ModifiersState, NamedKey},
    window::{CursorIcon as WinitCursor, Window},
};

/// How far one notch of a wheel scrolls, in logical pixels.
const LINE: f32 = 48.0;

/// Collects a window's events between frames into one [`Input`].
///
/// Feed it every [`WindowEvent`], take the input once per frame, and hand
/// the frame's [`Output`] back with [`InputCollector::apply`] so the cursor,
/// the input method and the clipboard follow what the UI asked for.
///
/// Where the pointer is and which buttons are held carry over from frame to
/// frame; keys, text, wheel movement and clipboard requests belong to the
/// frame they happened before.
pub struct InputCollector {
    input: Input,
    pointer: Option<Point>,
    down: bool,
    secondary_down: bool,
    modifiers: ModifiersState,
    composing: bool,
    start: Instant,
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
            pointer: None,
            down: false,
            secondary_down: false,
            modifiers: ModifiersState::default(),
            composing: false,
            start: Instant::now(),
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
                let p = position.to_logical::<f32>(scale);
                self.pointer = Some(Point::new(p.x, p.y));
            }
            WindowEvent::CursorLeft { .. } => {
                // A drag that leaves the window still ends where it is let go.
                if !self.down {
                    self.pointer = None;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.down = pressed,
                    MouseButton::Right => self.secondary_down = pressed,
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
                self.down = false;
                self.secondary_down = false;
                self.pointer = None;
                self.modifiers = ModifiersState::default();
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
                if event.state != ElementState::Pressed {
                    return false;
                }
                self.key(&event.logical_key, event.text.as_deref());
            }
            _ => return false,
        }
        true
    }

    fn key(&mut self, key: &WinitKey, text: Option<&str>) {
        let command = self.modifiers.control_key() || self.modifiers.super_key();
        let named = match key {
            WinitKey::Named(named) => Some(*named),
            _ => None,
        };
        let pressed = match (key, named) {
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
        };
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

    /// The input for a frame laid out over `viewport`, clearing what belongs
    /// to this frame only.
    pub fn take(&mut self, viewport: Rect) -> Input {
        let mut input = std::mem::take(&mut self.input);
        input.pointer = self.pointer;
        input.down = self.down;
        input.secondary_down = self.secondary_down;
        input.modifiers = modifiers(self.modifiers);
        input.extend_selection = self.modifiers.shift_key();
        input.time = Some(self.start.elapsed().as_secs_f64());
        input.viewport = Some(viewport);
        // A composition in progress stays on screen until it is committed.
        self.input.preedit = input.preedit.clone();
        input
    }

    /// Make the window follow what the frame asked for: its cursor, where
    /// and whether the input method opens, and the clipboard. The output's
    /// rectangles are in logical pixels.
    pub fn apply(&mut self, window: &Window, output: &Output) {
        window.set_cursor(cursor(output.cursor));
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_become_shortcuts_navigation_and_text() {
        let mut collector = InputCollector::new();
        collector.key(&WinitKey::Named(NamedKey::ArrowLeft), None);
        collector.key(&WinitKey::Character("a".into()), Some("a"));
        collector.modifiers = ModifiersState::CONTROL;
        collector.key(&WinitKey::Character("C".into()), Some("\u{3}"));
        let input = collector.take(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(
            input.keys,
            vec![Key::Left, Key::Character('a'), Key::Character('c')]
        );
        assert!(matches!(input.navigation, Some(TextNavigation::Left)));
        assert_eq!(input.text, "a", "a shortcut types nothing");
        assert!(input.copy && input.modifiers.control);
        assert!(input.time.is_some());

        let next = collector.take(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(
            next.keys.is_empty() && !next.copy,
            "keys belong to one frame"
        );
    }

    #[test]
    fn text_from_a_key_with_no_name_is_still_typed() {
        let mut collector = InputCollector::new();
        collector.key(&WinitKey::Unidentified(winit::keyboard::NativeKey::Unidentified), Some("v"));
        let input = collector.take(Rect::default());
        assert_eq!(input.text, "v");
        assert!(input.keys.is_empty());
    }

    #[test]
    fn keys_wait_while_an_input_method_composes() {
        let mut collector = InputCollector::new();
        collector.composing = true;
        collector.input.preedit = Some("\u{304b}".into());
        collector.key(&WinitKey::Named(NamedKey::Enter), None);
        let input = collector.take(Rect::default());
        assert!(input.keys.is_empty() && !input.activate);
        assert_eq!(input.preedit.as_deref(), Some("\u{304b}"));
        assert_eq!(
            collector.take(Rect::default()).preedit.as_deref(),
            Some("\u{304b}")
        );
    }
}
