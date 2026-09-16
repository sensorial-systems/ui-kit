use super::card::panel;
use super::theme::{base, with_alpha, DANGER, INFO, SUCCESS, WARNING};
use super::Label;
use crate::{Response, Strip, Ui};
use ui_kit_core::{Color, Rect, Style, TextAlign};

/// What a badge or notification is about, which decides its colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StatusVariant {
    /// Neutral, in the muted colour.
    #[default]
    Default,
    Success,
    Warning,
    Error,
    Info,
}

impl StatusVariant {
    pub fn color(self, ui: &Ui) -> Color {
        match self {
            StatusVariant::Default => ui.theme.palette.muted,
            StatusVariant::Success => SUCCESS,
            StatusVariant::Warning => WARNING,
            StatusVariant::Error => DANGER,
            StatusVariant::Info => INFO,
        }
    }
}

/// A small coloured tag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Badge {
    variant: StatusVariant,
    color: Option<Color>,
    large: bool,
    ellipsis: bool,
}

impl Badge {
    pub fn new(variant: StatusVariant) -> Self {
        Self {
            variant,
            ..Self::default()
        }
    }

    /// A badge in any colour, not one of the status ones.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Larger text with no tint or border, as a metric reads.
    pub fn large(mut self) -> Self {
        self.large = true;
        self
    }

    /// Keep the text on one line, ending in an ellipsis where it would be
    /// cut, instead of wrapping.
    pub fn ellipsis(mut self) -> Self {
        self.ellipsis = true;
        self
    }

    pub fn style(&self, ui: &Ui) -> Style {
        let color = self.color.unwrap_or_else(|| self.variant.color(ui));
        Style {
            fill: with_alpha(color, if self.large { 0.0 } else { 0.1 }),
            foreground: color,
            stroke: if self.large {
                [0.0; 4]
            } else {
                with_alpha(color, 0.2)
            },
            text_align: TextAlign::Center,
            text_padding: 6.0,
            font_size: if self.large { 15.0 } else { 12.0 },
            font_weight: 600,
            radius: 6.0,
            overflow: if self.ellipsis {
                crate::TextOverflow::Ellipsis
            } else {
                crate::TextOverflow::Wrap
            },
            ..base(ui)
        }
    }

    /// How wide the badge has to be for `text`.
    pub fn width(&self, ui: &Ui, text: &str) -> f32 {
        ui.fit_text(text, &self.style(ui))
    }

    pub fn show(&self, ui: &mut Ui, rect: Rect, text: &str) {
        let style = self.style(ui);
        ui.with_style(style, |ui| {
            ui.paint("button", text, rect, 0.0, Response::default(), false)
        });
    }
}

/// A tinted message box with a title and a line of body text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Notification {
    variant: StatusVariant,
}

impl Notification {
    pub fn new(variant: StatusVariant) -> Self {
        Self { variant }
    }

    /// How tall a notification is.
    pub const HEIGHT: f32 = 66.0;

    pub fn show(&self, ui: &mut Ui, rect: Rect, title: &str, body: &str) {
        let color = self.variant.color(ui);
        panel(
            ui,
            rect,
            with_alpha(color, 0.08),
            with_alpha(color, 0.2),
            10.0,
        );
        let mut lines = Strip::column(rect.inset_by(crate::Insets::symmetric(16.0, 8.0)), 2.0);
        Label::new()
            .weight(600)
            .color(color)
            .show(ui, lines.leading(24.0), title);
        Label::new().size(13.0).show(ui, lines.leading(22.0), body);
    }
}

/// A horizontal bar filled to a fraction.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProgressBar {
    color: Option<Color>,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Draw the bar filled to `value`, from 0 to 1.
    pub fn show(&self, ui: &mut Ui, rect: Rect, value: f32) {
        let p = ui.theme.palette;
        let radius = (rect.height * 0.5).min(4.0);
        panel(ui, rect, p.raised, [0.0; 4], radius);
        panel(
            ui,
            Rect {
                width: rect.width * value.clamp(0.0, 1.0),
                ..rect
            },
            self.color.unwrap_or(p.accent),
            [0.0; 4],
            radius,
        );
    }
}

/// A ring of fading dots that turns with time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spinner {
    size: f32,
    color: Option<Color>,
}

impl Spinner {
    pub fn new(size: f32) -> Self {
        Self { size, color: None }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Draw the spinner in the middle of `rect`, as it is `time` seconds in.
    pub fn show(&self, ui: &mut Ui, rect: Rect, time: f32) {
        let color = self.color.unwrap_or(ui.theme.palette.accent);
        let center = crate::vector::center(rect);
        let dot = 4.0;
        for i in 0..16 {
            let angle = time * 4.0 + i as f32 / 16.0 * std::f32::consts::TAU;
            let radius = self.size * 0.5;
            panel(
                ui,
                Rect::new(
                    center.x + angle.cos() * radius - dot * 0.5,
                    center.y + angle.sin() * radius - dot * 0.5,
                    dot,
                    dot,
                ),
                with_alpha(color, i as f32 / 16.0),
                [0.0; 4],
                dot * 0.5,
            );
        }
    }
}

/// A small dot that says whether something is alive, with a ring that
/// breathes while it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusDot {
    color: Color,
    pulse: bool,
}

impl StatusDot {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            pulse: false,
        }
    }

    /// Animate a fading ring around the dot, as a heartbeat.
    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    /// Draw the dot in the middle of `rect`, `time` seconds in.
    pub fn show(&self, ui: &mut Ui, rect: Rect, time: f32) {
        let middle = crate::vector::center(rect);
        let radius = (rect.width.min(rect.height) * 0.5).max(2.0);
        if self.pulse {
            let phase = (time * 1.2).fract();
            let ring = radius * (1.0 + phase * 1.6);
            crate::vector::circle(
                ui,
                middle,
                ring,
                with_alpha(self.color, 0.35 * (1.0 - phase)),
            );
        }
        crate::vector::circle(ui, middle, radius * 0.55, self.color);
    }
}
