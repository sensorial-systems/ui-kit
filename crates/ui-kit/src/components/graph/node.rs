use dioxus::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeShape {
    #[default]
    Box,
    Pill,
    Circle,
    Underline,
    Plain,
}

impl NodeShape {
    fn geometry(self) -> (f64, f64, bool) {
        match self {
            NodeShape::Circle => (35.0, 35.0, true),
            NodeShape::Pill => (70.0, 19.5, true),
            NodeShape::Box => (75.0, 32.5, false),
            NodeShape::Underline | NodeShape::Plain => (80.0, 13.75, false),
        }
    }

    pub(crate) fn dimensions(self) -> (f64, f64) {
        let (half_width, half_height, _) = self.geometry();
        (half_width * 2.0, half_height * 2.0)
    }

    pub fn class_name(&self) -> &'static str {
        match self {
            NodeShape::Box => "uikit-graph-node-box",
            NodeShape::Pill => "uikit-graph-node-pill",
            NodeShape::Circle => "uikit-graph-node-circle",
            NodeShape::Underline => "uikit-graph-node-underline",
            NodeShape::Plain => "uikit-graph-node-plain",
        }
    }

    /// Return the point where a line aimed at `toward` meets this node.
    ///
    /// Graph positions describe node centers, so edges must be clipped to the
    /// node boundary instead of being moved by a fixed radius. The latter only
    /// works for circles and was the source of the visibly displaced anchors
    /// on pills, boxes, and plain nodes.
    pub(crate) fn connection_point(self, center: (f64, f64), toward: (f64, f64)) -> (f64, f64) {
        let dx = toward.0 - center.0;
        let dy = toward.1 - center.1;

        if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
            return center;
        }

        let (half_width, half_height, elliptical) = self.geometry();

        let scale = if elliptical {
            1.0 / ((dx / half_width).powi(2) + (dy / half_height).powi(2)).sqrt()
        } else {
            (half_width / dx.abs()).min(half_height / dy.abs())
        };

        (center.0 + dx * scale, center.1 + dy * scale)
    }

    /// Return the outward unit normal of this shape at a connection point.
    /// Organic curves use this for surface-orthogonal tangents and arrows.
    pub(crate) fn connection_normal(
        self,
        center: (f64, f64),
        point: (f64, f64),
    ) -> (f64, f64) {
        let (half_width, half_height, elliptical) = self.geometry();
        let dx = point.0 - center.0;
        let dy = point.1 - center.1;
        let (nx, ny) = if elliptical {
            (dx / half_width.powi(2), dy / half_height.powi(2))
        } else {
            let x_ratio = dx.abs() / half_width;
            let y_ratio = dy.abs() / half_height;
            if (x_ratio - y_ratio).abs() < 1e-9 {
                (dx / half_width, dy / half_height)
            } else if x_ratio > y_ratio {
                (dx.signum(), 0.0)
            } else {
                (0.0, dy.signum())
            }
        };
        let length = (nx * nx + ny * ny).sqrt();
        if length < f64::EPSILON {
            (0.0, 0.0)
        } else {
            (nx / length, ny / length)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NodeShape;

    #[test]
    fn clips_connections_to_each_shape_boundary() {
        assert_eq!(
            NodeShape::Circle.connection_point((100.0, 100.0), (200.0, 100.0)),
            (135.0, 100.0)
        );
        assert_eq!(
            NodeShape::Pill.connection_point((100.0, 100.0), (200.0, 100.0)),
            (170.0, 100.0)
        );
        assert_eq!(
            NodeShape::Box.connection_point((100.0, 100.0), (100.0, 200.0)),
            (100.0, 132.5)
        );
        assert_eq!(
            NodeShape::Plain.connection_point((100.0, 100.0), (200.0, 100.0)),
            (180.0, 100.0)
        );
    }

    #[test]
    fn connection_normals_are_orthogonal_to_surfaces() {
        let box_point = NodeShape::Box.connection_point((100.0, 100.0), (200.0, 150.0));
        assert_eq!(NodeShape::Box.connection_normal((100.0, 100.0), box_point), (0.0, 1.0));

        let circle_point = NodeShape::Circle.connection_point((0.0, 0.0), (100.0, 100.0));
        let normal = NodeShape::Circle.connection_normal((0.0, 0.0), circle_point);
        let expected = 1.0 / 2.0_f64.sqrt();
        assert!((normal.0 - expected).abs() < 1e-9);
        assert!((normal.1 - expected).abs() < 1e-9);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNodeData {
    pub id: String,
    pub x: f64,
    pub y: f64,
    /// Accent color used by the node shape (kept for backwards compatibility).
    pub color: Option<String>,
    /// A complete CSS border declaration, such as `"2px solid #8b5cf6"` or
    /// `"none"`. When omitted, the selected shape's default border is used.
    pub border: Option<String>,
    /// CSS background color. When omitted, the selected shape's default is used.
    pub background_color: Option<String>,
    pub shape: NodeShape,
    pub selected: bool,
}

#[component]
pub fn Node(
    id: String,
    x: f64,
    y: f64,
    #[props(into, default)] color: Option<String>,
    #[props(into, default)] border: Option<String>,
    #[props(into, default)] background_color: Option<String>,
    #[props(default)] shape: NodeShape,
    #[props(default = false)] selected: bool,
    children: Element,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    let shape_class = shape.class_name();
    let selected_class = if selected {
        "uikit-graph-node-selected"
    } else {
        ""
    };

    let mut custom_style = String::new();
    if let Some(col) = color.as_ref() {
        custom_style.push_str(&format!("--uikit-graph-node-color: {};", col));
        match shape {
            NodeShape::Underline => {
                custom_style.push_str(&format!("border-bottom-color: {};", col));
            }
            NodeShape::Plain => {}
            _ => {
                custom_style.push_str(&format!("border-color: {};", col));
            }
        }
    }
    if let Some(value) = border.as_ref() {
        custom_style.push_str(&format!("border: {};", value));
    }
    if let Some(value) = background_color.as_ref() {
        custom_style.push_str(&format!("background-color: {};", value));
    }

    rsx! {
        div {
            class: "uikit-graph-node-wrapper",
            style: "left: {x}px; top: {y}px;",
            onclick: move |e| onclick.call(e),
            div {
                class: "uikit-graph-node {shape_class} {selected_class}",
                style: "{custom_style}",
                {children}
            }
        }
    }
}
