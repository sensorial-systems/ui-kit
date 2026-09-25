//! The page's DOM events, collected between frames into one [`Input`], and
//! a frame's [`Output`] applied back to the page.
//!
//! Keyboard input goes through a hidden `<textarea>` that keeps the focus,
//! rather than through the canvas: that is what gives an input method a
//! place to compose, an on-screen keyboard a field to type into, and paste a
//! target the browser hands the clipboard's text to.

use std::cell::RefCell;
use std::rc::Rc;
use ui_kit_core::{Point, Rect};
use ui_kit_immediate::{CursorIcon, Input, Key, Modifiers, Output, TextNavigation, Touch};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    AddEventListenerOptions, ClipboardEvent, CompositionEvent, Event, EventTarget, HtmlCanvasElement,
    HtmlTextAreaElement, KeyboardEvent, PointerEvent, WheelEvent,
};

/// How far one line of a wheel scrolls, in CSS pixels.
const LINE: f32 = 48.0;

#[derive(Default)]
struct State {
    input: Input,
    pointer: Option<Point>,
    down: bool,
    secondary_down: bool,
    middle_down: bool,
    /// Input already closed into frames of its own, oldest first.
    queue: Vec<Input>,
    modifiers: Modifiers,
    composing: bool,
    /// Fingers down, in the order they touched.
    touches: Vec<Touch>,
    /// The finger driving the pointer: the first of a gesture, until it lifts.
    primary_touch: Option<u64>,
    held: Vec<Key>,
    /// Whether the last pointer was a finger, to raise the on-screen keyboard
    /// only when a text field asks for it.
    touch: bool,
    wants_keyboard: bool,
    /// Whether the page holds the pointer on the canvas.
    pointer_locked: bool,
}

/// An event listener, removed from its target when dropped.
struct Listener {
    target: EventTarget,
    event: &'static str,
    closure: Closure<dyn FnMut(Event)>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, self.closure.as_ref().unchecked_ref());
    }
}

pub(crate) struct DomInput {
    state: Rc<RefCell<State>>,
    canvas: HtmlCanvasElement,
    textarea: HtmlTextAreaElement,
    cursor: CursorIcon,
    /// Whether a frame's ask for the pointer was passed on, until frames
    /// stop asking: a lock the user took back is not asked for again.
    lock_asked: bool,
    _listeners: Vec<Listener>,
}

impl DomInput {
    /// Listen to `canvas`, the hidden text field and the window.
    pub fn attach(canvas: &HtmlCanvasElement) -> Result<Self, wasm_bindgen::JsValue> {
        let window = web_sys::window().ok_or("no window")?;
        let document = window.document().ok_or("no document")?;
        let textarea: HtmlTextAreaElement = document.create_element("textarea")?.dyn_into()?;
        for (name, value) in [
            ("autocapitalize", "off"),
            ("autocomplete", "off"),
            ("autocorrect", "off"),
            ("spellcheck", "false"),
            ("aria-hidden", "true"),
            ("tabindex", "-1"),
        ] {
            textarea.set_attribute(name, value)?;
        }
        textarea.set_attribute(
            "style",
            "position: fixed; left: 0; top: 0; width: 1px; height: 1px; opacity: 0; \
             border: 0; padding: 0; margin: 0; resize: none; overflow: hidden; \
             pointer-events: none; z-index: -1; font-size: 16px;",
        )?;
        document.body().ok_or("no body")?.append_child(&textarea)?;

        let state = Rc::new(RefCell::new(State::default()));
        let mut listeners = Vec::new();
        let canvas_target: EventTarget = canvas.clone().into();
        let text_target: EventTarget = textarea.clone().into();
        let window_target: EventTarget = window.clone().into();

        let mut listen = |target: &EventTarget, event: &'static str, handler: Box<dyn FnMut(Event)>| {
            let closure = Closure::wrap(handler);
            let options = AddEventListenerOptions::new();
            // A wheel or a touch the page is not allowed to cancel scrolls or
            // zooms the page under the canvas.
            options.set_passive(false);
            let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
                event,
                closure.as_ref().unchecked_ref(),
                &options,
            );
            listeners.push(Listener {
                target: target.clone(),
                event,
                closure,
            });
        };

        {
            let state = state.clone();
            let canvas = canvas.clone();
            let textarea = textarea.clone();
            listen(
                &canvas_target,
                "pointerdown",
                Box::new(move |event| {
                    let event: PointerEvent = event.unchecked_into();
                    // Keeps the page from selecting text or moving the focus
                    // off the text field.
                    event.prevent_default();
                    let _ = canvas.set_pointer_capture(event.pointer_id());
                    let mut state = state.borrow_mut();
                    state.set_modifiers(modifiers_of_pointer(&event));
                    let position = position(&canvas, event.client_x(), event.client_y());
                    if event.pointer_type() == "touch" {
                        state.touch = true;
                        state.finger(event.pointer_id() as u64, position, force(&event), Phase::Start);
                    } else {
                        state.touch = false;
                        state.pointer = Some(position);
                        state.button(event.button(), true);
                    }
                    // The text field holds the keyboard. A finger only raises the
                    // on-screen keyboard where something is being typed into.
                    if !state.touch || state.wants_keyboard {
                        let _ = textarea.focus();
                    }
                }),
            );
        }
        {
            let state = state.clone();
            let canvas = canvas.clone();
            listen(
                &canvas_target,
                "pointermove",
                Box::new(move |event| {
                    let event: PointerEvent = event.unchecked_into();
                    let mut state = state.borrow_mut();
                    let position = position(&canvas, event.client_x(), event.client_y());
                    if event.pointer_type() == "touch" {
                        event.prevent_default();
                        state.finger(event.pointer_id() as u64, position, force(&event), Phase::Move);
                    } else {
                        // Held, the pointer stays put and only its motion is news.
                        state.input.pointer_motion.x += event.movement_x() as f32;
                        state.input.pointer_motion.y += event.movement_y() as f32;
                        if !state.pointer_locked {
                            state.pointer = Some(position);
                        }
                    }
                }),
            );
        }
        for name in ["pointerup", "pointercancel"] {
            let state = state.clone();
            let canvas = canvas.clone();
            let textarea = textarea.clone();
            listen(
                &canvas_target,
                name,
                Box::new(move |event| {
                    let event: PointerEvent = event.unchecked_into();
                    let _ = canvas.release_pointer_capture(event.pointer_id());
                    let mut state = state.borrow_mut();
                    let position = position(&canvas, event.client_x(), event.client_y());
                    if event.pointer_type() == "touch" {
                        state.finger(event.pointer_id() as u64, position, force(&event), Phase::End);
                        // The lift that finishes a tap on a text field is the
                        // gesture a browser lets raise the keyboard from.
                        if state.wants_keyboard {
                            let _ = textarea.focus();
                        }
                    } else {
                        state.pointer = Some(position);
                        state.button(event.button(), false);
                    }
                }),
            );
        }
        {
            let state = state.clone();
            listen(
                &canvas_target,
                "pointerleave",
                Box::new(move |_| {
                    let mut state = state.borrow_mut();
                    // A drag that leaves the canvas still ends where it is let go.
                    if !(state.down || state.secondary_down || state.middle_down) {
                        state.pointer = None;
                    }
                }),
            );
        }
        listen(&canvas_target, "contextmenu", Box::new(|event| event.prevent_default()));
        {
            let state = state.clone();
            let canvas = canvas.clone();
            listen(
                &canvas_target,
                "wheel",
                Box::new(move |event| {
                    let event: WheelEvent = event.unchecked_into();
                    event.prevent_default();
                    let unit = match event.delta_mode() {
                        WheelEvent::DOM_DELTA_LINE => LINE,
                        WheelEvent::DOM_DELTA_PAGE => canvas.client_height() as f32,
                        _ => 1.0,
                    };
                    let mut state = state.borrow_mut();
                    state.set_modifiers(Modifiers {
                        shift: event.shift_key(),
                        control: event.ctrl_key(),
                        alt: event.alt_key(),
                        meta: event.meta_key(),
                    });
                    state.pointer = Some(position(&canvas, event.client_x(), event.client_y()));
                    state.input.scroll.x += event.delta_x() as f32 * unit;
                    state.input.scroll.y += event.delta_y() as f32 * unit;
                }),
            );
        }
        {
            let state = state.clone();
            listen(
                &text_target,
                "keydown",
                Box::new(move |event| {
                    let event: KeyboardEvent = event.unchecked_into();
                    let mut state = state.borrow_mut();
                    state.set_modifiers(modifiers_of_key(&event));
                    // While an input method composes, the keys are its to interpret.
                    if event.is_composing() || event.key_code() == 229 || state.composing {
                        return;
                    }
                    if !state.key_down(&event.key()) {
                        event.prevent_default();
                    }
                }),
            );
        }
        {
            let state = state.clone();
            listen(
                &text_target,
                "keyup",
                Box::new(move |event| {
                    let event: KeyboardEvent = event.unchecked_into();
                    let mut state = state.borrow_mut();
                    state.set_modifiers(modifiers_of_key(&event));
                    if let Some(key) = key_of(&event.key()) {
                        state.held.retain(|k| *k != key);
                    }
                }),
            );
        }
        {
            let state = state.clone();
            let textarea = textarea.clone();
            listen(
                &text_target,
                "input",
                Box::new(move |_| {
                    let mut state = state.borrow_mut();
                    if state.composing {
                        return;
                    }
                    let typed = textarea.value();
                    textarea.set_value("");
                    let typed: String = typed.chars().filter(|c| !c.is_control()).collect();
                    state.input.text.push_str(&typed);
                }),
            );
        }
        {
            let state = state.clone();
            listen(
                &text_target,
                "compositionstart",
                Box::new(move |_| state.borrow_mut().composing = true),
            );
        }
        {
            let state = state.clone();
            listen(
                &text_target,
                "compositionupdate",
                Box::new(move |event| {
                    let event: CompositionEvent = event.unchecked_into();
                    let text = event.data().unwrap_or_default();
                    state.borrow_mut().input.preedit = (!text.is_empty()).then_some(text);
                }),
            );
        }
        {
            let state = state.clone();
            let textarea = textarea.clone();
            listen(
                &text_target,
                "compositionend",
                Box::new(move |event| {
                    let event: CompositionEvent = event.unchecked_into();
                    let mut state = state.borrow_mut();
                    state.composing = false;
                    state.input.preedit = None;
                    state.input.text.push_str(&event.data().unwrap_or_default());
                    textarea.set_value("");
                }),
            );
        }
        {
            let state = state.clone();
            listen(
                &text_target,
                "paste",
                Box::new(move |event| {
                    let event: ClipboardEvent = event.unchecked_into();
                    event.prevent_default();
                    let text = event
                        .clipboard_data()
                        .and_then(|data| data.get_data("text/plain").ok())
                        .unwrap_or_default();
                    state.borrow_mut().input.paste = Some(text);
                }),
            );
        }
        for name in ["copy", "cut"] {
            let state = state.clone();
            listen(
                &text_target,
                name,
                Box::new(move |event| {
                    // The text is the frame's to choose; it is written once the
                    // frame has run (see `apply`).
                    event.prevent_default();
                    let mut state = state.borrow_mut();
                    if name == "copy" {
                        state.input.copy = true;
                    } else {
                        state.input.cut = true;
                    }
                }),
            );
        }
        {
            let state = state.clone();
            listen(
                &window_target,
                "blur",
                Box::new(move |_| {
                    let mut state = state.borrow_mut();
                    state.button(0, false);
                    state.button(1, false);
                    state.button(2, false);
                    state.pointer = None;
                    state.set_modifiers(Modifiers::default());
                    state.touches.clear();
                    state.primary_touch = None;
                    state.held.clear();
                }),
            );
        }

        {
            // Whether the canvas holds the pointer: taken when a frame asked,
            // given back when the user presses Esc or the page loses it.
            let state = state.clone();
            let canvas: web_sys::Element = canvas.clone().into();
            let document_target: EventTarget = document.clone().into();
            listen(
                &document_target,
                "pointerlockchange",
                Box::new(move |_| {
                    let locked = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.pointer_lock_element())
                        .is_some_and(|element| element == canvas);
                    let mut state = state.borrow_mut();
                    state.segment();
                    state.pointer_locked = locked;
                }),
            );
        }

        let _ = textarea.focus();
        Ok(Self {
            state,
            canvas: canvas.clone(),
            textarea,
            cursor: CursorIcon::Default,
            lock_asked: false,
            _listeners: listeners,
        })
    }

    /// The input since the last frame, laid out over `viewport` at `time`
    /// seconds: one input for each stretch between button changes, in the
    /// order they happened, so the UI can step through them.
    pub fn take(&mut self, viewport: Rect, time: f64) -> Vec<Input> {
        let mut state = self.state.borrow_mut();
        state.segment();
        let mut inputs = std::mem::take(&mut state.queue);
        for input in &mut inputs {
            input.time = Some(time);
            input.viewport = Some(viewport);
        }
        inputs
    }

    /// Make the page follow what the frame asked for: the cursor, where the
    /// input method opens, whether the on-screen keyboard shows, and the
    /// clipboard. The output's rectangles are in CSS pixels.
    pub fn apply(&mut self, output: &Output) {
        if output.cursor != self.cursor {
            self.cursor = output.cursor;
            let _ = self.canvas.style().set_property("cursor", css_cursor(output.cursor));
        }
        // The frame after the click that asked, well within the browser's
        // grace for a lock to follow a gesture.
        if output.pointer_lock && !self.lock_asked {
            self.lock_asked = true;
            self.canvas.request_pointer_lock();
        } else if !output.pointer_lock && self.lock_asked {
            self.lock_asked = false;
            if self.state.borrow().pointer_locked {
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    document.exit_pointer_lock();
                }
            }
        }
        {
            let mut state = self.state.borrow_mut();
            if state.wants_keyboard && !output.wants_keyboard && state.touch {
                // Put the on-screen keyboard away once nothing is typed into.
                let _ = self.textarea.blur();
            }
            state.wants_keyboard = output.wants_keyboard;
        }
        if let Some(area) = output.ime_area {
            let style = self.textarea.style();
            let _ = style.set_property("left", &format!("{}px", area.x));
            let _ = style.set_property("top", &format!("{}px", area.y + area.height));
        }
        if let Some(text) = &output.clipboard {
            if let Some(window) = web_sys::window() {
                let _ = window.navigator().clipboard().write_text(text);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Start,
    Move,
    End,
}

impl State {
    fn button(&mut self, button: i16, pressed: bool) {
        let held = match button {
            0 => self.down,
            1 => self.middle_down,
            2 => self.secondary_down,
            _ => return,
        };
        if held == pressed {
            return;
        }
        self.segment();
        match button {
            0 => self.down = pressed,
            1 => self.middle_down = pressed,
            _ => self.secondary_down = pressed,
        }
        // A press or a release is seen where it happened, before the
        // pointer moves on: a drag that starts and ends between two frames
        // still starts where it started, and a click let go and followed by
        // a move does not drag what it let go of.
        self.segment();
    }

    /// Close the input gathered so far into a frame of its own, with the
    /// buttons as they were, before a button changes. What was typed before
    /// a click goes where the focus was before it; a press and its release
    /// are each seen, however quickly they came.
    fn segment(&mut self) {
        let mut input = std::mem::take(&mut self.input);
        self.fill(&mut input);
        // A composition in progress stays on screen across frames.
        self.input.preedit = input.preedit.clone();
        self.queue.push(input);
    }

    /// The modifier keys changed. A scroll or keys already gathered are
    /// closed into an input of their own first, so they keep the modifiers
    /// they were made with: Control let go before the next frame still
    /// zooms, and Control+S still saves.
    fn set_modifiers(&mut self, modifiers: Modifiers) {
        let scrolled = self.input.scroll.x != 0.0 || self.input.scroll.y != 0.0;
        let typed = !self.input.keys.is_empty();
        if modifiers != self.modifiers && (scrolled || typed) {
            self.segment();
        }
        self.modifiers = modifiers;
    }

    /// Put what lasts across frames into a frame's input.
    fn fill(&self, input: &mut Input) {
        input.pointer = self.pointer;
        input.down = self.down;
        input.secondary_down = self.secondary_down;
        input.middle_down = self.middle_down;
        input.touches = self.touches.clone();
        input.held = self.held.clone();
        input.modifiers = self.modifiers;
        input.extend_selection = self.modifiers.shift;
        input.pointer_locked = self.pointer_locked;
    }

    /// A finger went down, moved or lifted. The first finger of a gesture is
    /// the pointer, pressed while it is down, so every widget answers to
    /// touch; the others are only in [`Input::touches`].
    fn finger(&mut self, id: u64, position: Point, force: Option<f32>, phase: Phase) {
        match phase {
            Phase::Start => {
                self.touches.retain(|t| t.id != id);
                self.touches.push(Touch { id, position, force });
                if self.primary_touch.is_none() {
                    self.primary_touch = Some(id);
                    self.pointer = Some(position);
                    self.button(0, true);
                }
            }
            Phase::Move => {
                if let Some(t) = self.touches.iter_mut().find(|t| t.id == id) {
                    t.position = position;
                    t.force = force;
                }
                if self.primary_touch == Some(id) {
                    self.pointer = Some(position);
                }
            }
            Phase::End => {
                self.touches.retain(|t| t.id != id);
                if self.primary_touch == Some(id) {
                    self.pointer = Some(position);
                    self.button(0, false);
                }
                if self.touches.is_empty() {
                    self.primary_touch = None;
                }
            }
        }
    }

    /// A key went down. Returns whether the browser should still act on it:
    /// typing, so the text field receives the characters, and the clipboard
    /// shortcuts, so the browser raises its copy, cut and paste events.
    fn key_down(&mut self, name: &str) -> bool {
        let command = self.modifiers.command();
        let Some(key) = key_of(name) else {
            // Keys with no name here (a dead key starting an accent) are the
            // text field's to turn into text.
            return true;
        };
        if !self.held.contains(&key) {
            self.held.push(key);
        }
        self.input.keys.push(key);
        match key {
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
                // The browser's own copy, cut and paste events follow.
                'c' | 'x' | 'v' => return true,
                'z' if self.modifiers.shift => self.input.redo = true,
                'z' => self.input.undo = true,
                'y' => self.input.redo = true,
                's' => self.input.save = true,
                // Reload stays the browser's.
                'r' => return true,
                _ => {}
            },
            // Typed characters reach the text field, which reports them.
            Key::Character(_) | Key::Space => return true,
            // Reload, full screen and the developer tools stay the browser's.
            Key::F(5 | 11 | 12) => return true,
            _ => {}
        }
        false
    }
}

/// The pointer's position over `canvas`, in CSS pixels.
fn position(canvas: &HtmlCanvasElement, client_x: i32, client_y: i32) -> Point {
    let rect = canvas.get_bounding_client_rect();
    Point::new(client_x as f32 - rect.left() as f32, client_y as f32 - rect.top() as f32)
}

fn force(event: &PointerEvent) -> Option<f32> {
    // Pointers without pressure report 0.5 while pressed.
    (event.pointer_type() == "pen" || event.pressure() != 0.5).then(|| event.pressure())
}

fn modifiers_of_pointer(event: &PointerEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        control: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

fn modifiers_of_key(event: &KeyboardEvent) -> Modifiers {
    Modifiers {
        shift: event.shift_key(),
        control: event.ctrl_key(),
        alt: event.alt_key(),
        meta: event.meta_key(),
    }
}

/// The UI key for a DOM `KeyboardEvent.key`; characters are lower case, so a
/// shortcut is the same key with or without shift.
fn key_of(name: &str) -> Option<Key> {
    Some(match name {
        "Escape" => Key::Escape,
        "Enter" => Key::Enter,
        "Tab" => Key::Tab,
        " " | "Spacebar" => Key::Space,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        _ => {
            let mut chars = name.chars();
            let first = chars.next()?;
            if chars.next().is_none() {
                Key::Character(first.to_ascii_lowercase())
            } else if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
                Key::F(n)
            } else {
                return None;
            }
        }
    })
}

fn css_cursor(icon: CursorIcon) -> &'static str {
    match icon {
        CursorIcon::Default => "default",
        CursorIcon::Pointer => "pointer",
        CursorIcon::Text => "text",
        CursorIcon::ResizeHorizontal => "ew-resize",
        CursorIcon::ResizeVertical => "ns-resize",
        CursorIcon::Grab => "grab",
        CursorIcon::Grabbing => "grabbing",
        CursorIcon::NotAllowed => "not-allowed",
        CursorIcon::Crosshair => "crosshair",
        CursorIcon::ResizeDiagonal => "nwse-resize",
        CursorIcon::Help => "help",
    }
}
