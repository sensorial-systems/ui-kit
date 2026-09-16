use super::theme::{base, faded, with_alpha};
use crate::icon::Icon;
use crate::layout::{HorizontalAlignment, VerticalAlignment};
use crate::{Response, Ui};
use ui_kit_core::{Rect, Style, TextAlign};

/// How a [`Button`] reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    /// Filled with the accent: the one thing to press.
    #[default]
    Primary,
    /// A soft grey fill.
    Secondary,
    /// A surface with a border.
    Outline,
    /// No fill and no border until hovered.
    Text,
    /// Filled red: pressing it destroys something.
    Danger,
    /// An accent tint and border, for a chosen option or tag.
    Selected,
}

/// How big an icon is next to a label, and how far it sits from the edge.
const ICON: f32 = 16.0;
const ICON_GAP: f32 = 6.0;
const TRAILING_INSET: f32 = 10.0;

/// A clickable button with a label, icons, or both.
#[derive(Clone, Debug, PartialEq)]
pub struct Button<'a> {
    label: &'a str,
    variant: ButtonVariant,
    disabled: bool,
    circular: bool,
    icon: Option<Icon>,
    suffix: Option<Icon>,
    trailing: Option<Icon>,
    align: TextAlign,
    font_size: f32,
    padding: Option<f32>,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            variant: ButtonVariant::default(),
            disabled: false,
            circular: false,
            icon: None,
            suffix: None,
            trailing: None,
            align: TextAlign::Center,
            font_size: 14.0,
            padding: None,
        }
    }

    /// A button showing only `icon`.
    pub fn icon_only(icon: Icon) -> Self {
        Self::new("").icon(icon)
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fully rounded ends: a circle for a square rect, a pill otherwise.
    pub fn circular(mut self) -> Self {
        self.circular = true;
        self
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// An icon right after the label, centred together with it.
    pub fn suffix(mut self, icon: Icon) -> Self {
        self.suffix = Some(icon);
        self
    }

    /// An icon pinned to the button's trailing edge, the label keeping the rest.
    pub fn trailing(mut self, icon: Icon) -> Self {
        self.trailing = Some(icon);
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Space between the label and the button's sides.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The style the button's surface and label are drawn in.
    pub fn style(&self, ui: &Ui, rect: Rect) -> Style {
        let p = ui.theme.palette;
        let mut style = base(ui);
        style.font_weight = 500;
        style.font_size = self.font_size;
        style.text_align = self.align;
        if let Some(padding) = self.padding {
            style.text_padding = padding;
        }
        match self.variant {
            ButtonVariant::Primary => {
                style.fill = p.accent;
                style.stroke = p.accent;
                style.foreground = [1.0; 4];
            }
            ButtonVariant::Secondary => {
                style.fill = p.raised;
                style.stroke = [0.0; 4];
            }
            ButtonVariant::Outline => {}
            ButtonVariant::Text => {
                style.fill = [0.0; 4];
                style.stroke = [0.0; 4];
            }
            ButtonVariant::Danger => {
                style.fill = p.danger;
                style.stroke = p.danger;
                style.foreground = [1.0; 4];
            }
            ButtonVariant::Selected => {
                style.fill = with_alpha(p.accent, 0.15);
                style.stroke = p.accent;
                style.foreground = p.accent;
            }
        }
        if self.circular {
            style.radius = rect.height * 0.5;
        }
        if self.disabled {
            style.fill = faded(style.fill, 0.5);
            style.foreground = faded(style.foreground, 0.5);
            style.stroke = faded(style.stroke, 0.5);
        }
        style
    }

    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect) -> Response {
        let response = if self.disabled {
            Response::default()
        } else {
            ui.interact(id, rect)
        };
        let style = self.style(ui, rect);
        let color = style.foreground;
        // The renderer centres a plain label; anything else is laid out here,
        // so labels line up whether or not a row has an icon.
        let plain = self.icon.is_none()
            && self.suffix.is_none()
            && self.trailing.is_none()
            && self.align == TextAlign::Center;
        let focused = ui.focused(id) && !self.disabled;
        ui.with_style(style.clone(), |ui| {
            let text = if plain { self.label } else { "" };
            ui.paint("button", text, rect, 0.0, response, focused);
        });
        if plain {
            return response;
        }
        let icon_size = ICON.min(rect.height - 8.0).max(8.0);
        let icon_in = |area: Rect| {
            area.align(
                icon_size,
                icon_size,
                HorizontalAlignment::Left,
                VerticalAlignment::Center,
            )
        };
        let mut area = rect.inset_by(crate::Insets::symmetric(style.text_padding, 0.0));
        if let Some(icon) = self.trailing {
            let (rest, slot) = rect
                .inset_by(crate::Insets::new(0.0, 0.0, TRAILING_INSET, 0.0))
                .split_right(icon_size, ICON_GAP);
            icon.paint(ui, slot.center_vertically(icon_size), color);
            area.width = (rest.right() - area.x).max(0.0);
        }
        if self.label.is_empty() && self.suffix.is_none() {
            if let Some(icon) = self.icon {
                icon.paint(ui, rect.centered(icon_size, icon_size), color);
            }
            return response;
        }
        let label = super::Label::new()
            .size(self.font_size)
            .weight(500)
            .color(color)
            .align(TextAlign::Left);
        if self.icon.is_none() && self.suffix.is_none() {
            // Only a trailing icon: the label keeps its alignment in what is left.
            label.align(self.align).show(ui, area, self.label);
            return response;
        }
        let text_width = label.width(ui, self.label).min(area.width);
        let icons = [self.icon, self.suffix].iter().flatten().count() as f32;
        let content = text_width + icons * (icon_size + ICON_GAP);
        let start = match self.align {
            TextAlign::Left => area.x,
            TextAlign::Right => area.right() - content,
            _ => area.x + (area.width - content) * 0.5,
        };
        let mut row = crate::Strip::row(
            Rect::new(
                start.max(area.x),
                rect.y,
                content.min(area.width),
                rect.height,
            ),
            ICON_GAP,
        );
        if let Some(icon) = self.icon {
            icon.paint(ui, icon_in(row.leading(icon_size)), color);
        }
        label.show(ui, row.leading(text_width), self.label);
        if let Some(icon) = self.suffix {
            icon.paint(ui, icon_in(row.leading(icon_size)), color);
        }
        response
    }
}
