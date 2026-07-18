use dioxus::prelude::*;
use std::rc::Rc;

use crate::components::graph::node::NodeShape;
use crate::components::input::{Button, ButtonSize, ButtonVariant};

const MIN_ZOOM: f64 = 0.1;
const MAX_ZOOM: f64 = 5.0;
const MINIMAP_WIDTH: f64 = 220.0;
const MINIMAP_HEIGHT: f64 = 150.0;
const MINIMAP_INSET: f64 = 10.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NavigationNode {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub shape: NodeShape,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PinchState {
    start_distance: f64,
    start_zoom: f64,
    start_pan_x: f64,
    start_pan_y: f64,
    center_x: f64,
    center_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MiniMapProjection {
    min_x: f64,
    min_y: f64,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

impl MiniMapProjection {
    fn graph_to_map(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.min_x) * self.scale + self.offset_x,
            (y - self.min_y) * self.scale + self.offset_y,
        )
    }

    fn map_to_graph(self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.offset_x) / self.scale + self.min_x,
            (y - self.offset_y) / self.scale + self.min_y,
        )
    }
}

fn mini_map_projection(
    nodes: &[NavigationNode],
    viewport: (f64, f64, f64, f64),
) -> MiniMapProjection {
    let mut min_x = viewport.0;
    let mut min_y = viewport.1;
    let mut max_x = viewport.2;
    let mut max_y = viewport.3;
    for node in nodes {
        min_x = min_x.min(node.x);
        min_y = min_y.min(node.y);
        max_x = max_x.max(node.x + node.width);
        max_y = max_y.max(node.y + node.height);
    }
    min_x -= 100.0;
    min_y -= 100.0;
    max_x += 100.0;
    max_y += 100.0;

    let content_width = MINIMAP_WIDTH - MINIMAP_INSET * 2.0;
    let content_height = MINIMAP_HEIGHT - MINIMAP_INSET * 2.0;
    let scale =
        (content_width / (max_x - min_x).max(1.0)).min(content_height / (max_y - min_y).max(1.0));
    let drawn_width = (max_x - min_x) * scale;
    let drawn_height = (max_y - min_y) * scale;

    MiniMapProjection {
        min_x,
        min_y,
        scale,
        offset_x: MINIMAP_INSET + (content_width - drawn_width) / 2.0,
        offset_y: MINIMAP_INSET + (content_height - drawn_height) / 2.0,
    }
}

fn fit_transform(nodes: &[NavigationNode], viewport: (f64, f64)) -> Option<(f64, f64, f64)> {
    if nodes.is_empty() {
        return None;
    }

    let min_x = nodes
        .iter()
        .map(|node| node.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = nodes
        .iter()
        .map(|node| node.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = nodes
        .iter()
        .map(|node| node.x + node.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = nodes
        .iter()
        .map(|node| node.y + node.height)
        .fold(f64::NEG_INFINITY, f64::max);

    let margin = 50.0;
    let content_width = (max_x - min_x).max(1.0);
    let content_height = (max_y - min_y).max(1.0);
    let available_width = (viewport.0 - margin * 2.0).max(1.0);
    let available_height = (viewport.1 - margin * 2.0).max(1.0);
    let zoom = (available_width / content_width)
        .min(available_height / content_height)
        .clamp(MIN_ZOOM, 1.0);
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    Some((
        zoom,
        viewport.0 / 2.0 - center_x * zoom,
        viewport.1 / 2.0 - center_y * zoom,
    ))
}

#[component]
pub(crate) fn GraphNavigator(
    canvas_width: f64,
    canvas_height: f64,
    nodes: Vec<NavigationNode>,
    #[props(default = true)] framed: bool,
    #[props(default = false)] center_on_mount: bool,
    #[props(into)] canvas_class: String,
    #[props(into)] canvas_style: String,
    children: Element,
) -> Element {
    let navigation_height = canvas_height.min(480.0);
    let mut pan_x = use_signal(|| 0.0);
    let mut pan_y = use_signal(|| 0.0);
    let mut zoom = use_signal(|| 1.0);
    let mut viewport_size = use_signal(|| (canvas_width, navigation_height));
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    let mut panning = use_signal(|| None::<(f64, f64)>);
    let mut pinching = use_signal(|| None::<PinchState>);
    let mut minimap_dragging = use_signal(|| false);

    let current_pan_x = *pan_x.read();
    let current_pan_y = *pan_y.read();
    let current_zoom = *zoom.read();
    let grid_size = 20.0 * current_zoom;
    let (viewport_width, viewport_height) = *viewport_size.read();

    let viewport_x1 = -current_pan_x / current_zoom;
    let viewport_y1 = -current_pan_y / current_zoom;
    let viewport_x2 = (viewport_width - current_pan_x) / current_zoom;
    let viewport_y2 = (viewport_height - current_pan_y) / current_zoom;

    let map_projection =
        mini_map_projection(&nodes, (viewport_x1, viewport_y1, viewport_x2, viewport_y2));
    let (map_viewport_x1, map_viewport_y1) = map_projection.graph_to_map(viewport_x1, viewport_y1);
    let (map_viewport_x2, map_viewport_y2) = map_projection.graph_to_map(viewport_x2, viewport_y2);
    let fit_nodes = nodes.clone();

    let mut navigate_minimap = move |map_x: f64, map_y: f64| {
        let (width, height) = *viewport_size.peek();
        let (graph_x, graph_y) = map_projection.map_to_graph(map_x, map_y);
        let current_zoom = *zoom.peek();
        pan_x.set(width / 2.0 - graph_x * current_zoom);
        pan_y.set(height / 2.0 - graph_y * current_zoom);
    };

    rsx! {
        div {
            class: if framed { "uikit-graph-navigation uikit-graph-navigation-framed uikit-graph-grid" } else { "uikit-graph-navigation uikit-graph-grid" },
            style: "height: {navigation_height}px; background-position: {current_pan_x}px {current_pan_y}px; background-size: {grid_size}px {grid_size}px;",
            tabindex: "0",
            onmounted: move |event| {
                let data = event.data().clone();
                mounted.set(Some(data.clone()));
                spawn(async move {
                    if let Ok(rect) = data.get_client_rect().await {
                        viewport_size.set((rect.width(), rect.height()));
                        if center_on_mount {
                            pan_x.set((rect.width() - canvas_width) / 2.0);
                            pan_y.set((rect.height() - canvas_height) / 2.0);
                        }
                    }
                });
            },
            onkeydown: move |event: KeyboardEvent| {
                if (event.modifiers().ctrl() || event.modifiers().meta())
                    && event.key().to_string() == "0"
                {
                    event.prevent_default();
                    event.stop_propagation();
                    zoom.set(1.0);
                    pan_x.set(0.0);
                    pan_y.set(0.0);
                }
            },
            oncontextmenu: move |event| {
                if panning.peek().is_some() {
                    event.prevent_default();
                }
            },
            onmousedown: move |event: MouseEvent| {
                if format!("{:?}", event.data().trigger_button()).contains("Auxiliary") {
                    event.prevent_default();
                    event.stop_propagation();
                    let coordinates = event.data().client_coordinates();
                    panning.set(Some((coordinates.x, coordinates.y)));
                }
            },
            onmousemove: move |event: MouseEvent| {
                let last_position = *panning.peek();
                if let Some((last_x, last_y)) = last_position {
                    event.prevent_default();
                    let coordinates = event.data().client_coordinates();
                    pan_x.with_mut(|value| *value += coordinates.x - last_x);
                    pan_y.with_mut(|value| *value += coordinates.y - last_y);
                    panning.set(Some((coordinates.x, coordinates.y)));
                }
            },
            onmouseup: move |_| panning.set(None),
            onmouseleave: move |_| panning.set(None),
            onwheel: move |event: WheelEvent| {
                event.prevent_default();
                event.stop_propagation();
                let delta = event.data().delta().strip_units();
                if event.modifiers().ctrl() || event.modifiers().meta() {
                    if delta.y.abs() < 0.001 {
                        return;
                    }
                    let coordinates = event.data().client_coordinates();
                    let mounted_data = mounted.peek().clone();
                    spawn(async move {
                        let Some(mounted_data) = mounted_data else { return };
                        let Ok(rect) = mounted_data.get_client_rect().await else { return };
                        let old_zoom = *zoom.peek();
                        let new_zoom =
                            (old_zoom * if delta.y > 0.0 { 0.9 } else { 1.1 })
                                .clamp(MIN_ZOOM, MAX_ZOOM);
                        let mouse_x = coordinates.x - rect.min_x();
                        let mouse_y = coordinates.y - rect.min_y();
                        let graph_x = (mouse_x - *pan_x.peek()) / old_zoom;
                        let graph_y = (mouse_y - *pan_y.peek()) / old_zoom;
                        zoom.set(new_zoom);
                        pan_x.set(mouse_x - graph_x * new_zoom);
                        pan_y.set(mouse_y - graph_y * new_zoom);
                    });
                } else {
                    pan_x.with_mut(|value| *value -= delta.x);
                    pan_y.with_mut(|value| *value -= delta.y);
                }
            },
            ontouchstart: move |event: TouchEvent| {
                event.stop_propagation();
                let touches = event.data().touches();
                if touches.len() == 2 {
                    if let (Some(first), Some(second)) = (touches.first(), touches.get(1)) {
                        let first = first.page_coordinates();
                        let second = second.page_coordinates();
                        let dx = second.x - first.x;
                        let dy = second.y - first.y;
                        pinching.set(Some(PinchState {
                            start_distance: (dx * dx + dy * dy).sqrt(),
                            start_zoom: *zoom.peek(),
                            start_pan_x: *pan_x.peek(),
                            start_pan_y: *pan_y.peek(),
                            center_x: (first.x + second.x) / 2.0,
                            center_y: (first.y + second.y) / 2.0,
                        }));
                    }
                }
            },
            ontouchmove: move |event: TouchEvent| {
                event.prevent_default();
                event.stop_propagation();
                let Some(start) = *pinching.peek() else { return };
                let touches = event.data().touches();
                if let (Some(first), Some(second)) = (touches.first(), touches.get(1)) {
                    let first = first.page_coordinates();
                    let second = second.page_coordinates();
                    let dx = second.x - first.x;
                    let dy = second.y - first.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    if start.start_distance <= f64::EPSILON { return; }
                    let new_zoom = (start.start_zoom * distance / start.start_distance)
                        .clamp(MIN_ZOOM, MAX_ZOOM);
                    let center_x = (first.x + second.x) / 2.0;
                    let center_y = (first.y + second.y) / 2.0;
                    let graph_x = (start.center_x - start.start_pan_x) / start.start_zoom;
                    let graph_y = (start.center_y - start.start_pan_y) / start.start_zoom;
                    zoom.set(new_zoom);
                    pan_x.set(center_x - graph_x * new_zoom);
                    pan_y.set(center_y - graph_y * new_zoom);
                }
            },
            ontouchend: move |_| pinching.set(None),
            ontouchcancel: move |_| pinching.set(None),

            div {
                class: "uikit-graph-navigation-canvas",
                style: "width: {canvas_width}px; height: {canvas_height}px; transform: translate({current_pan_x}px, {current_pan_y}px) scale({current_zoom});",
                div {
                    class: "{canvas_class}",
                    style: "{canvas_style}",
                    {children}
                }
            }

            div {
                class: "uikit-graph-navigation-controls",
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Small,
                    onclick: move |event: MouseEvent| {
                        event.stop_propagation();
                        if let Some((new_zoom, new_pan_x, new_pan_y)) =
                            fit_transform(&fit_nodes, *viewport_size.peek())
                        {
                            zoom.set(new_zoom);
                            pan_x.set(new_pan_x);
                            pan_y.set(new_pan_y);
                        }
                    },
                    "Fit Screen"
                }
                Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Small,
                    onclick: move |event: MouseEvent| {
                        event.stop_propagation();
                        zoom.set(1.0);
                        pan_x.set(0.0);
                        pan_y.set(0.0);
                    },
                    "Reset Zoom"
                }
            }

            div {
                class: if *minimap_dragging.read() { "uikit-graph-minimap uikit-graph-minimap-dragging" } else { "uikit-graph-minimap" },
                onpointerdown: move |event: PointerEvent| {
                    event.prevent_default();
                    event.stop_propagation();
                    minimap_dragging.set(true);
                    let coordinates = event.data().element_coordinates();
                    navigate_minimap(coordinates.x, coordinates.y);
                },
                onpointermove: move |event: PointerEvent| {
                    if *minimap_dragging.peek() {
                        event.prevent_default();
                        event.stop_propagation();
                        let coordinates = event.data().element_coordinates();
                        navigate_minimap(coordinates.x, coordinates.y);
                    }
                },
                onpointerup: move |event| {
                    event.stop_propagation();
                    minimap_dragging.set(false);
                },
                onpointerleave: move |_| minimap_dragging.set(false),
                onpointercancel: move |_| minimap_dragging.set(false),
                svg {
                    width: "100%",
                    height: "100%",
                    class: "uikit-graph-minimap-canvas",
                    rect {
                        x: "{map_viewport_x1}",
                        y: "{map_viewport_y1}",
                        width: "{(map_viewport_x2 - map_viewport_x1).max(2.0)}",
                        height: "{(map_viewport_y2 - map_viewport_y1).max(2.0)}",
                        class: "uikit-graph-minimap-viewport",
                    }
                    for (index, node) in nodes.iter().enumerate() {
                        {
                            let (x, y) = map_projection.graph_to_map(node.x, node.y);
                            let width = (node.width * map_projection.scale).max(2.0);
                            let height = (node.height * map_projection.scale).max(2.0);
                            if node.shape == NodeShape::Circle {
                                rsx! {
                                    circle {
                                        key: "minimap-node-{index}",
                                        cx: "{x + width / 2.0}",
                                        cy: "{y + height / 2.0}",
                                        r: "{width.min(height) / 2.0}",
                                        class: "uikit-graph-minimap-node",
                                    }
                                }
                            } else {
                                rsx! {
                                    rect {
                                        key: "minimap-node-{index}",
                                        x: "{x}",
                                        y: "{y}",
                                        width: "{width}",
                                        height: "{height}",
                                        rx: "2",
                                        class: "uikit-graph-minimap-node",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fit_transform, mini_map_projection, NavigationNode, NodeShape, MINIMAP_HEIGHT,
        MINIMAP_WIDTH,
    };

    #[test]
    fn fit_centers_content_with_the_reference_margin() {
        let nodes = vec![NavigationNode {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 100.0,
            shape: NodeShape::Box,
        }];

        let (zoom, pan_x, pan_y) = fit_transform(&nodes, (500.0, 300.0)).unwrap();

        assert_eq!(zoom, 1.0);
        assert_eq!(pan_x, 50.0);
        assert_eq!(pan_y, 0.0);
    }

    #[test]
    fn fit_never_enlarges_or_exceeds_the_reference_bounds() {
        let small = vec![NavigationNode {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            shape: NodeShape::Box,
        }];
        let huge = vec![NavigationNode {
            x: 0.0,
            y: 0.0,
            width: 10_000.0,
            height: 10_000.0,
            shape: NodeShape::Box,
        }];

        assert_eq!(fit_transform(&small, (500.0, 300.0)).unwrap().0, 1.0);
        assert_eq!(fit_transform(&huge, (500.0, 300.0)).unwrap().0, 0.1);
    }

    #[test]
    fn minimap_always_contains_the_complete_viewport() {
        let projection = mini_map_projection(&[], (-800.0, -600.0, 1700.0, 1200.0));
        let top_left = projection.graph_to_map(-800.0, -600.0);
        let bottom_right = projection.graph_to_map(1700.0, 1200.0);

        assert!(top_left.0 >= 0.0 && top_left.1 >= 0.0);
        assert!(bottom_right.0 <= MINIMAP_WIDTH && bottom_right.1 <= MINIMAP_HEIGHT);
    }

    #[test]
    fn minimap_centers_a_fitted_viewport_without_unused_canvas_space() {
        let nodes = vec![NavigationNode {
            x: 300.0,
            y: 100.0,
            width: 200.0,
            height: 100.0,
            shape: NodeShape::Box,
        }];
        let projection = mini_map_projection(&nodes, (200.0, 0.0, 600.0, 300.0));
        let top_left = projection.graph_to_map(200.0, 0.0);
        let bottom_right = projection.graph_to_map(600.0, 300.0);

        assert!(((top_left.0 + bottom_right.0) / 2.0 - MINIMAP_WIDTH / 2.0).abs() < 1e-9);
        assert!(((top_left.1 + bottom_right.1) / 2.0 - MINIMAP_HEIGHT / 2.0).abs() < 1e-9);
    }
}
