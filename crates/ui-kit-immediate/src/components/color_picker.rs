use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::input::{swatch, Slider, TextInput};
use super::popup;
use super::theme::{rgb, DANGER, INFO, SUCCESS, WARNING};
use super::Label;
use crate::vector::fill_colored_triangles;
use crate::{Grid, Insets, Layer, Response, Stack, Strip, Ui};
use ui_kit_core::{Color, Rect};

/// A colour from hue, saturation and value, each 0 to 1.
pub fn hsv(h: f32, s: f32, v: f32) -> Color {
    let f = h.rem_euclid(1.0) * 6.0;
    let i = f.floor() as i32;
    let a = v * (1.0 - s);
    let b = v * (1.0 - (f - i as f32) * s);
    let c = v * (1.0 - (1.0 - (f - i as f32)) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, c, a),
        1 => (b, v, a),
        2 => (a, v, c),
        3 => (a, b, v),
        4 => (c, a, v),
        _ => (v, a, b),
    };
    [r, g, b, 1.0]
}

/// Read `#RGB`, `#RGBA`, `#RRGGBB` or `#RRGGBBAA`.
pub fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim().strip_prefix('#')?;
    if !s.is_ascii() || ![3, 4, 6, 8].contains(&s.len()) {
        return None;
    }
    let bytes = if s.len() <= 4 {
        s.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        s.into()
    };
    let mut result = [1.0; 4];
    for (i, part) in bytes.as_bytes().chunks_exact(2).enumerate() {
        result[i] = u8::from_str_radix(std::str::from_utf8(part).ok()?, 16).ok()? as f32 / 255.0;
    }
    Some(result)
}

/// How a [`ColorPicker`] lets a colour be chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorPickerMode {
    /// A saturation/value field and a hue strip.
    #[default]
    Canvas,
    Presets,
    Hex,
    Rgb,
    Hsl,
}

impl ColorPickerMode {
    pub const ALL: [ColorPickerMode; 5] = [
        ColorPickerMode::Canvas,
        ColorPickerMode::Presets,
        ColorPickerMode::Hex,
        ColorPickerMode::Rgb,
        ColorPickerMode::Hsl,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ColorPickerMode::Canvas => "Canvas",
            ColorPickerMode::Presets => "Presets",
            ColorPickerMode::Hex => "Hex",
            ColorPickerMode::Rgb => "RGB",
            ColorPickerMode::Hsl => "HSL",
        }
    }
}

/// A colour being edited, held as HSV and alpha with its hex text alongside.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorPicker {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
    pub hex: String,
    pub mode: ColorPickerMode,
}

/// The preset swatches.
pub const PRESETS: [Color; 12] = [
    INFO,
    SUCCESS,
    WARNING,
    DANGER,
    rgb(0x8b5cf6),
    rgb(0xec4899),
    rgb(0x06b6d4),
    rgb(0xf97316),
    rgb(0xffffff),
    rgb(0x000000),
    rgb(0x64748b),
    rgb(0xa3e635),
];

impl ColorPicker {
    /// A picker starting at the colour `hex` names.
    pub fn new(hex: &str) -> Self {
        let mut picker = Self {
            hue: 0.0,
            saturation: 0.0,
            value: 0.0,
            alpha: 1.0,
            hex: hex.into(),
            mode: ColorPickerMode::Canvas,
        };
        picker.read_hex();
        picker
    }

    /// Take the colour from the hex text; false if it is not a colour.
    pub fn read_hex(&mut self) -> bool {
        let Some(rgb) = parse_hex(&self.hex) else {
            return false;
        };
        let max = rgb[0].max(rgb[1]).max(rgb[2]);
        let min = rgb[0].min(rgb[1]).min(rgb[2]);
        let d = max - min;
        if d > 0.00001 {
            self.hue = (if max == rgb[0] {
                (rgb[1] - rgb[2]) / d
            } else if max == rgb[1] {
                (rgb[2] - rgb[0]) / d + 2.0
            } else {
                (rgb[0] - rgb[1]) / d + 4.0
            })
            .rem_euclid(6.0)
                / 6.0;
        }
        self.saturation = if max == 0.0 { 0.0 } else { d / max };
        self.value = max;
        self.alpha = rgb[3];
        true
    }

    pub fn color(&self) -> Color {
        let mut c = hsv(self.hue, self.saturation, self.value);
        c[3] = self.alpha;
        c
    }

    /// Write the colour back to the hex text.
    pub fn update_hex(&mut self) {
        let c = self.color();
        self.hex = format!(
            "#{:02x}{:02x}{:02x}",
            (c[0] * 255.0).round() as u8,
            (c[1] * 255.0).round() as u8,
            (c[2] * 255.0).round() as u8
        );
        if self.alpha < 0.999 {
            self.hex += &format!("{:02x}", (self.alpha * 255.0).round() as u8);
        }
    }

    /// How tall the picker is.
    pub const HEIGHT: f32 = 344.0;

    /// Draw the picker in `rect`; returns whether the colour changed.
    pub fn show(&mut self, ui: &mut Ui, id: &str, rect: Rect) -> bool {
        let mut changed = false;
        // Mode tabs, the mode's own controls, then alpha, hex and its validation.
        let (tabs, rest) = rect.split_top(30.0, 12.0);
        let (body, footer) = rest.split_top(190.0, 12.0);
        let mut footer = Stack::new(footer, 12.0);
        for (i, (mode, tab)) in ColorPickerMode::ALL
            .iter()
            .zip(tabs.columns(5, 0.0))
            .enumerate()
        {
            let variant = if self.mode == *mode {
                ButtonVariant::Selected
            } else {
                ButtonVariant::Text
            };
            if Button::new(mode.label())
                .variant(variant)
                .font_size(12.0)
                .padding(4.0)
                .show(
                    ui,
                    &format!("{id}-mode-{i}"),
                    tab.inset_by(Insets::new(0.0, 0.0, 4.0, 0.0)),
                )
                .clicked
            {
                self.mode = *mode;
            }
        }
        match self.mode {
            ColorPickerMode::Canvas => changed |= self.canvas(ui, id, body),
            ColorPickerMode::Presets => {
                let swatches = Grid::columns(body, 6, 0.0).with_cell_height(56.0);
                for (i, color) in PRESETS.iter().enumerate() {
                    let cell = swatches.cell(i).inset_by(Insets::new(0.0, 0.0, 8.0, 12.0));
                    if swatch(ui, &format!("{id}-preset-{i}"), cell, *color).clicked {
                        self.hex = format!(
                            "#{:02x}{:02x}{:02x}",
                            (color[0] * 255.0).round() as u8,
                            (color[1] * 255.0).round() as u8,
                            (color[2] * 255.0).round() as u8
                        );
                        self.read_hex();
                        changed = true;
                    }
                }
            }
            ColorPickerMode::Hex => Label::muted().size(13.0).show(
                ui,
                body.split_top(50.0, 0.0).0,
                "Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA.",
            ),
            ColorPickerMode::Rgb | ColorPickerMode::Hsl => changed |= self.channels(ui, id, body),
        }
        if changed {
            self.update_hex();
        }

        let mut alpha = Strip::row(footer.row(28.0), 4.0);
        Label::muted()
            .size(12.0)
            .weight(500)
            .show(ui, alpha.leading(60.0), "Alpha");
        let before = self.alpha;
        Slider::new(0.0, 1.0).step(0.01).show(
            ui,
            &format!("{id}-alpha"),
            alpha.rest(),
            &mut self.alpha,
        );
        if before != self.alpha {
            self.update_hex();
            changed = true;
        }
        let (hex, invalid) = footer.row(60.0).split_top(34.0, 6.0);
        let (hex, preview) = hex.split_right(48.0, 10.0);
        if TextInput::new("#RRGGBB")
            .show(ui, &format!("{id}-hex"), hex, &mut self.hex)
            .changed
        {
            changed |= self.read_hex();
        }
        // A checkerboard behind the preview shows its transparency.
        let checks = Grid::columns(preview, 6, 0.0).with_cell_height(8.5);
        for row in 0..4 {
            for col in 0..6 {
                let shade = if (row + col) % 2 == 0 {
                    rgb(0xffffff)
                } else {
                    rgb(0xb0b0b0)
                };
                panel(ui, checks.cell(row * 6 + col), shade, [0.0; 4], 0.0);
            }
        }
        swatch(ui, &format!("{id}-preview"), preview, self.color());
        if parse_hex(&self.hex).is_none() {
            Label::new()
                .size(12.0)
                .color(DANGER)
                .show(ui, invalid, "Invalid hex color");
        }
        changed
    }

    fn canvas(&mut self, ui: &mut Ui, id: &str, body: Rect) -> bool {
        let mut changed = false;
        let mut rows = Stack::new(body, 12.0);
        let canvas = rows.row(140.0);
        let response = ui.interact(&format!("{id}-canvas"), canvas);
        if response.active {
            if let Some(pt) = ui.input().pointer {
                self.saturation = ((pt.x - canvas.x) / canvas.width).clamp(0.0, 1.0);
                self.value = (1.0 - (pt.y - canvas.y) / canvas.height).clamp(0.0, 1.0);
                changed = true;
            }
        }
        // A saturation/value field, sampled densely in one mesh.
        let cells = Grid::columns(canvas, 40, 0.0).with_cell_height(7.0);
        let mut vertices = Vec::with_capacity(20 * 40 * 6);
        for row in 0..20 {
            for col in 0..40 {
                let cell = cells.cell(row * 40 + col);
                for (u, v) in [
                    (0.0, 0.0),
                    (1.0, 0.0),
                    (0.0, 1.0),
                    (0.0, 1.0),
                    (1.0, 0.0),
                    (1.0, 1.0),
                ] {
                    let c = hsv(
                        self.hue,
                        (col as f32 + u) / 40.0,
                        1.0 - (row as f32 + v) / 20.0,
                    );
                    vertices.push([
                        cell.x + u * cell.width,
                        cell.y + v * cell.height,
                        c[0],
                        c[1],
                        c[2],
                        1.0,
                    ]);
                }
            }
        }
        fill_colored_triangles(ui, &vertices);
        panel(
            ui,
            Rect::new(
                canvas.x + self.saturation * canvas.width - 5.0,
                canvas.y + (1.0 - self.value) * canvas.height - 5.0,
                10.0,
                10.0,
            ),
            [0.0; 4],
            [1.0; 4],
            5.0,
        );
        let (hue, caption) = rows.row(44.0).split_top(24.0, 2.0);
        let before = self.hue;
        Slider::new(0.0, 1.0)
            .step(0.001)
            .show(ui, &format!("{id}-hue"), hue, &mut self.hue);
        changed |= before != self.hue;
        gradient(
            ui,
            hue.center_vertically(10.0),
            &(0..=6)
                .map(|i| hsv(i as f32 / 6.0, 1.0, 1.0))
                .collect::<Vec<_>>(),
        );
        panel(
            ui,
            Rect::new(hue.x + self.hue * hue.width - 7.0, hue.y + 5.0, 14.0, 14.0),
            [1.0; 4],
            ui.theme.palette.border,
            7.0,
        );
        Label::muted().size(12.0).show(ui, caption, "Hue");
        changed
    }

    fn channels(&mut self, ui: &mut Ui, id: &str, body: Rect) -> bool {
        let rgb_mode = self.mode == ColorPickerMode::Rgb;
        let c = self.color();
        let lightness = self.value * (1.0 - self.saturation * 0.5);
        let mut channels = if rgb_mode {
            [c[0] * 255.0, c[1] * 255.0, c[2] * 255.0]
        } else {
            [
                self.hue * 360.0,
                if lightness <= 0.0 || lightness >= 1.0 {
                    0.0
                } else {
                    (self.value - lightness) / lightness.min(1.0 - lightness) * 100.0
                },
                lightness * 100.0,
            ]
        };
        let before = channels;
        let labels = if rgb_mode {
            ["Red", "Green", "Blue"]
        } else {
            ["Hue", "Saturation", "Lightness"]
        };
        let mut rows = Stack::new(body, 12.0);
        for i in 0..3 {
            let (caption, slider) = rows.row(50.0).split_top(22.0, 4.0);
            Label::muted().size(12.0).weight(500).show(
                ui,
                caption,
                &format!("{}: {:.0}", labels[i], channels[i]),
            );
            let max = if rgb_mode {
                255.0
            } else if i == 0 {
                360.0
            } else {
                100.0
            };
            Slider::new(0.0, max).step(1.0).show(
                ui,
                &format!("{id}-channel-{i}"),
                slider,
                &mut channels[i],
            );
        }
        if before == channels {
            return false;
        }
        if rgb_mode {
            self.hex = format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                channels[0].round() as u8,
                channels[1].round() as u8,
                channels[2].round() as u8,
                (self.alpha * 255.0).round() as u8
            );
            self.read_hex();
        } else {
            self.hue = (channels[0] / 360.0).rem_euclid(1.0);
            let l = channels[2] / 100.0;
            self.value = l + channels[1] / 100.0 * l.min(1.0 - l);
            self.saturation = if self.value == 0.0 {
                0.0
            } else {
                2.0 * (1.0 - l / self.value)
            };
        }
        true
    }
}

/// A horizontal gradient through `stops`, evenly spaced.
fn gradient(ui: &mut Ui, rect: Rect, stops: &[Color]) {
    let count = stops.len().saturating_sub(1).max(1);
    let mut vertices = Vec::with_capacity(count * 6);
    for i in 0..count {
        for (u, v) in [
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (1.0, 1.0),
        ] {
            let c = stops[(i + if u == 1.0 { 1 } else { 0 }).min(stops.len() - 1)];
            vertices.push([
                rect.x + (i as f32 + u) * rect.width / count as f32,
                rect.y + v * rect.height,
                c[0],
                c[1],
                c[2],
                c[3],
            ]);
        }
    }
    fill_colored_triangles(ui, &vertices);
}

/// What a [`ColorField`] keeps between frames.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorFieldState {
    pub open: bool,
    pub picker: ColorPicker,
}

impl Default for ColorFieldState {
    fn default() -> Self {
        Self {
            open: false,
            picker: ColorPicker::new("#000000"),
        }
    }
}

/// A button showing a colour that opens a [`ColorPicker`] in a popover.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorField;

impl ColorField {
    /// The popover's size.
    pub const SIZE: (f32, f32) = (380.0, 408.0);

    /// Draw the field; `changed` means `value` was changed in the popover.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        rect: Rect,
        value: &mut String,
        state: &mut ColorFieldState,
    ) -> Response {
        let mut response = Button::new(value)
            .variant(ButtonVariant::Outline)
            .show(ui, id, rect);
        let dot = rect
            .inset_by(Insets::new(10.0, 0.0, 0.0, 0.0))
            .split_left(18.0, 0.0)
            .0;
        if let Some(color) = parse_hex(value) {
            panel(
                ui,
                dot.center_vertically(18.0),
                color,
                ui.theme.palette.border,
                9.0,
            );
        }
        if response.clicked {
            state.open = !state.open;
            state.picker = ColorPicker::new(value);
        }
        if !state.open {
            return response;
        }
        let (width, height) = Self::SIZE;
        let area = popup::below(ui, rect, 4.0, width, height);
        let done = popup::show(ui, area, Layer::OVERLAY, |ui| {
            let inner = area.inset_by(Insets::new(20.0, 18.0, 20.0, 14.0));
            if state.picker.show(
                ui,
                &format!("{id}-picker"),
                inner.split_top(ColorPicker::HEIGHT, 0.0).0,
            ) {
                response.changed |= *value != state.picker.hex;
                *value = state.picker.hex.clone();
            }
            Button::new("Done")
                .show(
                    ui,
                    &format!("{id}-done"),
                    inner.align(
                        90.0,
                        32.0,
                        crate::HorizontalAlignment::Right,
                        crate::VerticalAlignment::Bottom,
                    ),
                )
                .clicked
        });
        if done || popup::dismissed(ui, &[area, rect]) {
            state.open = false;
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_alpha_and_rejects_nonsense() {
        for value in ["#ff000080", "#3b82f6"] {
            let picker = ColorPicker::new(value);
            let mut copy = picker.clone();
            copy.update_hex();
            assert_eq!(copy.hex, value);
        }
        for value in ["ff0000", "#12", "#gggggg"] {
            assert!(parse_hex(value).is_none());
        }
    }
}
