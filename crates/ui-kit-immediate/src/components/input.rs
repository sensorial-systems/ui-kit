use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::theme::{base, faded, with_alpha};
use super::Label;
use crate::icon::Icon;
use crate::layout::{HorizontalAlignment, VerticalAlignment};
use crate::{Response, Strip, Ui};
use ui_kit_core::{Color, Rect, TextAlign};

/// A single-line text field with a placeholder.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextInput<'a> {
    placeholder: &'a str,
    disabled: bool,
}

impl<'a> TextInput<'a> {
    pub fn new(placeholder: &'a str) -> Self {
        Self {
            placeholder,
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Edit `value`; the response is `changed` when it was typed into.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, value: &mut String) -> Response {
        let p = ui.theme.palette;
        let style = base(ui);
        ui.with_style(style, |ui| {
            if self.disabled {
                ui.paint("text_input", value, rect, 0.0, Response::default(), false);
                return Response::default();
            }
            let response = ui.text_input(id, rect, value);
            if let Some(command) = ui.last_command_mut() {
                // A selection highlight in the accent.
                if command.value > 0.5 {
                    command.style.fill = with_alpha(p.accent, 0.2);
                }
                if value.is_empty() {
                    command.text = self.placeholder.into();
                    command.style.foreground = p.muted;
                }
            }
            response
        })
    }
}

/// A button that opens a list of options, showing the chosen one and a chevron.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Select<'a> {
    value: &'a str,
    disabled: bool,
}

impl<'a> Select<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            value,
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draw the closed select; `clicked` means it should open.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect) -> Response {
        Button::new(self.value)
            .variant(ButtonVariant::Outline)
            .trailing(Icon::ChevronDown)
            .disabled(self.disabled)
            .show(ui, id, rect)
    }
}

/// A box that is ticked or not, with its label beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Checkbox {
    disabled: bool,
}

/// A sliding on/off switch, with its label beside it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Switch {
    disabled: bool,
}

fn toggle(
    ui: &mut Ui,
    id: &str,
    rect: Rect,
    label: &str,
    value: &mut bool,
    disabled: bool,
    switch: bool,
) -> Response {
    let mut response = if disabled {
        Response::default()
    } else {
        ui.interact(id, rect)
    };
    if response.clicked {
        *value = !*value;
        response.changed = true;
    }
    let p = ui.theme.palette;
    let fade = if disabled { 0.5 } else { 1.0 };
    let color = faded(if *value { p.accent } else { p.border }, fade);
    let (width, height) = if switch { (40.0, 22.0) } else { (18.0, 18.0) };
    let (control, text) = rect.split_left(width, 10.0);
    let control = control.center_vertically(height);
    let fill = if switch || *value { color } else { p.surface };
    panel(
        ui,
        control,
        fill,
        color,
        if switch { height * 0.5 } else { 4.0 },
    );
    if switch {
        let knob = control.inset(3.0).align(
            16.0,
            16.0,
            if *value {
                HorizontalAlignment::Right
            } else {
                HorizontalAlignment::Left
            },
            VerticalAlignment::Center,
        );
        panel(ui, knob, [1.0; 4], [0.0; 4], 8.0);
    } else if *value {
        Icon::Check.paint(ui, control.inset(2.0), [1.0, 1.0, 1.0, fade]);
    }
    Label::new()
        .color(faded(p.text, fade))
        .show(ui, text, label);
    response
}

impl Checkbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        label: &str,
        value: &mut bool,
    ) -> Response {
        toggle(ui, id, rect, label, value, self.disabled, false)
    }
}

impl Switch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        label: &str,
        value: &mut bool,
    ) -> Response {
        toggle(ui, id, rect, label, value, self.disabled, true)
    }
}

/// A horizontal slider over a range, snapping to a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider {
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
}

impl Slider {
    pub fn new(min: f32, max: f32) -> Self {
        Self {
            min,
            max,
            step: 0.0,
            disabled: false,
        }
    }

    /// Snap to multiples of `step`; 0 slides freely.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step.max(0.0);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn snap(&self, value: f32) -> f32 {
        let value = if self.step > 0.0 {
            (value / self.step).round() * self.step
        } else {
            value
        };
        value.clamp(self.min.min(self.max), self.max.max(self.min))
    }

    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, value: &mut f32) -> Response {
        let mut response = if self.disabled {
            Response::default()
        } else {
            ui.interact(id, rect)
        };
        let before = *value;
        if response.active {
            if let Some(pt) = ui.input().pointer {
                let fraction = ((pt.x - rect.x) / rect.width.max(1.0)).clamp(0.0, 1.0);
                *value = self.snap(self.min + fraction * (self.max - self.min));
            }
        }
        if ui.focused(id) && !self.disabled {
            let step = if self.step > 0.0 {
                self.step
            } else {
                (self.max - self.min) / 100.0
            };
            *value = self.snap(*value + ui.input().step as f32 * step);
        }
        response.changed = *value != before;
        let p = ui.theme.palette;
        let fraction = ((*value - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        let accent = if self.disabled {
            with_alpha(p.accent, 0.5)
        } else {
            p.accent
        };
        let track = rect.center_vertically(6.0);
        panel(ui, track, p.raised, p.border, 3.0);
        panel(
            ui,
            Rect {
                width: track.width * fraction,
                ..track
            },
            accent,
            [0.0; 4],
            3.0,
        );
        let thumb = Rect::new(track.x + track.width * fraction, track.y + 3.0, 0.0, 0.0);
        panel(
            ui,
            Rect::new(thumb.x - 7.0, thumb.y - 7.0, 14.0, 14.0),
            accent,
            [0.0; 4],
            7.0,
        );
        response
    }
}

/// A colour square that can be clicked, outlined when hovered.
pub fn swatch(ui: &mut Ui, id: &str, rect: Rect, color: Color) -> Response {
    let response = ui.interact(id, rect);
    let p = ui.theme.palette;
    panel(
        ui,
        rect,
        color,
        if response.hovered { p.text } else { p.border },
        6.0,
    );
    response
}

/// A row of digit cells for a one-time code.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OtpInput {
    length: usize,
    cell: f32,
    gap: f32,
}

impl Default for OtpInput {
    fn default() -> Self {
        Self {
            length: 6,
            cell: 42.0,
            gap: 8.0,
        }
    }
}

impl OtpInput {
    pub fn new(length: usize) -> Self {
        Self {
            length,
            ..Self::default()
        }
    }

    /// How wide the cells are together.
    pub fn width(&self) -> f32 {
        self.length as f32 * (self.cell + self.gap) - self.gap
    }

    /// Edit `value`, keeping only digits and at most `length` of them.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, value: &mut String) -> Response {
        let mut response = ui.interact(id, rect);
        let focused = ui.focused(id);
        let before = value.clone();
        if focused {
            if ui.input().backspace {
                value.pop();
            }
            let typed: String = ui.input().text.clone();
            value.extend(typed.chars().filter(|c| c.is_ascii_digit()));
        }
        *value = value
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(self.length)
            .collect();
        response.changed = *value != before;
        let p = ui.theme.palette;
        let mut cells = Strip::row(rect, self.gap);
        for i in 0..self.length {
            let cell = cells.leading(self.cell);
            let active = focused && i == value.chars().count().min(self.length - 1);
            panel(
                ui,
                cell,
                p.surface,
                if active { p.accent } else { p.border },
                8.0,
            );
            let digit = value.chars().nth(i).map(String::from).unwrap_or_default();
            Label::new()
                .size(20.0)
                .weight(500)
                .align(TextAlign::Center)
                .show(ui, cell, &digit);
        }
        response
    }
}
