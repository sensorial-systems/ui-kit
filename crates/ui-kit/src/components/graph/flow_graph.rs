use crate::components::graph::edge::{Edge, EdgeDefs, EdgeType, GraphEdgeData};
use crate::components::graph::navigation::{GraphNavigator, NavigationNode};
use crate::components::graph::node::{GraphNodeData, Node, NodeShape};
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn FlowGraph(
    nodes: Vec<(GraphNodeData, Element)>,
    edges: Vec<GraphEdgeData>,
    #[props(default)] active_node_id: Option<String>,
    #[props(default)] on_node_click: EventHandler<String>,
) -> Element {
    // Canvas dimensions
    let canvas_width = 900.0;
    let canvas_height = 420.0;

    // Build incoming adjacency list to calculate levels (auto-layout DAG)
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &edges {
        incoming
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    // Determine levels of nodes
    let mut levels: HashMap<String, usize> = HashMap::new();

    // Initialize
    for (node, _) in &nodes {
        levels.insert(node.id.clone(), 0);
    }

    // Relax levels (DAG longest path)
    let max_iterations = nodes.len();
    for _ in 0..max_iterations {
        let mut changed = false;
        for edge in &edges {
            let from_level = *levels.get(&edge.from).unwrap_or(&0);
            let to_level = *levels.get(&edge.to).unwrap_or(&0);
            if to_level <= from_level {
                levels.insert(edge.to.clone(), from_level + 1);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Group nodes by level
    let mut nodes_by_level: HashMap<usize, Vec<String>> = HashMap::new();
    for (node, _) in &nodes {
        let lvl = *levels.get(&node.id).unwrap_or(&0);
        nodes_by_level.entry(lvl).or_default().push(node.id.clone());
    }

    let max_level = levels.values().cloned().max().unwrap_or(0);

    // Calculate final layout positions
    let mut node_positions: HashMap<String, (f64, f64)> = HashMap::new();
    for (node, _) in &nodes {
        // If node has manually specified coordinates (non-zero), use them
        if node.x > 0.01 || node.y > 0.01 {
            node_positions.insert(node.id.clone(), (node.x, node.y));
            continue;
        }

        // Otherwise, place based on calculated column (level) and vertical spacing
        let lvl = *levels.get(&node.id).unwrap_or(&0);

        // Calculate X coordinate
        let x = if max_level == 0 {
            canvas_width / 2.0
        } else {
            let col_gap = (canvas_width - 200.0) / max_level as f64;
            100.0 + (lvl as f64 * col_gap)
        };

        // Calculate Y coordinate
        let nodes_in_lvl = nodes_by_level.get(&lvl).unwrap();
        let idx = nodes_in_lvl
            .iter()
            .position(|id| id == &node.id)
            .unwrap_or(0);
        let count = nodes_in_lvl.len();

        let y = if count <= 1 {
            canvas_height / 2.0
        } else {
            let row_gap = (canvas_height - 120.0) / (count - 1) as f64;
            60.0 + (idx as f64 * row_gap)
        };

        node_positions.insert(node.id.clone(), (x, y));
    }

    // Render edges by looking up start/end coordinates from calculated positions
    let rendered_edges = edges.iter().map(|edge| {
        let (fx, fy) = node_positions
            .get(&edge.from)
            .copied()
            .unwrap_or((0.0, 0.0));
        let (tx, ty) = node_positions.get(&edge.to).copied().unwrap_or((0.0, 0.0));

        // Offset connection points to the borders of the Box nodes (width = 150px)
        let fx_conn = fx + 75.0;
        let tx_conn = tx - 75.0;

        rsx! {
            Edge {
                key: "{edge.from}-{edge.to}",
                from_x: fx_conn,
                from_y: fy,
                to_x: tx_conn,
                to_y: ty,
                edge_type: EdgeType::Orthogonal,
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

        rsx! {
            Node {
                key: "{node.id}",
                id: node.id.clone(),
                x: x,
                y: y,
                color: node.color.clone(),
                border: node.border.clone(),
                background_color: node.background_color.clone(),
                shape: NodeShape::Box,
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
            let (width, height) = NodeShape::Box.dimensions();
            NavigationNode {
                x: x - width / 2.0,
                y: y - height / 2.0,
                width,
                height,
                shape: NodeShape::Box,
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
