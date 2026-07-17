use dioxus::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeType {
    #[default]
    Straight,
    Bezier,
    Orthogonal,
    CurvedOrthogonal,
    /// A long cubic curve whose endpoint tangents follow the source and target
    /// surface normals.
    OrganicCurved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    #[default]
    None,
    Start,
    End,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdgeData {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub edge_type: EdgeType,
    pub color: Option<String>,
    pub animated: bool,
    pub arrow: ArrowHead,
}

#[component]
pub fn Edge(
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    #[props(default)] edge_type: EdgeType,
    #[props(default)] arrow: ArrowHead,
    #[props(default = false)] animated: bool,
    #[props(into, default)] label: Option<String>,
    #[props(into, default)] color: Option<String>,
    /// Outward unit normal at the source surface. Used by `OrganicCurved`.
    #[props(default)] from_normal: Option<(f64, f64)>,
    /// Outward unit normal at the target surface. Used by `OrganicCurved`.
    #[props(default)] to_normal: Option<(f64, f64)>,
) -> Element {
    let animated_class = if animated { "uikit-graph-edge-path-animated" } else { "" };

    // Calculate path description
    let path_d = match edge_type {
        EdgeType::Straight => {
            format!("M {},{} L {},{}", from_x, from_y, to_x, to_y)
        }
        EdgeType::Bezier => {
            // Horizontal Bezier flow calculation
            let dx = to_x - from_x;
            // Follow the direction of travel so right-to-left mind-map
            // branches do not loop behind their nodes.
            let control_offset = (dx.abs() * 0.5).max(30.0) * dx.signum();
            let cx1 = from_x + control_offset;
            let cy1 = from_y;
            let cx2 = to_x - control_offset;
            let cy2 = to_y;
            format!("M {},{} C {},{} {},{} {},{}", from_x, from_y, cx1, cy1, cx2, cy2, to_x, to_y)
        }
        EdgeType::Orthogonal => {
            let mid_x = from_x + (to_x - from_x) * 0.5;
            format!("M {},{} L {},{} L {},{} L {},{}", from_x, from_y, mid_x, from_y, mid_x, to_y, to_x, to_y)
        }
        EdgeType::CurvedOrthogonal => {
            let dx = to_x - from_x;
            let dy = to_y - from_y;
            let mid_x = from_x + dx * 0.5;
            
            // Corner radius, constrained by available spacing to avoid overshoots
            let r = 12.0_f64.min(dx.abs() * 0.4).min(dy.abs() * 0.4);
            let r_x = r * dx.signum();
            let r_y = r * dy.signum();
            
            format!(
                "M {},{} L {},{} Q {},{} {},{} L {},{} Q {},{} {},{} L {},{}",
                from_x, from_y,
                mid_x - r_x, from_y,
                mid_x, from_y,
                mid_x, from_y + r_y,
                mid_x, to_y - r_y,
                mid_x, to_y,
                mid_x + r_x, to_y,
                to_x, to_y
            )
        }
        EdgeType::OrganicCurved => {
            let dx = to_x - from_x;
            let dy = to_y - from_y;
            if dx.abs() < f64::EPSILON {
                format!("M {},{} L {},{}", from_x, from_y, to_x, to_y)
            } else {
                let distance = (dx * dx + dy * dy).sqrt();
                let normalize = |vector: (f64, f64), fallback: (f64, f64)| {
                    let length = (vector.0 * vector.0 + vector.1 * vector.1).sqrt();
                    if length < f64::EPSILON {
                        fallback
                    } else {
                        (vector.0 / length, vector.1 / length)
                    }
                };
                let radial = (dx / distance, dy / distance);
                let source_normal = normalize(from_normal.unwrap_or(radial), radial);
                let target_fallback = (-radial.0, -radial.1);
                let target_normal = normalize(to_normal.unwrap_or(target_fallback), target_fallback);
                let handle = distance * 0.45;

                // Cubic endpoint tangents are defined by P1-P0 and P3-P2.
                // Aligning those handles with the outward surface normals
                // makes both the stroke and auto-oriented arrowheads meet the
                // node surfaces orthogonally.
                let cx1 = from_x + source_normal.0 * handle;
                let cy1 = from_y + source_normal.1 * handle;
                let cx2 = to_x + target_normal.0 * handle;
                let cy2 = to_y + target_normal.1 * handle;
                format!(
                    "M {},{} C {},{} {},{} {},{}",
                    from_x, from_y, cx1, cy1, cx2, cy2, to_x, to_y
                )
            }
        }
    };

    // Determine markers
    let marker_start = match arrow {
        ArrowHead::Start | ArrowHead::Both => {
            "url(#uikit-marker-start-default)".to_string()
        }
        _ => "none".to_string(),
    };

    let marker_end = match arrow {
        ArrowHead::End | ArrowHead::Both => {
            "url(#uikit-marker-end-default)".to_string()
        }
        _ => "none".to_string(),
    };

    // Label position (center of the edge)
    let label_x = from_x + (to_x - from_x) * 0.5;
    let label_y = from_y + (to_y - from_y) * 0.5;

    let mut path_style = String::new();
    if let Some(col) = color.as_ref() {
        path_style.push_str(&format!("stroke: {};", col));
    }

    rsx! {
        g {
            path {
                class: "uikit-graph-edge-path {animated_class}",
                style: "{path_style}",
                d: "{path_d}",
                marker_start: "{marker_start}",
                marker_end: "{marker_end}"
            }
            if let Some(lbl) = label {
                g {
                    // SVG text with background mask
                    rect {
                        class: "uikit-graph-edge-label-bg",
                        x: label_x - 30.0,
                        y: label_y - 8.0,
                        width: 60.0,
                        height: 16.0,
                        rx: 3
                    }
                    text {
                        class: "uikit-graph-edge-label-text",
                        x: label_x,
                        y: label_y,
                        "{lbl}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn EdgeDefs() -> Element {
    rsx! {
        defs {
            // Marker reference points are the triangle tips. Because edge
            // endpoints are already clipped to node boundaries, this makes the
            // visible arrow tip—not its body—touch the node exactly.
            // End markers
            marker { id: "uikit-marker-end-default", marker_width: "8", marker_height: "8", ref_x: "8", ref_y: "4", orient: "auto", marker_units: "strokeWidth",
                path { d: "M 0 0 L 8 4 L 0 8 z", fill: "context-stroke" }
            }

            // Start markers
            marker { id: "uikit-marker-start-default", marker_width: "8", marker_height: "8", ref_x: "0", ref_y: "4", orient: "auto", marker_units: "strokeWidth",
                path { d: "M 8 0 L 0 4 L 8 8 z", fill: "context-stroke" }
            }
        }
    }
}
