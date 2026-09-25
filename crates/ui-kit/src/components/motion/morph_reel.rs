use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use ui_kit_core::morph::{hex, Item, Reel, Scene};
use ui_kit_core::motion::BeatGrid;
use ui_kit_core::{Color, TextAlign};

/// Filter ids have to be unique on the page, so every reel numbers its own.
static NEXT_REEL: AtomicUsize = AtomicUsize::new(0);

/// The morph reel as it is `time` seconds in: one shape through twelve UI
/// states, drawn as SVG from the same scene the immediate renderer draws.
///
/// The reel is a pure function of time, so the caller owns the clock: play
/// it by advancing `time`, scrub it by setting it.
#[component]
pub fn MorphReel(
    /// Seconds from the first downbeat; wraps around, so the reel loops.
    time: f32,
    /// Beats per minute of the grid the reel is timed on.
    #[props(default = 120.0)]
    bpm: f32,
    /// One accent colour as `#rrggbb`, or black and white.
    #[props(default)]
    accent: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] style: Option<String>,
) -> Element {
    let id = use_hook(|| NEXT_REEL.fetch_add(1, Ordering::Relaxed));
    let mut reel = use_signal(|| build(bpm, accent.as_deref()));
    let mut built_for = use_signal(|| (bpm, accent.clone()));
    if *built_for.peek() != (bpm, accent.clone()) {
        built_for.set((bpm, accent.clone()));
        reel.set(build(bpm, accent.as_deref()));
    }
    let scene = reel.read().seek(time);
    let class = format!("uikit-morph-reel {}", class.unwrap_or_default());
    let style = format!(
        "display: block; width: 100%; aspect-ratio: 1; font-family: Geist, Inter, 'Segoe UI', system-ui, sans-serif; {}",
        style.unwrap_or_default()
    );
    rsx! {
        svg {
            class: "{class}",
            style: "{style}",
            view_box: "0 0 {scene.size} {scene.size}",
            role: "img",
            "aria-label": "{reel.read().state_at(time).name()}",
            {render_scene(&scene, id)}
        }
    }
}

fn build(bpm: f32, accent: Option<&str>) -> Reel {
    let accent = accent.and_then(|hex_code| {
        u32::from_str_radix(hex_code.trim_start_matches('#'), 16)
            .ok()
            .map(hex)
    });
    Reel::new(BeatGrid::new(bpm), accent)
}

/// A colour as CSS.
fn css(color: Color) -> String {
    format!(
        "rgba({},{},{},{:.3})",
        (color[0].clamp(0.0, 1.0) * 255.0).round(),
        (color[1].clamp(0.0, 1.0) * 255.0).round(),
        (color[2].clamp(0.0, 1.0) * 255.0).round(),
        color[3].clamp(0.0, 1.0)
    )
}

fn points(points: &[ui_kit_core::Point]) -> String {
    points
        .iter()
        .map(|p| format!("{:.2},{:.2}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every layer of `scene` as SVG groups. A blurred layer is blurred by an
/// SVG filter; its `blur` is a radius, about two standard deviations.
pub fn render_scene(scene: &Scene, reel: usize) -> Element {
    rsx! {
        rect {
            width: "{scene.size}",
            height: "{scene.size}",
            fill: "{css(scene.background)}",
        }
        for (index, layer) in scene.layers.iter().enumerate() {
            if layer.opacity > 0.001 {
                if layer.blur > 0.3 {
                    filter {
                        id: "morph-{reel}-{index}",
                        x: "-50%",
                        y: "-50%",
                        width: "200%",
                        height: "200%",
                        feGaussianBlur { std_deviation: "{layer.blur * 0.5:.2}" }
                    }
                }
                g {
                    opacity: "{layer.opacity:.3}",
                    filter: if layer.blur > 0.3 { format!("url(#morph-{reel}-{index})") } else { String::new() },
                    for (at, item) in layer.items.iter().enumerate() {
                        {render_item(item, reel, index, at)}
                    }
                }
            }
        }
    }
}

fn render_item(item: &Item, reel: usize, layer: usize, index: usize) -> Element {
    match item {
        Item::Shadow {
            rect,
            radius,
            blur,
            color,
        } => {
            let id = format!("morph-{reel}-{layer}-{index}-shadow");
            rsx! {
                filter {
                    id: "{id}",
                    x: "-50%",
                    y: "-50%",
                    width: "200%",
                    height: "200%",
                    feGaussianBlur { std_deviation: "{blur * 0.5:.2}" }
                }
                rect {
                    x: "{rect.x:.2}",
                    y: "{rect.y:.2}",
                    width: "{rect.width:.2}",
                    height: "{rect.height:.2}",
                    rx: "{radius:.2}",
                    fill: "{css(*color)}",
                    filter: "url(#{id})",
                }
            }
        }
        Item::Rect {
            rect,
            radius,
            fill,
            stroke,
        } => rsx! {
            rect {
                x: "{rect.x:.2}",
                y: "{rect.y:.2}",
                width: "{rect.width.max(0.0):.2}",
                height: "{rect.height.max(0.0):.2}",
                rx: "{radius:.2}",
                fill: "{css(*fill)}",
                stroke: stroke.map_or("none".to_string(), |s| css(s.color)),
                stroke_width: stroke.map_or(0.0, |s| s.width),
            }
        },
        Item::Text {
            at,
            text,
            size,
            weight,
            color,
            align,
            caret,
        } => {
            let anchor = match align {
                TextAlign::Left => "start",
                TextAlign::Center => "middle",
                TextAlign::Right => "end",
            };
            rsx! {
                text {
                    x: "{at.x:.2}",
                    y: "{at.y:.2}",
                    dy: "0.36em",
                    font_size: "{size:.2}",
                    font_weight: "{weight}",
                    fill: "{css(*color)}",
                    text_anchor: "{anchor}",
                    style: "white-space: pre",
                    "{text}"
                    if let Some(caret) = caret {
                        tspan { fill: "{css(*caret)}", font_weight: "300", dx: "0.04em", "|" }
                    }
                }
            }
        }
        Item::Path {
            points: path,
            stroke,
            closed,
        } => {
            if *closed {
                rsx! {
                    polygon {
                        points: "{points(path)}",
                        fill: "none",
                        stroke: "{css(stroke.color)}",
                        stroke_width: "{stroke.width:.2}",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                    }
                }
            } else {
                rsx! {
                    polyline {
                        points: "{points(path)}",
                        fill: "none",
                        stroke: "{css(stroke.color)}",
                        stroke_width: "{stroke.width:.2}",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                    }
                }
            }
        }
        Item::Polygon {
            points: outline,
            fill,
            stroke,
        } => rsx! {
            polygon {
                points: "{points(outline)}",
                fill: "{css(*fill)}",
                stroke: stroke.map_or("none".to_string(), |s| css(s.color)),
                stroke_width: stroke.map_or(0.0, |s| s.width),
                stroke_linejoin: "round",
                // The outline goes under the fill, as the immediate renderer draws it.
                paint_order: "stroke",
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_renders_as_one_svg_with_its_text_and_blurs() {
        let html = dioxus_ssr::render_element(rsx! { MorphReel { time: 0.0 } });
        assert!(html.starts_with("<svg"), "{html}");
        assert!(html.contains("viewBox=\"0 0 1440 1440\""));
        assert!(html.contains("Listen now"));
        // Just after the click the loader is fading in, blurred.
        let html = dioxus_ssr::render_element(rsx! { MorphReel { time: 0.53 } });
        assert!(html.contains("feGaussianBlur"));
    }

    #[test]
    fn an_accent_colours_the_tab_indicator() {
        let html = dioxus_ssr::render_element(
            rsx! { MorphReel { time: 8.95, accent: "#3366ff".to_string() } },
        );
        assert!(html.contains("rgba(51,102,25"), "{html}");
    }
}
