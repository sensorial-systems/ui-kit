use crate::components::graph::edge::{Edge, EdgeDefs, EdgeType, GraphEdgeData};
use crate::components::graph::node::{GraphNodeData, Node};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

fn leaf_count(
    node_id: &str,
    children: &HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
) -> usize {
    if !visiting.insert(node_id.to_string()) {
        return 1;
    }

    let count = children
        .get(node_id)
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .map(|child| leaf_count(child, children, visiting))
                .sum()
        })
        .unwrap_or(1);
    visiting.remove(node_id);
    count
}

fn layout_branch(
    node_id: &str,
    depth: usize,
    side: f64,
    current_y: &mut f64,
    row_gap: f64,
    children: &HashMap<String, Vec<String>>,
    positions: &mut HashMap<String, (usize, f64, f64)>,
) {
    if positions.contains_key(node_id) {
        return;
    }

    let child_list = children.get(node_id).filter(|items| !items.is_empty());
    if let Some(child_list) = child_list {
        let mut child_ys = Vec::with_capacity(child_list.len());
        for child in child_list {
            layout_branch(
                child,
                depth + 1,
                side,
                current_y,
                row_gap,
                children,
                positions,
            );
            if let Some((_, y, _)) = positions.get(child) {
                child_ys.push(*y);
            }
        }
        let y = if child_ys.is_empty() {
            let y = *current_y;
            *current_y += row_gap;
            y
        } else {
            child_ys.iter().sum::<f64>() / child_ys.len() as f64
        };
        positions.insert(node_id.to_string(), (depth, y, side));
    } else {
        positions.insert(node_id.to_string(), (depth, *current_y, side));
        *current_y += row_gap;
    }
}

fn brighter(color: &str, level: usize) -> String {
    if level == 0 {
        return color.to_string();
    }
    let retained = 0.7_f64.powi(level as i32) * 100.0;
    format!("color-mix(in srgb, {color} {retained:.0}%, white)")
}

fn branch_ancestor(
    node_id: &str,
    parents: &HashMap<String, String>,
    depths: &HashMap<String, usize>,
) -> String {
    let mut current = node_id;
    while depths.get(current).copied().unwrap_or(0) > 1 {
        let Some(parent) = parents.get(current) else {
            break;
        };
        current = parent;
    }
    current.to_string()
}

fn rounded_t_paths(
    from: (f64, f64),
    mut targets: Vec<(f64, f64)>,
    side: f64,
) -> Vec<String> {
    if targets.is_empty() {
        return Vec::new();
    }
    targets.sort_by(|a, b| a.1.total_cmp(&b.1));
    if targets.len() == 1 {
        return vec![format!(
            "M {},{} L {},{}",
            from.0, from.1, targets[0].0, targets[0].1
        )];
    }

    let nearest_target_x = targets[0].0;
    let available = (nearest_target_x - from.0).abs();
    let trunk_x = from.0 + side * (available * 0.5).clamp(12.0, 32.0);
    let first_y = targets.first().map(|point| point.1).unwrap_or(from.1);
    let last_y = targets.last().map(|point| point.1).unwrap_or(from.1);
    let radius = 9.0_f64.min((last_y - first_y).abs() * 0.25);

    let mut paths = vec![
        format!("M {},{} L {},{}", from.0, from.1, trunk_x, from.1),
        format!(
            "M {},{} L {},{}",
            trunk_x,
            first_y + radius,
            trunk_x,
            last_y - radius
        ),
    ];
    for target in targets {
        let path = if target.1 < from.1 - 0.01 {
            format!(
                "M {},{} Q {},{} {},{} L {},{}",
                trunk_x,
                target.1 + radius,
                trunk_x,
                target.1,
                trunk_x + side * radius,
                target.1,
                target.0,
                target.1
            )
        } else if target.1 > from.1 + 0.01 {
            format!(
                "M {},{} Q {},{} {},{} L {},{}",
                trunk_x,
                target.1 - radius,
                trunk_x,
                target.1,
                trunk_x + side * radius,
                target.1,
                target.0,
                target.1
            )
        } else {
            format!("M {},{} L {},{}", trunk_x, target.1, target.0, target.1)
        };
        paths.push(path);
    }
    paths
}

fn rectangular_connection(
    center: (f64, f64),
    toward: (f64, f64),
    width: f64,
    height: f64,
) -> (f64, f64) {
    let dx = toward.0 - center.0;
    let dy = toward.1 - center.1;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return center;
    }
    let scale = ((width / 2.0) / dx.abs()).min((height / 2.0) / dy.abs());
    (center.0 + dx * scale, center.1 + dy * scale)
}

fn rectangular_normal(
    center: (f64, f64),
    point: (f64, f64),
    width: f64,
    height: f64,
) -> (f64, f64) {
    let dx = point.0 - center.0;
    let dy = point.1 - center.1;
    let x_ratio = dx.abs() / (width / 2.0);
    let y_ratio = dy.abs() / (height / 2.0);
    if (x_ratio - y_ratio).abs() < 1e-9 {
        let length = (dx * dx + dy * dy).sqrt();
        if length < f64::EPSILON {
            (0.0, 0.0)
        } else {
            (dx / length, dy / length)
        }
    } else if x_ratio > y_ratio {
        (dx.signum(), 0.0)
    } else {
        (0.0, dy.signum())
    }
}

#[component]
pub fn HierarchyGraph(
    nodes: Vec<(GraphNodeData, Element)>,
    edges: Vec<GraphEdgeData>,
    #[props(default)] active_node_id: Option<String>,
    #[props(default)] on_node_click: EventHandler<String>,
    /// Complete CSS border declaration for the graph canvas. `"none"` removes it.
    #[props(into, default)] border: Option<String>,
    /// CSS background color for the graph canvas.
    #[props(into, default)] background_color: Option<String>,
    /// Visible width used to anchor edges to the transparent core topic.
    #[props(default = 100.0)] core_width: f64,
    /// Visible height used to anchor edges to the transparent core topic.
    #[props(default = 28.0)] core_height: f64,
    /// Optional routing override for every core-to-main-topic edge. When
    /// omitted, each `GraphEdgeData::edge_type` is used independently.
    #[props(default)] root_edge_type: Option<EdgeType>,
) -> Element {
    let row_gap = 58.0;

    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut parents: HashMap<String, String> = HashMap::new();
    for edge in &edges {
        children
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        parents.insert(edge.to.clone(), edge.from.clone());
    }

    let mut roots: Vec<String> = nodes
        .iter()
        .filter(|(node, _)| !parents.contains_key(&node.id))
        .map(|(node, _)| node.id.clone())
        .collect();
    if roots.is_empty() && !nodes.is_empty() {
        roots.push(nodes[0].0.id.clone());
    }

    // XMind balances the root's main topics on both sides of the central
    // topic. Weighting by leaves keeps a large subtree from crowding one side.
    let primary_root = roots.first().cloned();
    let root_children = primary_root
        .as_ref()
        .and_then(|root| children.get(root))
        .cloned()
        .unwrap_or_default();
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut left_weight = 0;
    let mut right_weight = 0;
    for child in root_children {
        let weight = leaf_count(&child, &children, &mut HashSet::new());
        if right_weight <= left_weight {
            right.push(child);
            right_weight += weight;
        } else {
            left.push(child);
            left_weight += weight;
        }
    }

    // Treat additional disconnected roots as main topics so they remain
    // visible and participate in the same balanced mind-map layout.
    for root in roots.iter().skip(1) {
        let weight = leaf_count(root, &children, &mut HashSet::new());
        if right_weight <= left_weight {
            right.push(root.clone());
            right_weight += weight;
        } else {
            left.push(root.clone());
            left_weight += weight;
        }
    }

    let largest_side = left_weight.max(right_weight).max(1) as f64;
    let canvas_height = (largest_side * row_gap + 100.0).max(420.0);
    let horizontal_gap = 220.0;
    let center_y = canvas_height / 2.0;
    let largest_span = (largest_side - 1.0).max(0.0) * row_gap;

    let side_tracks = |weight: usize| {
        if weight <= 1 {
            (center_y, row_gap)
        } else {
            let gap = largest_span / (weight - 1) as f64;
            (center_y - gap * (weight - 1) as f64 / 2.0, gap)
        }
    };

    let mut layout_positions: HashMap<String, (usize, f64, f64)> = HashMap::new();
    if let Some(root) = primary_root.as_ref() {
        layout_positions.insert(root.clone(), (0, center_y, 0.0));
    }
    let (mut left_y, left_gap) = side_tracks(left_weight);
    for child in &left {
        layout_branch(
            child,
            1,
            -1.0,
            &mut left_y,
            left_gap,
            &children,
            &mut layout_positions,
        );
    }
    let (mut right_y, right_gap) = side_tracks(right_weight);
    for child in &right {
        layout_branch(
            child,
            1,
            1.0,
            &mut right_y,
            right_gap,
            &children,
            &mut layout_positions,
        );
    }

    let max_depth = layout_positions
        .values()
        .map(|(depth, _, _)| *depth)
        .max()
        .unwrap_or(1);
    let canvas_width = (max_depth as f64 * horizontal_gap * 2.0 + 180.0).max(900.0);
    let center_x = canvas_width / 2.0;

    let mut node_positions: HashMap<String, (f64, f64)> = HashMap::new();
    for (node, _) in &nodes {
        if node.x.abs() > 0.01 || node.y.abs() > 0.01 {
            node_positions.insert(node.id.clone(), (node.x, node.y));
            continue;
        }
        let (depth, y, side) = layout_positions
            .get(&node.id)
            .copied()
            .unwrap_or((0, center_y, 0.0));
        node_positions.insert(
            node.id.clone(),
            (center_x + side * horizontal_gap * depth as f64, y),
        );
    }

    let node_data: HashMap<&str, &GraphNodeData> = nodes
        .iter()
        .map(|(node, _)| (node.id.as_str(), node))
        .collect();
    let node_depths: HashMap<String, usize> = nodes
        .iter()
        .map(|(node, _)| {
            (
                node.id.clone(),
                layout_positions
                    .get(&node.id)
                    .map(|(depth, _, _)| *depth)
                    .unwrap_or(0),
            )
        })
        .collect();
    let branch_colors: HashMap<String, String> = nodes
        .iter()
        .filter(|(node, _)| node_depths.get(&node.id).copied() == Some(1))
        .filter_map(|(node, _)| {
            let color = node.color.clone().or_else(|| {
                edges
                    .iter()
                    .find(|edge| edge.to == node.id)
                    .and_then(|edge| edge.color.clone())
            });
            color.map(|color| (node.id.clone(), color))
        })
        .collect();

    // The root owns independent organic curves, one for each main topic.
    let root_edges = edges
        .iter()
        .filter(|edge| node_depths.get(&edge.from).copied().unwrap_or(0) == 0)
        .map(|edge| {
        let from = node_positions
            .get(&edge.from)
            .copied()
            .unwrap_or((0.0, 0.0));
        let to = node_positions
            .get(&edge.to)
            .copied()
            .unwrap_or((0.0, 0.0));
        let effective_edge_type = root_edge_type.unwrap_or(edge.edge_type);
        // The transparent core has no visible box to clip against, so compute
        // the exact radial intersection with its configured visible bounds.
        let from_point = rectangular_connection(from, to, core_width, core_height);
        // XMind main-topic branches enter the subject through the center of
        // its facing side. This is independent of variable text-driven node
        // height and guarantees a surface-orthogonal horizontal arrival.
        let target_toward = if (from.0 - to.0).abs() < f64::EPSILON {
            from
        } else {
            (from.0, to.1)
        };
        let to_point = node_data
            .get(edge.to.as_str())
            .map(|node| node.shape.connection_point(to, target_toward))
            .unwrap_or(to);
        let from_normal = rectangular_normal(from, from_point, core_width, core_height);
        let to_normal = node_data
            .get(edge.to.as_str())
            .map(|node| node.shape.connection_normal(to, to_point))
            .unwrap_or_else(|| {
                let dx = from.0 - to.0;
                let dy = from.1 - to.1;
                let length = (dx * dx + dy * dy).sqrt();
                if length < f64::EPSILON {
                    (0.0, 0.0)
                } else {
                    (dx / length, dy / length)
                }
            });
        let color = branch_colors
            .get(&edge.to)
            .cloned()
            .or_else(|| edge.color.clone());

        rsx! {
            Edge {
                key: "{edge.from}-{edge.to}",
                from_x: from_point.0,
                from_y: from_point.1,
                to_x: to_point.0,
                to_y: to_point.1,
                edge_type: effective_edge_type,
                arrow: edge.arrow,
                animated: edge.animated,
                label: edge.label.clone(),
                color,
                from_normal: Some(from_normal),
                to_normal: Some(to_normal)
            }
        }
    });

    // Every deeper parent renders one shared spine. Child arms join that
    // spine as rounded T connections, matching XMind's bracket-like branches.
    let shared_branches = children.iter().filter_map(|(parent_id, child_ids)| {
        let parent_depth = node_depths.get(parent_id).copied().unwrap_or(0);
        if parent_depth == 0 || child_ids.is_empty() {
            return None;
        }
        let parent_position = node_positions.get(parent_id).copied()?;
        let first_child_position = child_ids
            .iter()
            .find_map(|child| node_positions.get(child).copied())?;
        let side = (first_child_position.0 - parent_position.0).signum();
        if side == 0.0 {
            return None;
        }
        let from = node_data
            .get(parent_id.as_str())
            .map(|node| {
                node.shape.connection_point(
                    parent_position,
                    (first_child_position.0, parent_position.1),
                )
            })
            .unwrap_or(parent_position);
        let targets: Vec<(f64, f64)> = child_ids
            .iter()
            .filter_map(|child| {
                let position = node_positions.get(child).copied()?;
                Some(
                    node_data
                        .get(child.as_str())
                        .map(|node| {
                            node.shape.connection_point(
                                position,
                                (parent_position.0, position.1),
                            )
                        })
                        .unwrap_or(position),
                )
            })
            .collect();
        let branch_id = branch_ancestor(parent_id, &parents, &node_depths);
        let base_color = branch_colors
            .get(&branch_id)
            .cloned()
            .or_else(|| node_data.get(parent_id.as_str()).and_then(|node| node.color.clone()))
            .unwrap_or_else(|| "var(--uikit-border)".to_string());
        let color = brighter(&base_color, parent_depth.saturating_sub(1));
        let paths = rounded_t_paths(from, targets, side);
        let path_elements = paths.into_iter().map(|path| {
            rsx! {
                path {
                    class: "uikit-graph-edge-path",
                    style: "stroke: {color};",
                    d: "{path}"
                }
            }
        });

        Some(rsx! {
            g { key: "branch-{parent_id}", {path_elements} }
        })
    });

    let rendered_nodes = nodes.iter().map(|(node, node_element)| {
        let (x, y) = node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
        let is_selected = active_node_id.as_ref() == Some(&node.id) || node.selected;
        let node_id = node.id.clone();
        let depth = node_depths.get(&node.id).copied().unwrap_or(0);
        let branch_id = branch_ancestor(&node.id, &parents, &node_depths);
        let base_color = branch_colors
            .get(&branch_id)
            .cloned()
            .or_else(|| node.color.clone());
        let level_color = base_color
            .as_ref()
            .map(|color| brighter(color, depth.saturating_sub(1)));

        let (accent, node_border, node_background) = if depth == 0 {
            (node.color.clone(), Some("none".to_string()), Some("transparent".to_string()))
        } else if depth == 1 {
            (
                base_color.clone(),
                node.border.clone().or_else(|| Some("none".to_string())),
                node.background_color.clone().or_else(|| base_color.clone()),
            )
        } else {
            let accent = if node.shape == crate::components::graph::node::NodeShape::Plain {
                base_color.clone()
            } else {
                level_color.clone()
            };
            (
                accent,
                node.border.clone().or_else(|| Some("none".to_string())),
                node.background_color.clone().or(level_color),
            )
        };

        rsx! {
            Node {
                key: "{node.id}",
                id: node.id.clone(),
                x,
                y,
                color: accent,
                border: node_border,
                background_color: node_background,
                shape: node.shape,
                selected: is_selected,
                onclick: move |_| on_node_click.call(node_id.clone()),
                {node_element.clone()}
            }
        }
    });

    let mut canvas_style = format!(
        "position: relative; width: {canvas_width}px; height: {canvas_height}px; flex-shrink: 0;"
    );
    if let Some(value) = border.as_ref() {
        canvas_style.push_str(&format!("border: {};", value));
    }
    if let Some(value) = background_color.as_ref() {
        canvas_style.push_str(&format!("background-color: {};", value));
    }

    rsx! {
        div {
            style: "width: 100%; overflow: auto; display: flex; align-items: center; justify-content: center; padding: 12px;",
            div {
                class: "uikit-graph-container uikit-hierarchy-graph",
                style: "{canvas_style}",
                svg {
                    class: "uikit-graph-svg",
                    view_box: "0 0 {canvas_width} {canvas_height}",
                    EdgeDefs {}
                    {root_edges}
                    {shared_branches}
                }
                div {
                    class: "uikit-graph-nodes-container",
                    {rendered_nodes}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        brighter, layout_branch, leaf_count, rectangular_connection, rectangular_normal,
        rounded_t_paths,
    };
    use std::collections::{HashMap, HashSet};

    #[test]
    fn subtree_weight_counts_leaves() {
        let children = HashMap::from([
            ("root".to_string(), vec!["a".to_string(), "b".to_string()]),
            ("a".to_string(), vec!["a1".to_string(), "a2".to_string()]),
        ]);
        assert_eq!(leaf_count("root", &children, &mut HashSet::new()), 3);
    }

    #[test]
    fn parent_is_centered_between_children() {
        let children = HashMap::from([(
            "branch".to_string(),
            vec!["first".to_string(), "second".to_string()],
        )]);
        let mut positions = HashMap::new();
        let mut y = 10.0;
        layout_branch(
            "branch",
            1,
            -1.0,
            &mut y,
            20.0,
            &children,
            &mut positions,
        );
        assert_eq!(positions["branch"], (1, 20.0, -1.0));
    }

    #[test]
    fn shared_branch_uses_rounded_outer_corners() {
        let paths = rounded_t_paths((100.0, 50.0), vec![(200.0, 20.0), (200.0, 80.0)], 1.0);
        assert_eq!(paths[1], "M 132,29 L 132,71");
        assert!(paths[2].contains("Q 132,20 141,20"));
        assert!(paths[3].contains("Q 132,80 141,80"));
    }

    #[test]
    fn subdivisions_get_progressively_brighter() {
        assert_eq!(brighter("#8844cc", 0), "#8844cc");
        assert!(brighter("#8844cc", 2).contains("49%"));
    }

    #[test]
    fn transparent_core_edges_use_visible_bounds() {
        assert_eq!(
            rectangular_connection((500.0, 200.0), (200.0, 200.0), 110.0, 36.0),
            (445.0, 200.0)
        );
        assert_eq!(
            rectangular_connection((500.0, 200.0), (500.0, 100.0), 110.0, 36.0),
            (500.0, 182.0)
        );
        let diagonal = rectangular_connection((0.0, 0.0), (100.0, 100.0), 100.0, 28.0);
        assert!((diagonal.0 - 14.0).abs() < 1e-9);
        assert!((diagonal.1 - 14.0).abs() < 1e-9);
        let diagonal_normal = rectangular_normal((0.0, 0.0), diagonal, 100.0, 28.0);
        assert_eq!(diagonal_normal, (0.0, 1.0));
    }
}
