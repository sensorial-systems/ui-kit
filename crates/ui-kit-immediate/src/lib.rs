//! Immediate host: interaction state and rendering commands, independent of wgpu.
pub mod components;
pub mod contacts;
pub mod easing {
    pub use prism_ui::immediate::easing::*;
}
pub mod host;
pub mod icon;
pub mod svg;
pub mod task;
pub mod vector;
pub mod combobox {
    pub use prism_ui::immediate::Combobox;
}
pub mod composition {
    pub use prism_ui::composition::*;
}
pub mod immediate {
    pub use prism_ui::immediate::*;
}
/// Automatic layout: say how big things are, and where they go follows.
///
/// [`Flex`](layout::Flex) solves a run of fixed and filling items in one pass,
/// [`Strip`](layout::Strip) takes items off either end of a bar and leaves the
/// middle to what remains, [`Stack`](layout::Stack) lays rows down a growing
/// column, and [`Grid`](layout::Grid) wraps equal cells into rows. `Rect`
/// carries the alignment, inset and split operations they are built on.
pub mod layout {
    pub use prism_ui::immediate::{Axis, Flex, Grid, Insets, Justify, Size, Stack, Strip};
    pub use prism_ui::{HorizontalAlignment, VerticalAlignment};
}
pub mod number_picker {
    pub use prism_ui::immediate::NumberPicker;
}
pub mod scroll_card {
    pub use prism_ui::immediate::{ScrollCard, ScrollCardResponse};
}
pub mod scroll_container {
    pub use prism_ui::immediate::ScrollContainer;
}
pub mod shadow {
    pub use prism_ui::immediate::Shadow;
}
pub mod spatial {
    pub use prism_ui::immediate::{Vec3, WorldPanel};
}
pub mod splitter {
    pub use prism_ui::immediate::Splitter;
}
pub mod stack {
    pub use prism_ui::immediate::Stack;
}
pub mod xy_graph {
    pub use prism_ui::xy_graph::*;
}
// Everything else the immediate layer has: widgets, theming, input and
// output types, accessibility and the test harness.
pub use combobox::Combobox;
pub use contacts::{Contact, ContactEvent, ContactId, Contacts, Tap};
pub use immediate::{DrawCommand, Input, Layer, Response, RichTextPaint, TextMeasure, Ui};
pub use layout::{
    Axis, Flex, Grid, HorizontalAlignment, Insets, Justify, Size, Strip, VerticalAlignment,
};
pub use number_picker::NumberPicker;
pub use prism_ui::immediate::*;
/// How a line of text sits across its rect; [`Style::text_align`] takes it.
pub use prism_ui::TextAlign;
pub use scroll_card::{ScrollCard, ScrollCardResponse};
pub use scroll_container::ScrollContainer;
pub use shadow::Shadow;
pub use splitter::Splitter;
pub use stack::Stack;
use ui_kit_core::{layout, Event, Kind, Theme as ViewTheme, Value, View};

/// An option of a [`Kind::Select`], as the combobox shows it.
#[derive(Clone, Copy, PartialEq)]
struct Choice<'a> {
    index: usize,
    label: &'a str,
}
impl std::fmt::Display for Choice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label)
    }
}
pub use xy_graph::{XyGraphResponse, XyGraphWidget};

#[derive(Default)]
pub struct ImmediateHost {
    ui: Ui,
}
pub struct Frame<A> {
    pub commands: Vec<DrawCommand>,
    pub actions: Vec<A>,
}
impl ImmediateHost {
    pub fn frame<A>(
        &mut self,
        view: &View<A>,
        theme: &ViewTheme,
        input: Input,
        bounds: Rect,
    ) -> Result<Frame<A>, String> {
        view.validate()?;
        if ![bounds.x, bounds.y, bounds.width, bounds.height]
            .iter()
            .all(|v| v.is_finite())
            || bounds.width <= 0.0
            || bounds.height <= 0.0
        {
            return Err("invalid viewport".into());
        }
        let keyboard_step = input.step;
        self.ui.begin(input);
        let mut actions = vec![];
        for placed in layout(view, bounds) {
            let node = placed.view;
            let r = placed.rect;
            let id = node.id.as_deref().unwrap_or("");
            self.ui.style = node.resolved_style(theme).clone();
            self.ui.clip = Some(placed.clip);
            let event = if placed.disabled && node.is_control() {
                let (kind, label, value) = match &node.kind {
                    Kind::Button { label, loading } => (
                        "button",
                        if *loading {
                            format!("Working\u{2026} {label}")
                        } else {
                            label.clone()
                        },
                        0.0,
                    ),
                    Kind::Checkbox { label, checked } => {
                        ("checkbox", label.clone(), u8::from(*checked) as f32)
                    }
                    Kind::Slider {
                        label,
                        value,
                        range,
                    } => ("slider", label.clone(), range.fraction(*value)),
                    Kind::TextInput { value, .. } => ("text_input", value.clone(), 0.0),
                    Kind::Toggle { label, on } => ("checkbox", label.clone(), u8::from(*on) as f32),
                    Kind::Select {
                        options, selected, ..
                    } => (
                        "button",
                        options.get(*selected).cloned().unwrap_or_default(),
                        0.0,
                    ),
                    _ => unreachable!(),
                };
                self.ui.style.fill[3] *= 0.5;
                self.ui.style.foreground[3] *= 0.5;
                self.ui
                    .paint(kind, &label, r, value, Response::default(), false);
                None
            } else {
                match &node.kind {
                    Kind::Column | Kind::Row => None,
                    Kind::Separator => {
                        self.ui.separator(r);
                        None
                    }
                    Kind::Progress { label, value } => {
                        let (caption, bar) = r.split_left((r.width * 0.35).min(160.0), 12.0);
                        self.ui.label(caption, label);
                        self.ui.progress(bar.center_vertically(8.0), *value as f32);
                        None
                    }
                    Kind::Toggle { label, on } => {
                        let mut value = *on;
                        Toggle::show(&mut self.ui, id, r, label, &mut value)
                            .changed
                            .then_some(Value::Toggle(value))
                    }
                    Kind::Select {
                        options, selected, ..
                    } => {
                        let choices: Vec<Choice> = options
                            .iter()
                            .enumerate()
                            .map(|(index, label)| Choice { index, label })
                            .collect();
                        let mut value = choices.get(*selected).copied().unwrap_or(Choice {
                            index: 0,
                            label: "",
                        });
                        self.ui
                            .combobox(id, r, &mut value, &choices)
                            .changed
                            .then_some(Value::Select(value.index))
                    }
                    Kind::Panel => {
                        self.ui.panel(r);
                        None
                    }
                    Kind::Text(text) => {
                        self.ui.label(r, text);
                        None
                    }
                    Kind::Button { label, .. } => self
                        .ui
                        .button(id, r, label)
                        .clicked
                        .then_some(Value::Activate),
                    Kind::Checkbox { label, checked } => {
                        let mut value = *checked;
                        self.ui
                            .checkbox(id, r, label, &mut value)
                            .changed
                            .then_some(Value::Toggle(value))
                    }
                    Kind::Slider {
                        label,
                        value,
                        range,
                    } => {
                        let mut fraction = range.fraction(*value);
                        let response = self.ui.slider(id, r, label, &mut fraction);
                        if self.ui.focused(id) && keyboard_step != 0 {
                            Some(Value::Number(*value + keyboard_step as f64 * range.step))
                        } else {
                            response.changed.then(|| {
                                Value::Number(range.min + fraction as f64 * (range.max - range.min))
                            })
                        }
                    }
                    Kind::TextInput {
                        value, placeholder, ..
                    } => {
                        let mut value = value.clone();
                        let response = self.ui.text_input(id, r, &mut value);
                        if value.is_empty() {
                            if let Some(command) = self.ui.commands.last_mut() {
                                command.text = placeholder.clone();
                                command.style.foreground[3] *= 0.6;
                            }
                        }
                        response.changed.then_some(Value::Text(value))
                    }
                }
            };
            if let Some(value) = event {
                if let Some(action) = view.dispatch(&Event {
                    id: id.into(),
                    value,
                }) {
                    actions.push(action)
                }
            }
        }
        Ok(Frame {
            commands: self.ui.end().to_vec(),
            actions,
        })
    }
}
