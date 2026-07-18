use crate::components::graph::edge::{Edge, EdgeDefs, EdgeType, GraphEdgeData};
use crate::components::graph::navigation::{GraphNavigator, NavigationNode};
use crate::components::graph::node::{GraphNodeData, Node, NodeShape};
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn NetworkGraph(
    nodes: Vec<(GraphNodeData, Element)>,
    edges: Vec<GraphEdgeData>,
    #[props(default)] active_node_id: Option<String>,
    #[props(default)] on_node_click: EventHandler<String>,
) -> Element {
    let canvas_width = 900.0;
    let canvas_height = 420.0;

    let center_x = canvas_width / 2.0;
    let center_y = canvas_height / 2.0;

    // 1. Calculate degrees of nodes to identify a "hub" node for layout
    let mut degrees: HashMap<String, usize> = HashMap::new();
    for edge in &edges {
        *degrees.entry(edge.from.clone()).or_default() += 1;
        *degrees.entry(edge.to.clone()).or_default() += 1;
    }

    // Find the node with highest degree
    let mut hub_id: Option<String> = None;
    let mut max_degree = 0;
    for (id, deg) in &degrees {
        if *deg > max_degree {
            max_degree = *deg;
            hub_id = Some(id.clone());
        }
    }

    // Only use hub layout if the hub has significant connections relative to node count
    let use_hub_layout = nodes.len() >= 4 && max_degree >= 3;

    // 2. Position nodes (hub in center, rest in a circle; or all in a circle if no clear hub)
    let mut node_positions: HashMap<String, (f64, f64)> = HashMap::new();

    let mut outer_nodes = Vec::new();
    for (node, _) in &nodes {
        if node.x > 0.01 || node.y > 0.01 {
            node_positions.insert(node.id.clone(), (node.x, node.y));
            continue;
        }

        if use_hub_layout && Some(node.id.clone()) == hub_id {
            node_positions.insert(node.id.clone(), (center_x, center_y));
        } else {
            outer_nodes.push(node.id.clone());
        }
    }

    let n_outer = outer_nodes.len();
    let radius = 135.0;

    for (i, id) in outer_nodes.iter().enumerate() {
        let angle = (i as f64 * 2.0 * std::f64::consts::PI) / n_outer as f64;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        node_positions.insert(id.clone(), (x, y));
    }

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

        // Clip each end independently against its actual rendered shape.
        let (fx_conn, fy_conn) = shape_for(&edge.from).connection_point((fx, fy), (tx, ty));
        let (tx_conn, ty_conn) = shape_for(&edge.to).connection_point((tx, ty), (fx, fy));

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
            let (width, height) = shape.dimensions();
            NavigationNode {
                x: x - width / 2.0,
                y: y - height / 2.0,
                width,
                height,
                shape,
            }
        })
        .collect();

    rsx! {
        GraphNavigator {
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
}
