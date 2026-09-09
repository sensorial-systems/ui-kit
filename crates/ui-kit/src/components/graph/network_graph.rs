use crate::components::graph::edge::{Edge, EdgeDefs, EdgeType, GraphEdgeData};
use crate::components::graph::navigation::{GraphNavigation, GraphNavigationNode};
use crate::components::graph::node::{GraphNodeData, Node, NodeShape};
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn NetworkGraph(
    nodes: Vec<(GraphNodeData, Element)>,
    edges: Vec<GraphEdgeData>,
    #[props(default = true)] navigation: bool,
    #[props(default)] active_node_id: Option<String>,
    #[props(default)] on_node_click: EventHandler<String>,
) -> Element {
    let canvas_width = 900.0;
    let canvas_height = 420.0;

    let center_x = canvas_width / 2.0;
    let center_y = canvas_height / 2.0;

    let layout_nodes = nodes
        .iter()
        .map(|(node, _)| ui_kit_core::graph::NetworkNode {
            id: node.id.clone(),
            position: if node.x > 0.01 || node.y > 0.01 {
                Some(ui_kit_core::Point::new(node.x as f32, node.y as f32))
            } else {
                None
            },
        })
        .collect::<Vec<_>>();
    let connections = edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect::<Vec<_>>();
    let layout = ui_kit_core::graph::network_layout(
        &layout_nodes,
        &connections,
        ui_kit_core::Point::new(center_x as f32, center_y as f32),
        135.0,
    );
    let hub_id = layout.hub_id;
    let use_hub_layout = hub_id.is_some();
    let node_positions: HashMap<String, (f64, f64)> = layout
        .positions
        .into_iter()
        .map(|(id, p)| (id, (p.x as f64, p.y as f64)))
        .collect();
    // Render edges
    let rendered_edges = edges.iter().map(|edge| {
        let (fx, fy) = node_positions
            .get(&edge.from)
            .copied()
            .unwrap_or((0.0, 0.0));
        let (tx, ty) = node_positions.get(&edge.to).copied().unwrap_or((0.0, 0.0));

        let shape_for = |id: &str| {
            if use_hub_layout && hub_id.as_deref() == Some(id) {
                NodeShape::Circle
            } else {
                nodes
                    .iter()
                    .find(|(node, _)| node.id == id)
                    .map(|(node, _)| node.shape)
                    .unwrap_or_default()
            }
        };

        let dimensions_for = |id: &str| {
            nodes
                .iter()
                .find(|(node, _)| node.id == id)
                .and_then(|(node, _)| match (node.width, node.height) {
                    (Some(width), Some(height)) => Some((width, height)),
                    _ => None,
                })
        };
        // Clip each end independently against its actual rendered shape and size.
        let (fx_conn, fy_conn) = shape_for(&edge.from).connection_point_with_dimensions(
            (fx, fy),
            (tx, ty),
            dimensions_for(&edge.from),
        );
        let (tx_conn, ty_conn) = shape_for(&edge.to).connection_point_with_dimensions(
            (tx, ty),
            (fx, fy),
            dimensions_for(&edge.to),
        );

        rsx! {
            Edge {
                key: "{edge.from}-{edge.to}",
                from_x: fx_conn,
                from_y: fy_conn,
                to_x: tx_conn,
                to_y: ty_conn,
                edge_type: EdgeType::Straight, // Mesh networks look best with straight connections
                arrow: edge.arrow,
                animated: edge.animated,
                label: edge.label.clone(),
                color: edge.color.clone()
            }
        }
    });

    // Render nodes
    let rendered_nodes = nodes.iter().map(|(node, node_element)| {
        let (x, y) = node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
        let is_selected = active_node_id.as_ref() == Some(&node.id) || node.selected;
        let node_id = node.id.clone();

        // Use NodeShape::Circle for hubs/centers and NodeShape::Pill or NodeShape::Circle for others
        let shape = if Some(node.id.clone()) == hub_id && use_hub_layout {
            NodeShape::Circle
        } else {
            node.shape
        };

        rsx! {
            Node {
                key: "{node.id}",
                id: node.id.clone(),
                x: x,
                y: y,
                color: node.color.clone(),
                border: node.border.clone(),
                background_color: node.background_color.clone(),
                shape: shape,
                width: node.width,
                height: node.height,
                selected: is_selected,
                onclick: move |_| {
                    on_node_click.call(node_id.clone());
                },
                {node_element.clone()}
            }
        }
    });

    let navigation_nodes = nodes
        .iter()
        .map(|(node, _)| {
            let (x, y) = node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
            let shape = if Some(node.id.clone()) == hub_id && use_hub_layout {
                NodeShape::Circle
            } else {
                node.shape
            };
            let (default_width, default_height) = shape.dimensions();
            let width = node.width.unwrap_or(default_width);
            let height = node.height.unwrap_or(default_height);
            GraphNavigationNode {
                x: x - width / 2.0,
                y: y - height / 2.0,
                width,
                height,
                shape,
            }
        })
        .collect();

    if navigation {
        rsx! {
            GraphNavigation {
                canvas_width,
                canvas_height,
                nodes: navigation_nodes,
                canvas_class: "uikit-graph-container",
                canvas_style: "position: relative; width: {canvas_width}px; height: {canvas_height}px;",
                svg {
                    class: "uikit-graph-svg",
                    view_box: "0 0 {canvas_width} {canvas_height}",
                    EdgeDefs {}
                    {rendered_edges}
                }
                div {
                    class: "uikit-graph-nodes-container",
                    {rendered_nodes}
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "uikit-graph-container",
                style: "position: relative; width: {canvas_width}px; height: {canvas_height}px;",
                svg {
                    class: "uikit-graph-svg",
                    view_box: "0 0 {canvas_width} {canvas_height}",
                    EdgeDefs {}
                    {rendered_edges}
                }
                div {
                    class: "uikit-graph-nodes-container",
                    {rendered_nodes}
                }
            }
        }
    }
}
