//! Immediate UI in logical panel pixels. Hosts own rendering, input and application data.
use crate::spatial::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use ui_kit_core::{Material, Point as Vec2, Rect, Style};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawCommand {
    #[serde(default)]
    pub rich_text: Option<RichTextPaint>,
    pub rect: Rect,
    pub clip: Option<Rect>,
    pub style: Style,
    pub kind: String,
    pub text: String,
    pub value: f32,
    pub hovered: bool,
    pub active: bool,
    pub focused: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RichTextPaint {
    pub document: ui_kit_core::RichText,
    pub cursor: Option<usize>,
    pub anchor: usize,
}
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum TextNavigation {
    Left,
    Right,
    Home,
    End,
    Up,
    Down,
}
#[derive(Default, Clone, Debug, Deserialize)]
pub struct Input {
    pub pointer: Option<Vec2>,
    pub down: bool,
    /// Committed text from the host (IME composition is handled by the host).
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub backspace: bool,
    /// Select the whole focused text field (Ctrl/Cmd+A).
    #[serde(default)]
    pub select_all: bool,
    #[serde(default)]
    pub tab: bool,
    #[serde(default)]
    pub activate: bool,
    /// Signed keyboard steps for a focused semantic slider (e.g. arrow keys).
    #[serde(default)]
    pub step: i32,
    #[serde(default)]
    pub navigation: Option<TextNavigation>,
    #[serde(default)]
    pub extend_selection: bool,
    #[serde(default)]
    pub delete: bool,
    #[serde(default)]
    pub undo: bool,
    #[serde(default)]
    pub redo: bool,
    #[serde(default)]
    pub save: bool,
}
#[derive(Default, Clone, Copy, Debug)]
pub struct Response {
    pub hovered: bool,
    pub active: bool,
    pub clicked: bool,
    pub changed: bool,
}

/// Persist this object between frames; call widgets in paint order with unique stable IDs.
/// Interactive rectangles should not overlap. Values remain owned by the caller.
#[derive(Default)]
pub struct Ui {
    input: Input,
    previous_down: bool,
    capture: Option<String>,
    focus: Option<String>,
    selected_text: Option<String>,
    open_combobox: Option<String>,
    number_text: HashMap<String, String>,
    previous_ids: Vec<String>,
    ids: Vec<String>,
    pub commands: Vec<DrawCommand>,
    overlay_commands: Vec<DrawCommand>,
    in_overlay: bool,
    active_popup: Option<Rect>,
    pub style: Style,
    pub clip: Option<Rect>,
}
impl Ui {
    /// Request focus on a control emitted in this frame.
    pub fn request_focus(&mut self, id: &str) {
        self.focus = Some(id.into());
        self.selected_text = None;
    }
    pub fn focused(&self, id: &str) -> bool {
        self.focus.as_deref() == Some(id)
    }
    /// The input this frame began with. Widgets that are not plain rectangles --
    /// a window frame deciding whether a press starts a drag, for one -- need to
    /// read the pointer without claiming an ID for it.
    pub fn input(&self) -> &Input {
        &self.input
    }
    /// True on the frame the pointer went down, and only that frame.
    pub fn pressed(&self) -> bool {
        self.input.down && !self.previous_down
    }
    pub fn previous_down(&self) -> bool {
        self.previous_down
    }
    pub fn is_combobox_open(&self, id: &str) -> bool {
        self.open_combobox.as_deref() == Some(id)
    }
    pub fn open_combobox_id(&mut self, id: &str) {
        self.open_combobox = Some(id.into());
    }
    pub fn close_combobox(&mut self) {
        self.open_combobox = None;
    }
    pub fn set_in_overlay(&mut self, in_overlay: bool) {
        self.in_overlay = in_overlay;
    }
    pub fn in_overlay(&self) -> bool {
        self.in_overlay
    }
    pub fn set_active_popup(&mut self, rect: Option<Rect>) {
        self.active_popup = rect;
    }
    pub fn active_popup(&self) -> Option<Rect> {
        self.active_popup
    }
    pub fn begin(&mut self, input: Input) {
        self.commands.clear();
        self.overlay_commands.clear();
        self.active_popup = None;
        self.in_overlay = false;
        self.ids.clear();
        if input.tab || (input.down && !self.previous_down) {
            self.selected_text = None;
        }
        if input.tab && !self.previous_ids.is_empty() {
            let next = self
                .focus
                .as_ref()
                .and_then(|id| self.previous_ids.iter().position(|v| v == id))
                .map_or(0, |i| (i + 1) % self.previous_ids.len());
            self.focus = Some(self.previous_ids[next].clone());
        }
        if input.down && !self.previous_down {
            self.focus = None;
        }
        self.input = input;
    }
    pub fn end(&mut self) -> &[DrawCommand] {
        if !self.input.down
            || self
                .capture
                .as_ref()
                .is_some_and(|id| !self.ids.contains(id))
        {
            self.capture = None;
        }
        if self.focus.as_ref().is_some_and(|id| !self.ids.contains(id)) {
            self.focus = None;
        }
        if self
            .open_combobox
            .as_ref()
            .is_some_and(|id| !self.ids.contains(id))
        {
            self.open_combobox = None;
        }
        self.previous_down = self.input.down;
        self.previous_ids.clone_from(&self.ids);
        self.commands.append(&mut self.overlay_commands);
        &self.commands
    }
    /// Reuse capture/focus logic without emitting any drawing commands.
    pub fn interact(&mut self, id: &str, rect: Rect) -> Response {
        assert!(!self.ids.iter().any(|v| v == id), "duplicate UI ID: {id}");
        self.ids.push(id.into());
        let occluded = !self.in_overlay
            && self
                .active_popup
                .is_some_and(|pop| self.input.pointer.is_some_and(|p| pop.contains(p)));
        let hovered = !occluded
            && self
                .input
                .pointer
                .is_some_and(|p| rect.contains(p) && self.clip.is_none_or(|clip| clip.contains(p)));
        if hovered && self.input.down && !self.previous_down && self.capture.is_none() {
            self.capture = Some(id.into());
            self.focus = Some(id.into());
        }
        let captured = self.capture.as_deref() == Some(id);
        Response {
            hovered,
            active: captured && self.input.down,
            clicked: (captured && self.previous_down && !self.input.down && hovered)
                || (self.focus.as_deref() == Some(id) && self.input.activate),
            changed: false,
        }
    }
    /// Emit visuals independently of interaction. Custom renderers can handle custom kinds.
    pub fn paint(
        &mut self,
        kind: &str,
        text: &str,
        rect: Rect,
        value: f32,
        response: Response,
        focused: bool,
    ) {
        let cmd = DrawCommand {
            rich_text: None,
            rect,
            clip: if self.in_overlay { None } else { self.clip },
            style: self.style.clone(),
            kind: kind.into(),
            text: text.into(),
            value,
            hovered: response.hovered,
            active: response.active,
            focused,
        };
        if self.in_overlay {
            self.overlay_commands.push(cmd);
        } else {
            self.commands.push(cmd);
        }
    }
    pub fn panel(&mut self, rect: Rect) {
        self.paint("panel", "", rect, 0.0, Response::default(), false);
    }
    /// Emit a soft back shadow command drawn using signed distance fields and smoothstep.
    pub fn shadow(&mut self, rect: Rect, blur: f32) {
        self.paint("shadow", "", rect, blur, Response::default(), false);
    }
    /// Emit a soft back shadow with specific corner radius, blur radius, and color.
    pub fn shadow_box(&mut self, rect: Rect, radius: f32, blur: f32, color: [f32; 4]) {
        let prev_fill = self.style.fill;
        let prev_radius = self.style.radius;
        self.style.fill = color;
        self.style.radius = radius;
        self.shadow(rect, blur);
        self.style.fill = prev_fill;
        self.style.radius = prev_radius;
    }
    pub fn label(&mut self, rect: Rect, text: &str) {
        self.paint("label", text, rect, 0.0, Response::default(), false);
    }
    pub fn rich_text(&mut self, rect: Rect, paint: RichTextPaint) {
        self.paint(
            "label",
            &paint.document.text(),
            rect,
            0.0,
            Response::default(),
            paint.cursor.is_some(),
        );
        let target = if self.in_overlay {
            self.overlay_commands.last_mut()
        } else {
            self.commands.last_mut()
        };
        if let Some(cmd) = target {
            cmd.rich_text = Some(paint);
        }
    }
    pub fn button(&mut self, id: &str, rect: Rect, text: &str) -> Response {
        let r = self.interact(id, rect);
        self.paint(
            "button",
            text,
            rect,
            0.0,
            r,
            self.focus.as_deref() == Some(id),
        );
        r
    }
    pub fn checkbox(&mut self, id: &str, rect: Rect, text: &str, value: &mut bool) -> Response {
        let mut r = self.interact(id, rect);
        if r.clicked {
            *value = !*value;
            r.changed = true;
        }
        self.paint(
            "checkbox",
            text,
            rect,
            u8::from(*value) as f32,
            r,
            self.focus.as_deref() == Some(id),
        );
        r
    }
    /// Normalized slider. Hosts may map this to any application range.
    pub fn slider(&mut self, id: &str, rect: Rect, text: &str, value: &mut f32) -> Response {
        let mut r = self.interact(id, rect);
        if r.active && rect.width > 0.0 {
            if let Some(p) = self.input.pointer {
                let v = ((p.x - rect.x) / rect.width).clamp(0.0, 1.0);
                r.changed = *value != v;
                *value = v;
            }
        }
        if r.clicked && self.input.activate {
            *value = (*value + 0.1).min(1.0);
            r.changed = true;
        }
        self.paint(
            "slider",
            text,
            rect,
            *value,
            r,
            self.focus.as_deref() == Some(id),
        );
        r
    }
    pub fn text_input(&mut self, id: &str, rect: Rect, value: &mut String) -> Response {
        let mut r = self.interact(id, rect);
        let focused = self.focus.as_deref() == Some(id);
        if focused {
            if self.input.select_all {
                self.selected_text = Some(id.into());
            }
            if self.selected_text.as_deref() == Some(id)
                && (self.input.backspace || !self.input.text.is_empty())
            {
                r.changed |= !value.is_empty();
                value.clear();
                self.selected_text = None;
            }
            if self.input.backspace {
                r.changed |= value.pop().is_some();
            }
            if !self.input.text.is_empty() {
                value.push_str(&self.input.text);
                r.changed = true;
            }
        }
        self.paint(
            "text_input",
            value,
            rect,
            if self.selected_text.as_deref() == Some(id) {
                1.0
            } else {
                0.0
            },
            r,
            focused,
        );
        r
    }
    /// A compact select control. The caller owns the selected value; the UI
    /// keeps only which popup is open between frames.
    pub fn combobox<T: Copy + PartialEq + std::fmt::Display>(
        &mut self,
        id: &str,
        rect: Rect,
        value: &mut T,
        options: &[T],
    ) -> Response {
        crate::combobox::Combobox.show(self, id, rect, value, options)
    }
    /// An integer field whose edit buffer survives frames, unlike a formatted
    /// label. It deliberately accepts values outside a nearby slider's range.
    pub fn number_input_u32(&mut self, id: &str, rect: Rect, value: &mut u32) -> Response {
        if !self.focused(id) {
            self.number_text.insert(id.into(), value.to_string());
        }
        let mut text = self
            .number_text
            .remove(id)
            .unwrap_or_else(|| value.to_string());
        let response = self.text_input(id, rect, &mut text);
        if response.changed {
            if let Ok(parsed) = text.parse::<u32>() {
                *value = parsed;
            }
        }
        self.number_text.insert(id.into(), text);
        response
    }
    pub fn progress(&mut self, rect: Rect, value: f32) {
        self.paint(
            "progress",
            "",
            rect,
            value.clamp(0.0, 1.0),
            Response::default(),
            false,
        );
    }
    pub fn number_picker<
        T: Copy
            + PartialOrd
            + std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + std::fmt::Display,
    >(
        &mut self,
        id: &str,
        rect: Rect,
        value: &mut T,
        min: T,
        max: T,
        step: T,
    ) -> Response {
        crate::number_picker::NumberPicker::new(min, max, step).show(self, id, rect, value)
    }

    /// A flexible XY graph visualizer for curves, functions, and interactive handles.
    pub fn xy_graph(
        &mut self,
        id: &str,
        rect: Rect,
        plot: &mut ui_kit_core::XyPlot,
    ) -> crate::xy_graph::XyGraphResponse {
        crate::xy_graph::XyGraphWidget::new().show(self, id, rect, plot)
    }
}

/// A planar world-space UI. `right` and `down` span the full panel in world units.
/// The same panel texture can be rendered with each eye's view/projection matrix.
pub struct WorldPanel {
    pub origin: Vec3,
    pub right: Vec3,
    pub down: Vec3,
    pub pixels: Vec2,
}
impl WorldPanel {
    pub fn point(&self, p: Vec2) -> Vec3 {
        self.origin + self.right * (p.x / self.pixels.x) + self.down * (p.y / self.pixels.y)
    }
    /// Two-sided ray hit, supporting rotated, scaled and skewed panels.
    pub fn hit(&self, origin: Vec3, direction: Vec3) -> Option<Vec2> {
        if !(self.pixels.x > 0.0 && self.pixels.y > 0.0) {
            return None;
        }
        let n = self.right.cross(self.down);
        let denom = n.dot(direction);
        if !denom.is_finite() || denom.abs() < 1e-8 {
            return None;
        }
        let t = n.dot(self.origin - origin) / denom;
        if t < 0.0 {
            return None;
        }
        let p = origin + direction * t - self.origin;
        let a = self.right.dot(self.right);
        let b = self.right.dot(self.down);
        let c = self.down.dot(self.down);
        let det = a * c - b * b;
        if det.abs() < 1e-12 {
            return None;
        }
        let u = (p.dot(self.right) * c - p.dot(self.down) * b) / det;
        let v = (p.dot(self.down) * a - p.dot(self.right) * b) / det;
        if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
            Some(Vec2::new(u * self.pixels.x, v * self.pixels.y))
        } else {
            None
        }
    }
}
