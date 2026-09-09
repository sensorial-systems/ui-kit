//! Stateful controls; painting and input are supplied by the immediate host.
use super::*;

#[derive(Clone, Debug)]
pub struct Calendar {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: String,
    pub minute: String,
}
impl Default for Calendar {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 7,
            day: 16,
            hour: "18".into(),
            minute: "00".into(),
        }
    }
}
impl Calendar {
    pub fn from_value(value: &str) -> Self {
        let parts: Vec<_> = value.split(['-', ' ', ':']).collect();
        if parts.len() != 5 {
            return Self::default();
        }
        let mut c = Self::default();
        if let (Ok(y), Ok(m), Ok(d), Ok(h), Ok(min)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
            parts[3].parse::<u32>(),
            parts[4].parse::<u32>(),
        ) {
            if (1..=9999).contains(&y)
                && (1..=12).contains(&m)
                && d > 0
                && d <= days(y, m)
                && h < 24
                && min < 60
            {
                c = Self {
                    year: y,
                    month: m,
                    day: d,
                    hour: format!("{h:02}"),
                    minute: format!("{min:02}"),
                };
            }
        }
        c
    }
    pub fn change_month(&mut self, delta: i32) {
        let index = (self.year * 12 + self.month as i32 - 1 + delta).clamp(12, 9999 * 12 + 11);
        self.year = index / 12;
        self.month = (index % 12 + 1) as u32;
        self.day = self.day.min(days(self.year, self.month));
    }
    pub fn value(&self) -> Option<String> {
        let hour = self.hour.parse::<u32>().ok()?;
        let minute = self.minute.parse::<u32>().ok()?;
        if hour >= 24 || minute >= 60 {
            return None;
        }
        Some(format!(
            "{:04}-{:02}-{:02} {hour:02}:{minute:02}",
            self.year, self.month, self.day
        ))
    }
}
pub fn days(y: i32, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}
fn weekday(y: i32, m: u32) -> u32 {
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = y - if m < 3 { 1 } else { 0 };
    ((y + y / 4 - y / 100 + y / 400 + offsets[m as usize - 1] + 1) % 7) as u32
}

#[derive(Clone, Debug)]
pub struct ColorEditor {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub alpha: f32,
    pub hex: String,
    pub mode: usize,
}
impl ColorEditor {
    pub fn new(value: &str) -> Self {
        let mut c = Self {
            hue: 0.0,
            saturation: 0.0,
            value: 0.0,
            alpha: 1.0,
            hex: value.into(),
            mode: 0,
        };
        c.read_hex();
        c
    }
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
}
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

impl Gallery {
    pub(super) fn calendar_picker(&mut self, p: &mut Painter, rect: Rect) {
        let c = &mut self.calendar;
        let x = rect.x + 20.0;
        let y = rect.y + 18.0;
        let w = rect.width - 40.0;
        if p.button("calendar-prev", r(x, y, 32.0, 32.0), "‹", 3, false) {
            c.change_month(-1)
        }
        if p.button(
            "calendar-next",
            r(x + w - 32.0, y, 32.0, 32.0),
            "›",
            3,
            false,
        ) {
            c.change_month(1)
        }
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        p.label(
            x + 38.0,
            y,
            w - 76.0,
            32.0,
            &format!("{} {}", months[c.month as usize - 1], c.year),
            17.0,
            600,
            p.palette.fg,
        );
        let cell = w / 7.0;
        let offset = weekday(c.year, c.month);
        for (i, label) in ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
            .iter()
            .enumerate()
        {
            p.label(
                x + i as f32 * cell,
                y + 44.0,
                cell,
                24.0,
                label,
                12.0,
                600,
                p.palette.muted,
            );
        }
        for index in 0..42 {
            let day = index as i32 - offset as i32 + 1;
            let valid = day > 0 && day <= days(c.year, c.month) as i32;
            let shown = if day <= 0 {
                let prev = if c.month == 1 {
                    days(c.year - 1, 12)
                } else {
                    days(c.year, c.month - 1)
                };
                prev as i32 + day
            } else if !valid {
                day - days(c.year, c.month) as i32
            } else {
                day
            };
            if p.button(
                &format!("calendar-day-{index}"),
                r(
                    x + (index % 7) as f32 * cell,
                    y + 74.0 + (index / 7) as f32 * 34.0,
                    cell - 4.0,
                    30.0,
                ),
                &shown.to_string(),
                if valid && day as u32 == c.day { 5 } else { 3 },
                !valid,
            ) {
                c.day = day as u32;
            }
        }
        p.label(x, y + 284.0, 58.0, 32.0, "Time", 13.0, 500, p.palette.fg);
        p.field(
            "calendar-hour",
            r(x + 62.0, y + 284.0, 64.0, 32.0),
            &mut c.hour,
            "HH",
            false,
        );
        p.label(
            x + 130.0,
            y + 284.0,
            12.0,
            32.0,
            ":",
            16.0,
            500,
            p.palette.fg,
        );
        p.field(
            "calendar-minute",
            r(x + 146.0, y + 284.0, 64.0, 32.0),
            &mut c.minute,
            "MM",
            false,
        );
        let value = c.value();
        if value.is_none() {
            p.label(
                x,
                y + 320.0,
                w,
                20.0,
                "Enter hours 0–23 and minutes 0–59.",
                12.0,
                400,
                RED,
            );
        }
        if p.button(
            "calendar-cancel",
            r(x, y + 346.0, 100.0, 34.0),
            "Cancel",
            2,
            false,
        ) {
            self.overlay = None;
        }
        if p.button(
            "calendar-apply",
            r(x + w - 110.0, y + 346.0, 110.0, 34.0),
            "Apply",
            0,
            value.is_none(),
        ) {
            self.date = value.unwrap();
            self.overlay = None;
        }
    }
}
impl Painter<'_> {
    fn color_gradient(&mut self, rect: Rect, stops: &[Color]) {
        let mut parameters = Vec::new();
        let count = stops.len() - 1;
        for i in 0..count {
            for (u, v) in [
                (0.0, 0.0),
                (1.0, 0.0),
                (0.0, 1.0),
                (0.0, 1.0),
                (1.0, 0.0),
                (1.0, 1.0),
            ] {
                let color = stops[i + if u == 1.0 { 1 } else { 0 }];
                parameters.extend_from_slice(&[
                    rect.x + (i as f32 + u) * rect.width / count as f32,
                    rect.y + v * rect.height - self.scroll,
                    color[0],
                    color[1],
                    color[2],
                    color[3],
                ]);
            }
        }
        self.ui.style = Style {
            material: Material::Custom {
                name: "gallery.colored-triangles".into(),
                parameters,
            },
            ..self.base()
        };
        self.ui.paint(
            "triangles",
            "",
            self.screen(rect),
            0.0,
            Response::default(),
            false,
        );
    }
    pub(super) fn color_editor(&mut self, id: &str, rect: Rect, editor: &mut ColorEditor) -> bool {
        let mut changed = false;
        let x = rect.x;
        let y = rect.y;
        let w = rect.width;
        for (i, name) in ["Canvas", "Presets", "Hex", "RGB", "HSL"]
            .iter()
            .enumerate()
        {
            if self.button(
                &format!("{id}-mode-{i}"),
                r(x + i as f32 * w / 5.0, y, w / 5.0 - 4.0, 30.0),
                name,
                if editor.mode == i { 5 } else { 3 },
                false,
            ) {
                editor.mode = i;
            }
            let style = &mut self.ui.commands.last_mut().unwrap().style;
            style.text_padding = 4.0;
            style.font_size = 12.0;
        }
        if editor.mode == 0 {
            let canvas = r(x, y + 42.0, w, 140.0);
            let response = self.interact(&format!("{id}-canvas"), canvas);
            if response.active {
                if let Some(pt) = self.input.pointer {
                    editor.saturation = ((pt.x - canvas.x) / canvas.width).clamp(0.0, 1.0);
                    editor.value =
                        (1.0 - (pt.y - self.screen(canvas).y) / canvas.height).clamp(0.0, 1.0);
                    changed = true;
                }
            }
            self.box_(canvas, hsv(editor.hue, 1.0, 1.0), [0.0; 4], 0.0);
            // A two-dimensional SV field, sampled densely in one batched mesh.
            let mut parameters = Vec::new();
            for row in 0..20 {
                for col in 0..40 {
                    let cr = r(
                        x + col as f32 * w / 40.0,
                        y + 42.0 + row as f32 * 7.0,
                        w / 40.0,
                        7.0,
                    );
                    for (px, py) in [
                        (0.0, 0.0),
                        (1.0, 0.0),
                        (0.0, 1.0),
                        (0.0, 1.0),
                        (1.0, 0.0),
                        (1.0, 1.0),
                    ] {
                        let color = hsv(
                            editor.hue,
                            (col as f32 + px) / 40.0,
                            1.0 - (row as f32 + py) / 20.0,
                        );
                        parameters.extend_from_slice(&[
                            cr.x + px * cr.width,
                            cr.y + py * cr.height - self.scroll,
                            color[0],
                            color[1],
                            color[2],
                            1.0,
                        ]);
                    }
                }
            }
            self.ui.style = Style {
                material: Material::Custom {
                    name: "gallery.colored-triangles".into(),
                    parameters,
                },
                ..self.base()
            };
            self.ui.paint(
                "triangles",
                "",
                self.screen(canvas),
                0.0,
                Response::default(),
                false,
            );
            self.box_(
                r(
                    x + editor.saturation * w - 5.0,
                    y + 42.0 + (1.0 - editor.value) * 140.0 - 5.0,
                    10.0,
                    10.0,
                ),
                [0.0; 4],
                [1.0; 4],
                5.0,
            );
            let before = editor.hue;
            self.slider(
                &format!("{id}-hue"),
                r(x, y + 194.0, w, 24.0),
                &mut editor.hue,
                0.0,
                1.0,
                0.001,
                false,
            );
            changed |= before != editor.hue;
            self.color_gradient(
                r(x, y + 201.0, w, 10.0),
                &(0..=6)
                    .map(|i| hsv(i as f32 / 6.0, 1.0, 1.0))
                    .collect::<Vec<_>>(),
            );
            self.box_(
                r(x + editor.hue * w - 7.0, y + 199.0, 14.0, 14.0),
                [1.0; 4],
                self.palette.border,
                7.0,
            );
            self.label(x, y + 220.0, w, 18.0, "Hue", 12.0, 400, self.palette.muted);
        } else if editor.mode == 1 {
            for (i, color) in [
                BLUE,
                GREEN,
                YELLOW,
                RED,
                rgb(0x8b5cf6),
                rgb(0xec4899),
                rgb(0x06b6d4),
                rgb(0xf97316),
                rgb(0xffffff),
                rgb(0x000000),
                rgb(0x64748b),
                rgb(0xa3e635),
            ]
            .iter()
            .enumerate()
            {
                if self.swatch(
                    &format!("{id}-preset-{i}"),
                    r(
                        x + (i % 6) as f32 * w / 6.0,
                        y + 46.0 + (i / 6) as f32 * 56.0,
                        w / 6.0 - 8.0,
                        44.0,
                    ),
                    *color,
                ) {
                    editor.hex = format!(
                        "#{:02x}{:02x}{:02x}",
                        (color[0] * 255.0).round() as u8,
                        (color[1] * 255.0).round() as u8,
                        (color[2] * 255.0).round() as u8
                    );
                    editor.read_hex();
                    changed = true;
                }
            }
        } else if editor.mode >= 3 {
            let rgb = editor.color();
            let lightness = editor.value * (1.0 - editor.saturation * 0.5);
            let mut channels = if editor.mode == 3 {
                [rgb[0] * 255.0, rgb[1] * 255.0, rgb[2] * 255.0]
            } else {
                [
                    editor.hue * 360.0,
                    if lightness <= 0.0 || lightness >= 1.0 {
                        0.0
                    } else {
                        (editor.value - lightness) / lightness.min(1.0 - lightness) * 100.0
                    },
                    lightness * 100.0,
                ]
            };
            let before = channels;
            let labels = if editor.mode == 3 {
                ["Red", "Green", "Blue"]
            } else {
                ["Hue", "Saturation", "Lightness"]
            };
            for i in 0..3 {
                self.label(
                    x,
                    y + 42.0 + i as f32 * 62.0,
                    w,
                    22.0,
                    &format!("{}: {:.0}", labels[i], channels[i]),
                    12.0,
                    500,
                    self.palette.muted,
                );
                self.slider(
                    &format!("{id}-channel-{i}"),
                    r(x, y + 68.0 + i as f32 * 62.0, w, 24.0),
                    &mut channels[i],
                    0.0,
                    if editor.mode == 3 {
                        255.0
                    } else if i == 0 {
                        360.0
                    } else {
                        100.0
                    },
                    1.0,
                    false,
                );
            }
            if before != channels {
                if editor.mode == 3 {
                    editor.hex = format!(
                        "#{:02x}{:02x}{:02x}{:02x}",
                        channels[0].round() as u8,
                        channels[1].round() as u8,
                        channels[2].round() as u8,
                        (editor.alpha * 255.0).round() as u8
                    );
                    editor.read_hex();
                } else {
                    editor.hue = (channels[0] / 360.0).rem_euclid(1.0);
                    let l = channels[2] / 100.0;
                    editor.value = l + channels[1] / 100.0 * l.min(1.0 - l);
                    editor.saturation = if editor.value == 0.0 {
                        0.0
                    } else {
                        2.0 * (1.0 - l / editor.value)
                    };
                }
                changed = true;
            }
        } else {
            self.label(
                x,
                y + 48.0,
                w,
                50.0,
                "Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA.",
                13.0,
                400,
                self.palette.muted,
            );
        }
        if changed {
            editor.update_hex();
        }
        self.label(
            x,
            y + 244.0,
            60.0,
            28.0,
            "Alpha",
            12.0,
            500,
            self.palette.muted,
        );
        let before = editor.alpha;
        self.slider(
            &format!("{id}-alpha"),
            r(x + 64.0, y + 244.0, w - 64.0, 28.0),
            &mut editor.alpha,
            0.0,
            1.0,
            0.01,
            false,
        );
        if before != editor.alpha {
            editor.update_hex();
            changed = true;
        }
        if self.field(
            &format!("{id}-hex"),
            r(x, y + 284.0, w - 58.0, 34.0),
            &mut editor.hex,
            "#RRGGBB",
            false,
        ) {
            changed |= editor.read_hex();
        }
        for row in 0..4 {
            for col in 0..6 {
                self.box_(
                    r(
                        x + w - 48.0 + col as f32 * 8.0,
                        y + 284.0 + row as f32 * 8.5,
                        8.0,
                        8.5,
                    ),
                    if (row + col) % 2 == 0 {
                        rgb(0xffffff)
                    } else {
                        rgb(0xb0b0b0)
                    },
                    [0.0; 4],
                    0.0,
                );
            }
        }
        self.swatch(
            &format!("{id}-preview"),
            r(x + w - 48.0, y + 284.0, 48.0, 34.0),
            editor.color(),
        );
        if parse_hex(&editor.hex).is_none() {
            self.label(x, y + 324.0, w, 20.0, "Invalid hex color", 12.0, 400, RED);
        }
        changed
    }
}
