use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPickerMode {
    Presets,
    Hex,
    Rgb,
    Hsl,
    Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RgbColor {
    r: u8,
    g: u8,
    b: u8,
    a: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HslColor {
    h: f32, // 0..360
    s: f32, // 0..100
    l: f32, // 0..100
    a: f32, // 0..1
}

fn parse_hex_component(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn parse_color(input: &str) -> RgbColor {
    let s = input.trim();
    if s.starts_with('#') {
        let hex = &s[1..];
        let bytes = hex.as_bytes();
        if bytes.len() == 3 || bytes.len() == 4 {
            let r = parse_hex_component(bytes[0]).unwrap_or(0) * 17;
            let g = parse_hex_component(bytes[1]).unwrap_or(0) * 17;
            let b = parse_hex_component(bytes[2]).unwrap_or(0) * 17;
            let a = if bytes.len() == 4 {
                (parse_hex_component(bytes[3]).unwrap_or(15) * 17) as f32 / 255.0
            } else {
                1.0
            };
            return RgbColor { r, g, b, a };
        } else if bytes.len() >= 6 {
            let r = (parse_hex_component(bytes[0]).unwrap_or(0) << 4) | parse_hex_component(bytes[1]).unwrap_or(0);
            let g = (parse_hex_component(bytes[2]).unwrap_or(0) << 4) | parse_hex_component(bytes[3]).unwrap_or(0);
            let b = (parse_hex_component(bytes[4]).unwrap_or(0) << 4) | parse_hex_component(bytes[5]).unwrap_or(0);
            let a = if bytes.len() >= 8 {
                let alpha_byte = (parse_hex_component(bytes[6]).unwrap_or(15) << 4) | parse_hex_component(bytes[7]).unwrap_or(15);
                alpha_byte as f32 / 255.0
            } else {
                1.0
            };
            return RgbColor { r, g, b, a };
        }
    }
    // Default fallback to blue if unparseable
    RgbColor { r: 59, g: 130, b: 246, a: 1.0 }
}

fn rgb_to_hex(rgb: RgbColor) -> String {
    if (rgb.a - 1.0).abs() < 0.001 {
        format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
    } else {
        let alpha_byte = (rgb.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b, alpha_byte)
    }
}

fn rgb_to_hsl(rgb: RgbColor) -> HslColor {
    let r = rgb.r as f32 / 255.0;
    let g = rgb.g as f32 / 255.0;
    let b = rgb.b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) / 2.0;

    let (h, s) = if delta.abs() < 0.00001 {
        (0.0, 0.0)
    } else {
        let s = if l > 0.5 {
            delta / (2.0 - max - min)
        } else {
            delta / (max + min)
        };

        let h = if (max - r).abs() < 0.00001 {
            (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
        } else if (max - g).abs() < 0.00001 {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };

        (h * 60.0, s)
    };

    HslColor {
        h: h.clamp(0.0, 360.0),
        s: (s * 100.0).round(),
        l: (l * 100.0).round(),
        a: rgb.a,
    }
}

fn hsl_to_rgb(hsl: HslColor) -> RgbColor {
    let mut h = hsl.h;
    if h >= 360.0 {
        h = 359.99;
    } else if h < 0.0 {
        h = 0.0;
    }
    let s = (hsl.s / 100.0).clamp(0.0, 1.0);
    let l = (hsl.l / 100.0).clamp(0.0, 1.0);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    RgbColor {
        r: ((r_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        g: ((g_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        b: ((b_prime + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        a: hsl.a,
    }
}

const DEFAULT_PRESETS: &[&str] = &[
    "#ef4444", "#f97316", "#f59e0b", "#10b981",
    "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6",
    "#ec4899", "#18181b", "#71717a", "#ffffff",
];

#[component]
pub fn ColorPicker(
    value: String,
    on_change: EventHandler<String>,
    #[props(into, default)] label: Option<String>,
    #[props(default = false)] inline: bool,
    #[props(default = false)] disabled: bool,
    #[props(into, default)] preset_colors: Option<Vec<String>>,
    #[props(into, default)] class: String,
) -> Element {
    let mut is_open = use_signal(|| false);
    let mut active_mode = use_signal(|| ColorPickerMode::Canvas);

    let current_rgb = parse_color(&value);
    let current_hsl = rgb_to_hsl(current_rgb);

    let presets = preset_colors.unwrap_or_else(|| {
        DEFAULT_PRESETS.iter().map(|s| s.to_string()).collect()
    });

    let current_hex = rgb_to_hex(current_rgb);

    let update_color = move |new_rgb: RgbColor| {
        let hex = rgb_to_hex(new_rgb);
        on_change.call(hex);
    };

    let panel_content = rsx! {
        div { class: "uikit-color-picker-panel",
            // Tab mode header
            div { class: "uikit-color-picker-tabs",
                button {
                    class: if *active_mode.read() == ColorPickerMode::Canvas { "uikit-color-picker-tab active" } else { "uikit-color-picker-tab" },
                    onclick: move |_| active_mode.set(ColorPickerMode::Canvas),
                    "Picker"
                }
                button {
                    class: if *active_mode.read() == ColorPickerMode::Presets { "uikit-color-picker-tab active" } else { "uikit-color-picker-tab" },
                    onclick: move |_| active_mode.set(ColorPickerMode::Presets),
                    "Presets"
                }
                button {
                    class: if *active_mode.read() == ColorPickerMode::Hex { "uikit-color-picker-tab active" } else { "uikit-color-picker-tab" },
                    onclick: move |_| active_mode.set(ColorPickerMode::Hex),
                    "HEX"
                }
                button {
                    class: if *active_mode.read() == ColorPickerMode::Rgb { "uikit-color-picker-tab active" } else { "uikit-color-picker-tab" },
                    onclick: move |_| active_mode.set(ColorPickerMode::Rgb),
                    "RGB"
                }
                button {
                    class: if *active_mode.read() == ColorPickerMode::Hsl { "uikit-color-picker-tab active" } else { "uikit-color-picker-tab" },
                    onclick: move |_| active_mode.set(ColorPickerMode::Hsl),
                    "HSL"
                }
            }

            // Mode Body
            match *active_mode.read() {
                ColorPickerMode::Canvas => rsx! {
                    div { class: "uikit-color-picker-mode-canvas",
                        // Visual hue/sat area preview
                        div {
                            class: "uikit-color-picker-canvas-box",
                            style: "background-color: hsl({current_hsl.h}, 100%, 50%);",
                            div { class: "uikit-color-picker-canvas-sat" }
                            div { class: "uikit-color-picker-canvas-val" }
                        }
                        // Hue Slider
                        div { class: "uikit-color-picker-slider-row",
                            label { "Hue" }
                            input {
                                r#type: "range",
                                class: "uikit-color-picker-hue-range",
                                min: "0",
                                max: "360",
                                value: "{current_hsl.h}",
                                oninput: move |e| {
                                    if let Ok(h) = e.value().parse::<f32>() {
                                        let new_hsl = HslColor { h, ..current_hsl };
                                        update_color(hsl_to_rgb(new_hsl));
                                    }
                                }
                            }
                        }
                        // Alpha Slider
                        div { class: "uikit-color-picker-slider-row",
                            label { "Opacity" }
                            input {
                                r#type: "range",
                                class: "uikit-color-picker-alpha-range",
                                min: "0",
                                max: "100",
                                value: "{(current_rgb.a * 100.0) as u32}",
                                oninput: move |e| {
                                    if let Ok(val) = e.value().parse::<f32>() {
                                        let mut new_rgb = current_rgb;
                                        new_rgb.a = val / 100.0;
                                        update_color(new_rgb);
                                    }
                                }
                            }
                        }
                    }
                },
                ColorPickerMode::Presets => rsx! {
                    div { class: "uikit-color-picker-mode-presets",
                        div { class: "uikit-color-picker-preset-grid",
                            {presets.iter().map(|preset| {
                                let p = preset.clone();
                                let p_click = preset.clone();
                                let is_selected = p.to_lowercase() == current_hex.to_lowercase();
                                rsx! {
                                    button {
                                        key: "{p}",
                                        class: if is_selected { "uikit-color-picker-swatch active" } else { "uikit-color-picker-swatch" },
                                        style: "background-color: {p};",
                                        title: "{p}",
                                        onclick: move |_| {
                                            let new_rgb = parse_color(&p_click);
                                            update_color(new_rgb);
                                        }
                                    }
                                }
                            })}
                        }
                    }
                },
                ColorPickerMode::Hex => rsx! {
                    div { class: "uikit-color-picker-mode-hex",
                        div { class: "uikit-color-picker-hex-input-group",
                            span { class: "uikit-color-picker-hex-prefix", "#" }
                            input {
                                r#type: "text",
                                class: "uikit-color-picker-input-text",
                                value: "{current_hex.trim_start_matches('#')}",
                                oninput: move |e| {
                                    let raw = e.value();
                                    let formatted = format!("#{}", raw);
                                    let new_rgb = parse_color(&formatted);
                                    update_color(new_rgb);
                                }
                            }
                        }
                    }
                },
                ColorPickerMode::Rgb => rsx! {
                    div { class: "uikit-color-picker-mode-rgb",
                        div { class: "uikit-color-picker-channel-row",
                            span { class: "channel-label", "R" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "255",
                                value: "{current_rgb.r}",
                                oninput: move |e| {
                                    if let Ok(r) = e.value().parse::<u8>() {
                                        update_color(RgbColor { r, ..current_rgb });
                                    }
                                }
                            }
                            span { class: "channel-val", "{current_rgb.r}" }
                        }
                        div { class: "uikit-color-picker-channel-row",
                            span { class: "channel-label", "G" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "255",
                                value: "{current_rgb.g}",
                                oninput: move |e| {
                                    if let Ok(g) = e.value().parse::<u8>() {
                                        update_color(RgbColor { g, ..current_rgb });
                                    }
                                }
                            }
                            span { class: "channel-val", "{current_rgb.g}" }
                        }
                        div { class: "uikit-color-picker-channel-row",
                            span { class: "channel-label", "B" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "255",
                                value: "{current_rgb.b}",
                                oninput: move |e| {
                                    if let Ok(b) = e.value().parse::<u8>() {
                                        update_color(RgbColor { b, ..current_rgb });
                                    }
                                }
                            }
                            span { class: "channel-val", "{current_rgb.b}" }
                        }
                    }
                },
                ColorPickerMode::Hsl => rsx! {
                    div { class: "uikit-color-picker-mode-hsl",
                        div { class: "uikit-color-picker-channel-row",
                            span { class: "channel-label", "H" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "360",
                                value: "{current_hsl.h}",
                                oninput: move |e| {
                                    if let Ok(h) = e.value().parse::<f32>() {
                                        update_color(hsl_to_rgb(HslColor { h, ..current_hsl }));
                                    }
                                }
                            }
                            span { class: "channel-val", "{current_hsl.h as u32}°" }
                        }
                        div { class: "uikit-color-picker-channel-row",
                            span { class: "channel-label", "S" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "100",
                                value: "{current_hsl.s}",
                                oninput: move |e| {
                                    if let Ok(s) = e.value().parse::<f32>() {
                                        update_color(hsl_to_rgb(HslColor { s, ..current_hsl }));
                                    }
                                }
                            }
                            span { class: "channel-val", "{current_hsl.s as u32}%" }
                        }
                        div { class: "uikit-color-picker-channel-row",
                            span { class: "channel-label", "L" }
                            input {
                                r#type: "range",
                                min: "0",
                                max: "100",
                                value: "{current_hsl.l}",
                                oninput: move |e| {
                                    if let Ok(l) = e.value().parse::<f32>() {
                                        update_color(hsl_to_rgb(HslColor { l, ..current_hsl }));
                                    }
                                }
                            }
                            span { class: "channel-val", "{current_hsl.l as u32}%" }
                        }
                    }
                },
            }

            // Preview & Current Value Footer
            div { class: "uikit-color-picker-footer",
                div {
                    class: "uikit-color-picker-preview-dot",
                    style: "background-color: {current_hex};"
                }
                span { class: "uikit-color-picker-value-text", "{current_hex}" }
            }
        }
    };

    rsx! {
        div { class: "uikit-color-picker-wrapper {class}",
            if let Some(lbl) = label {
                label { class: "uikit-color-picker-label", "{lbl}" }
            }

            if inline {
                {panel_content}
            } else {
                div { class: "uikit-color-picker-trigger-container",
                    button {
                        class: if disabled { "uikit-color-picker-trigger disabled" } else { "uikit-color-picker-trigger" },
                        disabled: disabled,
                        onclick: move |_| {
                            if !disabled {
                                let cur = *is_open.read();
                                is_open.set(!cur);
                            }
                        },
                        div {
                            class: "uikit-color-picker-swatch-badge",
                            style: "background-color: {current_hex};"
                        }
                        span { class: "uikit-color-picker-trigger-val", "{current_hex}" }
                    }

                    if *is_open.read() {
                        div { class: "uikit-color-picker-popover",
                            {panel_content}
                        }
                    }
                }
            }
        }
    }
}
