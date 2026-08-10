use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct PieChartSlice {
    pub label: String,
    pub value: f64,
    pub color: String,
}

impl PieChartSlice {
    pub fn new(label: impl Into<String>, value: f64, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            color: color.into(),
        }
    }
}

/// Helper to compute SVG arc SVG path `d` string.
fn describe_arc(
    cx: f64,
    cy: f64,
    outer_r: f64,
    inner_r: f64,
    start_angle: f64,
    end_angle: f64,
) -> String {
    let angle_diff = (end_angle - start_angle).abs();
    let effective_end = if angle_diff >= 360.0 - 1e-4 {
        start_angle + 359.999
    } else {
        end_angle
    };

    let start_rad = (start_angle - 90.0).to_radians();
    let end_rad = (effective_end - 90.0).to_radians();

    let x1_out = cx + outer_r * start_rad.cos();
    let y1_out = cy + outer_r * start_rad.sin();
    let x2_out = cx + outer_r * end_rad.cos();
    let y2_out = cy + outer_r * end_rad.sin();

    let large_arc = if (effective_end - start_angle).abs() > 180.0 { 1 } else { 0 };

    if inner_r <= 0.0 {
        format!(
            "M {:.2},{:.2} L {:.2},{:.2} A {:.2},{:.2} 0 {},1 {:.2},{:.2} Z",
            cx, cy, x1_out, y1_out, outer_r, outer_r, large_arc, x2_out, y2_out
        )
    } else {
        let x1_in = cx + inner_r * end_rad.cos();
        let y1_in = cy + inner_r * end_rad.sin();
        let x2_in = cx + inner_r * start_rad.cos();
        let y2_in = cy + inner_r * start_rad.sin();

        format!(
            "M {:.2},{:.2} A {:.2},{:.2} 0 {},1 {:.2},{:.2} L {:.2},{:.2} A {:.2},{:.2} 0 {},0 {:.2},{:.2} Z",
            x1_out, y1_out, outer_r, outer_r, large_arc, x2_out, y2_out, x1_in, y1_in, inner_r, inner_r, large_arc, x2_in, y2_in
        )
    }
}

#[component]
pub fn PieChart(
    data: Vec<PieChartSlice>,
    #[props(default = 200.0)] size: f64,
    #[props(default = true)] show_legend: bool,
    #[props(default = true)] show_labels: bool,
    #[props(into, default)] class: String,
) -> Element {
    let total: f64 = data.iter().map(|s| s.value).sum();
    let cx = size / 2.0;
    let cy = size / 2.0;
    let radius = size * 0.42;

    let mut slices_geometry = Vec::new();
    let mut current_angle = 0.0;

    for slice in &data {
        let sweep = if total > 0.0 {
            (slice.value / total) * 360.0
        } else {
            0.0
        };
        let start_angle = current_angle;
        let end_angle = current_angle + sweep;
        current_angle = end_angle;

        let path = describe_arc(cx, cy, radius, 0.0, start_angle, end_angle);

        let mid_angle = (start_angle + (sweep / 2.0) - 90.0).to_radians();
        let label_radius = radius * 0.65;
        let lx = cx + label_radius * mid_angle.cos();
        let ly = cy + label_radius * mid_angle.sin();
        let pct = if total > 0.0 { (slice.value / total) * 100.0 } else { 0.0 };

        slices_geometry.push((slice, path, lx, ly, pct, sweep));
    }

    let legend_items: Vec<_> = data.iter().map(|slice| {
        let pct_str = if total > 0.0 {
            format!("{:.1}%", (slice.value / total) * 100.0)
        } else {
            "0%".to_string()
        };
        (slice.label.clone(), slice.color.clone(), slice.value, pct_str)
    }).collect();

    rsx! {
        div { class: "uikit-chart-container {class}",
            div { class: "uikit-chart-wrapper",
                svg {
                    width: "{size}",
                    height: "{size}",
                    view_box: "0 0 {size} {size}",
                    class: "uikit-pie-chart-svg",

                    if total <= 0.0 {
                        circle {
                            cx: "{cx}",
                            cy: "{cy}",
                            r: "{radius}",
                            fill: "var(--uikit-muted-bg)",
                            stroke: "var(--uikit-border)",
                            stroke_width: "1"
                        }
                    } else {
                        for (slice, path, lx, ly, pct, sweep) in slices_geometry {
                            g { key: "{slice.label}", class: "uikit-chart-slice-group",
                                path {
                                    d: "{path}",
                                    fill: "{slice.color}",
                                    class: "uikit-chart-slice"
                                }
                                if show_labels && sweep > 15.0 {
                                    text {
                                        x: "{lx}",
                                        y: "{ly}",
                                        text_anchor: "middle",
                                        dominant_baseline: "central",
                                        class: "uikit-chart-slice-label",
                                        "{pct:.0}%"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if show_legend && !legend_items.is_empty() {
                div { class: "uikit-chart-legend",
                    for (label, color, val, pct_str) in legend_items {
                        div { key: "{label}", class: "uikit-chart-legend-item",
                            span {
                                class: "uikit-chart-legend-color",
                                style: "background-color: {color};"
                            }
                            span { class: "uikit-chart-legend-label", "{label}" }
                            span { class: "uikit-chart-legend-value", "{val} ({pct_str})" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn DonutChart(
    data: Vec<PieChartSlice>,
    #[props(default = 200.0)] size: f64,
    #[props(default = 0.6)] inner_radius_ratio: f64,
    #[props(default = true)] show_legend: bool,
    #[props(default = true)] show_labels: bool,
    #[props(into, default)] class: String,
    children: Element,
) -> Element {
    let total: f64 = data.iter().map(|s| s.value).sum();
    let cx = size / 2.0;
    let cy = size / 2.0;
    let outer_radius = size * 0.42;
    let inner_radius = outer_radius * inner_radius_ratio.clamp(0.1, 0.9);

    let mut slices_geometry = Vec::new();
    let mut current_angle = 0.0;

    for slice in &data {
        let sweep = if total > 0.0 {
            (slice.value / total) * 360.0
        } else {
            0.0
        };
        let start_angle = current_angle;
        let end_angle = current_angle + sweep;
        current_angle = end_angle;

        let path = describe_arc(cx, cy, outer_radius, inner_radius, start_angle, end_angle);

        let mid_angle = (start_angle + (sweep / 2.0) - 90.0).to_radians();
        let label_radius = (outer_radius + inner_radius) / 2.0;
        let lx = cx + label_radius * mid_angle.cos();
        let ly = cy + label_radius * mid_angle.sin();
        let pct = if total > 0.0 { (slice.value / total) * 100.0 } else { 0.0 };

        slices_geometry.push((slice, path, lx, ly, pct, sweep));
    }

    let center_diameter = inner_radius * 1.85;

    let legend_items: Vec<_> = data.iter().map(|slice| {
        let pct_str = if total > 0.0 {
            format!("{:.1}%", (slice.value / total) * 100.0)
        } else {
            "0%".to_string()
        };
        (slice.label.clone(), slice.color.clone(), slice.value, pct_str)
    }).collect();

    rsx! {
        div { class: "uikit-chart-container {class}",
            div { class: "uikit-chart-wrapper uikit-donut-chart-wrapper",
                svg {
                    width: "{size}",
                    height: "{size}",
                    view_box: "0 0 {size} {size}",
                    class: "uikit-donut-chart-svg",

                    if total <= 0.0 {
                        path {
                            d: "{describe_arc(cx, cy, outer_radius, inner_radius, 0.0, 360.0)}",
                            fill: "var(--uikit-muted-bg)",
                            stroke: "var(--uikit-border)",
                            stroke_width: "1"
                        }
                    } else {
                        for (slice, path, lx, ly, pct, sweep) in slices_geometry {
                            g { key: "{slice.label}", class: "uikit-chart-slice-group",
                                path {
                                    d: "{path}",
                                    fill: "{slice.color}",
                                    class: "uikit-chart-slice"
                                }
                                if show_labels && sweep > 20.0 {
                                    text {
                                        x: "{lx}",
                                        y: "{ly}",
                                        text_anchor: "middle",
                                        dominant_baseline: "central",
                                        class: "uikit-chart-slice-label",
                                        "{pct:.0}%"
                                    }
                                }
                            }
                        }
                    }
                }

                // Center Info Overlay
                div {
                    class: "uikit-donut-center-info",
                    style: "width: {center_diameter}px; height: {center_diameter}px;",
                    {children}
                }
            }

            if show_legend && !legend_items.is_empty() {
                div { class: "uikit-chart-legend",
                    for (label, color, val, pct_str) in legend_items {
                        div { key: "{label}", class: "uikit-chart-legend-item",
                            span {
                                class: "uikit-chart-legend-color",
                                style: "background-color: {color};"
                            }
                            span { class: "uikit-chart-legend-label", "{label}" }
                            span { class: "uikit-chart-legend-value", "{val} ({pct_str})" }
                        }
                    }
                }
            }
        }
    }
}
