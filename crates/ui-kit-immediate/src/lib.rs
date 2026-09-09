//! Immediate host: interaction state and rendering commands, independent of wgpu.
pub mod composition;
pub mod immediate;
pub mod spatial;
pub use immediate::{DrawCommand, Input, Response, RichTextPaint, Ui};
use ui_kit_core::{layout, Event, Kind, Rect, Theme, Value, View};

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
        theme: &Theme,
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
                            format!("Working… {label}")
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
