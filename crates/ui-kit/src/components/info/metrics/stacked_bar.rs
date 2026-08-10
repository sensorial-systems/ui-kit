use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct BarSegment {
    pub label: String,
    pub value: f64,
    pub color: String,
}

impl BarSegment {
    pub fn new(label: impl Into<String>, value: f64, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            color: color.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StackedBarGroup {
    pub group_label: String,
    pub segments: Vec<BarSegment>,
}

impl StackedBarGroup {
    pub fn new(group_label: impl Into<String>, segments: Vec<BarSegment>) -> Self {
        Self {
            group_label: group_label.into(),
            segments,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartOrientation {
    Horizontal,
    Vertical,
}

impl Default for ChartOrientation {
    fn default() -> Self {
        Self::Horizontal
    }
}

struct ProcessedSegment {
    label: String,
    value: f64,
    color: String,
    pct: f64,
    size_style: String,
    value_label: String,
}

struct ProcessedGroup {
    group_label: String,
    total: f64,
    effective_total: f64,
    segments: Vec<ProcessedSegment>,
}

#[component]
pub fn StackedBarChart(
    groups: Vec<StackedBarGroup>,
    #[props(default = ChartOrientation::Horizontal)] orientation: ChartOrientation,
    #[props(default = 30.0)] bar_thickness: f64,
    #[props(default = true)] show_legend: bool,
    #[props(default = true)] show_values: bool,
    #[props(default = false)] normalize_100: bool,
    #[props(into, default)] class: String,
) -> Element {
    let mut legend_items: Vec<(String, String)> = Vec::new();
    for g in &groups {
        for seg in &g.segments {
            if !legend_items.iter().any(|(lbl, _)| lbl == &seg.label) {
                legend_items.push((seg.label.clone(), seg.color.clone()));
            }
        }
    }

    let max_group_total = if normalize_100 {
        100.0
    } else {
        groups
            .iter()
            .map(|g| g.segments.iter().map(|s| s.value.max(0.0)).sum::<f64>())
            .fold(0.0f64, f64::max)
    };

    let is_horizontal = orientation == ChartOrientation::Horizontal;

    let processed_groups: Vec<_> = groups
        .iter()
        .map(|g| {
            let total: f64 = g.segments.iter().map(|s| s.value.max(0.0)).sum();
            let effective_total = if normalize_100 { total } else { max_group_total };

            let segments = g
                .segments
                .iter()
                .map(|seg| {
                    let pct = if normalize_100 {
                        if total > 0.0 { (seg.value / total) * 100.0 } else { 0.0 }
                    } else if max_group_total > 0.0 {
                        (seg.value / max_group_total) * 100.0
                    } else {
                        0.0
                    };

                    let size_style = if is_horizontal {
                        format!("width: {:.2}%;", pct)
                    } else {
                        format!("height: {:.2}%;", pct)
                    };

                    let value_label = if normalize_100 {
                        let p = if total > 0.0 { (seg.value / total) * 100.0 } else { 0.0 };
                        format!("{:.0}%", p)
                    } else {
                        format!("{}", seg.value)
                    };

                    ProcessedSegment {
                        label: seg.label.clone(),
                        value: seg.value,
                        color: seg.color.clone(),
                        pct,
                        size_style,
                        value_label,
                    }
                })
                .collect();

            ProcessedGroup {
                group_label: g.group_label.clone(),
                total,
                effective_total,
                segments,
            }
        })
        .collect();

    rsx! {
        div {
            class: "uikit-chart-container uikit-stacked-bar-container {class}",
            class: if is_horizontal { "uikit-stacked-bar-horizontal" } else { "uikit-stacked-bar-vertical" },

            div { class: "uikit-stacked-bar-chart-body",
                for group in processed_groups {
                    div { key: "{group.group_label}", class: "uikit-stacked-bar-group",
                        div { class: "uikit-stacked-bar-group-label", "{group.group_label}" }

                        div {
                            class: "uikit-stacked-bar-track",
                            style: if is_horizontal { format!("height: {}px;", bar_thickness) } else { format!("width: {}px;", bar_thickness) },

                            if group.effective_total > 0.0 {
                                for segment in group.segments {
                                    div {
                                        key: "{segment.label}",
                                        class: "uikit-stacked-bar-segment",
                                        style: "background-color: {segment.color}; {segment.size_style}",
                                        title: "{segment.label}: {segment.value}",

                                        if show_values && segment.pct > 8.0 {
                                            span { class: "uikit-stacked-bar-segment-label", "{segment.value_label}" }
                                        }
                                    }
                                }
                            }
                        }

                        if !normalize_100 && show_values && is_horizontal {
                            div { class: "uikit-stacked-bar-total-label", "{group.total}" }
                        }
                    }
                }
            }

            if show_legend && !legend_items.is_empty() {
                div { class: "uikit-chart-legend",
                    for (label, color) in legend_items {
                        div { key: "{label}", class: "uikit-chart-legend-item",
                            span {
                                class: "uikit-chart-legend-color",
                                style: "background-color: {color};"
                            }
                            span { class: "uikit-chart-legend-label", "{label}" }
                        }
                    }
                }
            }
        }
    }
}
