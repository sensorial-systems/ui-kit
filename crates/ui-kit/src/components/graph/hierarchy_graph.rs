use crate::components::graph::edge::{ArrowHead, Edge, EdgeDefs, EdgeType, GraphEdgeData};
use crate::components::graph::navigation::{GraphNavigator, NavigationNode};
use crate::components::graph::node::{GraphNodeData, Node, NodeShape};
use crate::components::input::{Button, ButtonSize, EditableText, EditableTextVariant};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyNode {
    pub data: GraphNodeData,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HierarchyGraphModel {
    pub nodes: Vec<HierarchyNode>,
    pub edges: Vec<GraphEdgeData>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NodeDimensions {
    width: f64,
    height: f64,
}

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
    let retained = 0.55_f64.powi(level as i32) * 100.0;
    format!("color-mix(in srgb, {color} {retained:.0}%, white)")
}

fn hierarchy_node_shape(shape: NodeShape, depth: usize, _background: Option<&str>) -> NodeShape {
    if depth == 2 && shape == NodeShape::Plain {
        NodeShape::Box
    } else {
        shape
    }
}

fn hierarchy_depth(node_id: &str, parents: &HashMap<String, String>) -> usize {
    let mut depth = 0;
    let mut current = node_id;
    let mut visited = HashSet::new();
    while let Some(parent) = parents.get(current) {
        if !visited.insert(current.to_string()) {
            break;
        }
        depth += 1;
        current = parent;
    }
    depth
}

fn strip_html(value: &str) -> String {
    let value = value
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</div>", "\n")
        .replace("</p>", "\n")
        .replace("</li>", "\n");
    let mut text = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn label_dimensions(label: &str, shape: NodeShape, depth: usize) -> NodeDimensions {
    let text = strip_html(label);
    let lines = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let line_count = lines.len().max(1);
    let longest_line = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let font_size = if depth == 0 {
        20.0
    } else if depth == 1 {
        14.0
    } else {
        13.0
    };
    let horizontal_padding = if shape == NodeShape::Plain { 8.0 } else { 32.0 };
    let vertical_padding = if shape == NodeShape::Plain { 8.0 } else { 24.0 };
    let measured_width = longest_line as f64 * font_size * 0.58 + horizontal_padding;
    let measured_height = line_count as f64 * font_size * 1.25 + vertical_padding;
    let (base_width, base_height) = match shape {
        NodeShape::Circle => shape.dimensions(),
        NodeShape::Pill => (70.0, 32.0),
        NodeShape::Box if depth == 0 => (96.0, 40.0),
        NodeShape::Box => (44.0, 34.0),
        NodeShape::Underline | NodeShape::Plain => (24.0, 22.0),
    };

    NodeDimensions {
        width: measured_width.max(base_width).ceil(),
        height: measured_height.max(base_height),
    }
}

fn measurement_key(label: &str, shape: NodeShape, depth: usize) -> String {
    format!("{depth}:{shape:?}:{label}")
}

fn shape_connection_point(
    shape: NodeShape,
    dimensions: NodeDimensions,
    center: (f64, f64),
    toward: (f64, f64),
) -> (f64, f64) {
    let dx = toward.0 - center.0;
    let dy = toward.1 - center.1;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return center;
    }

    let half_width = dimensions.width / 2.0;
    let half_height = dimensions.height / 2.0;
    let scale = match shape {
        NodeShape::Circle | NodeShape::Pill => {
            1.0 / ((dx / half_width).powi(2) + (dy / half_height).powi(2)).sqrt()
        }
        _ => (half_width / dx.abs()).min(half_height / dy.abs()),
    };

    (center.0 + dx * scale, center.1 + dy * scale)
}

fn shape_connection_normal(
    shape: NodeShape,
    dimensions: NodeDimensions,
    center: (f64, f64),
    point: (f64, f64),
) -> (f64, f64) {
    let half_width = dimensions.width / 2.0;
    let half_height = dimensions.height / 2.0;
    let dx = point.0 - center.0;
    let dy = point.1 - center.1;
    let (nx, ny) = match shape {
        NodeShape::Circle | NodeShape::Pill => {
            (dx / half_width.powi(2), dy / half_height.powi(2))
        }
        _ => {
            let x_ratio = dx.abs() / half_width;
            let y_ratio = dy.abs() / half_height;
            if (x_ratio - y_ratio).abs() < 1e-9 {
                (dx / half_width, dy / half_height)
            } else if x_ratio > y_ratio {
                (dx.signum(), 0.0)
            } else {
                (0.0, dy.signum())
            }
        }
    };
    let length = (nx * nx + ny * ny).sqrt();
    if length < f64::EPSILON {
        (0.0, 0.0)
    } else {
        (nx / length, ny / length)
    }
}

fn descendant_ids(node_id: &str, children: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut descendants = HashSet::new();
    let mut pending = children.get(node_id).cloned().unwrap_or_default();
    while let Some(id) = pending.pop() {
        if descendants.insert(id.clone()) {
            pending.extend(children.get(&id).cloned().unwrap_or_default());
        }
    }
    descendants
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

fn rounded_t_paths(from: (f64, f64), mut targets: Vec<(f64, f64)>, side: f64) -> Vec<String> {
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
pub fn HierarchyGraphViewer(
    graph: HierarchyGraphModel,
    /// Optional node bodies keyed by id. The editor uses this slot to render
    /// inline editors while retaining the viewer's exact canvas and layout.
    #[props(default)]
    node_elements: Vec<(String, Element)>,
    #[props(default)] active_node_id: Option<String>,
    #[props(default)] on_node_click: EventHandler<String>,
    /// Complete CSS border declaration for the graph canvas. `"none"` removes it.
    #[props(into, default)]
    border: Option<String>,
    /// CSS background color for the graph canvas.
    #[props(into, default)]
    background_color: Option<String>,
    /// Visible width used to anchor edges to the transparent core topic.
    #[props(default = 100.0)]
    core_width: f64,
    /// Visible height used to anchor edges to the transparent core topic.
    #[props(default = 28.0)]
    core_height: f64,
    /// Optional routing override for every core-to-main-topic edge. When
    /// omitted, each `GraphEdgeData::edge_type` is used independently.
    #[props(default)]
    root_edge_type: Option<EdgeType>,
) -> Element {
    let mut collapsed_node_ids = use_signal(Vec::<String>::new);
    let mut hovered_node_id = use_signal(|| None::<String>);
    let mut measured_dimensions =
        use_signal(HashMap::<String, (String, NodeDimensions)>::new);
    let raw_edges = graph.edges.clone();
    let node_elements: HashMap<String, Element> = node_elements.into_iter().collect();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut parents: HashMap<String, String> = HashMap::new();
    for edge in &raw_edges {
        children
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        parents.insert(edge.to.clone(), edge.from.clone());
    }
    let collapsed: HashSet<String> = collapsed_node_ids().into_iter().collect();
    let hidden_nodes = collapsed
        .iter()
        .flat_map(|node_id| descendant_ids(node_id, &children))
        .collect::<HashSet<_>>();
    let edges: Vec<GraphEdgeData> = raw_edges
        .iter()
        .filter(|edge| !hidden_nodes.contains(&edge.from) && !hidden_nodes.contains(&edge.to))
        .cloned()
        .collect();
    let nodes: Vec<(GraphNodeData, Element, String)> = graph
        .nodes
        .iter()
        .filter(|node| !hidden_nodes.contains(&node.data.id))
        .map(|node| {
            let element = node_elements
                .get(&node.data.id)
                .cloned()
                .unwrap_or_else(|| {
                    let label = node.label.clone();
                    rsx! {
                        div {
                            class: "uikit-hierarchy-graph-label-value",
                            dangerous_inner_html: "{label}"
                        }
                    }
                });
            (node.data.clone(), element, node.label.clone())
        })
        .collect();

    let mut roots: Vec<String> = nodes
        .iter()
        .filter(|(node, _, _)| !parents.contains_key(&node.id))
        .map(|(node, _, _)| node.id.clone())
        .collect();
    if roots.is_empty() && !nodes.is_empty() {
        roots.push(nodes[0].0.id.clone());
    }

    // Canvas geometry is based on the complete graph. Collapsing only changes
    // visibility; it must not move the root by changing canvas dimensions.
    let node_depths: HashMap<String, usize> = graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.data.id.clone(),
                hierarchy_depth(&node.data.id, &parents),
            )
        })
        .collect();
    let branch_colors: HashMap<String, String> = nodes
        .iter()
        .filter(|(node, _, _)| node_depths.get(&node.id).copied() == Some(1))
        .filter_map(|(node, _, _)| {
            let color = node.color.clone().or_else(|| {
                edges
                    .iter()
                    .find(|edge| edge.to == node.id)
                    .and_then(|edge| edge.color.clone())
            });
            color.map(|color| (node.id.clone(), color))
        })
        .collect();
    let node_dimensions: HashMap<String, NodeDimensions> = graph
        .nodes
        .iter()
        .map(|node| {
            let depth = node_depths.get(&node.data.id).copied().unwrap_or(0);
            let shape = hierarchy_node_shape(
                node.data.shape,
                depth,
                node.data.background_color.as_deref(),
            );
            let key = measurement_key(&node.label, shape, depth);
            let dimensions = measured_dimensions
                .read()
                .get(&node.data.id)
                .filter(|(measured_key, _)| measured_key == &key)
                .map(|(_, dimensions)| *dimensions)
                .unwrap_or_else(|| label_dimensions(&node.label, shape, depth));
            (node.data.id.clone(), dimensions)
        })
        .collect();
    let row_gap = node_dimensions
        .values()
        .map(|dimensions| dimensions.height)
        .fold(0.0_f64, f64::max)
        .max(34.0)
        + 28.0;

    // Keep the root's main topics evenly distributed by count. Subtree leaf
    // weights are still used below to allocate enough vertical space per side.
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
        if right.len() <= left.len() {
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
        if right.len() <= left.len() {
            right.push(root.clone());
            right_weight += weight;
        } else {
            left.push(root.clone());
            left_weight += weight;
        }
    }

    let largest_side = left_weight.max(right_weight).max(1) as f64;
    let canvas_height = (largest_side * row_gap + 100.0).max(420.0);
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

    let max_depth = node_depths
        .values()
        .copied()
        .max()
        .unwrap_or(1);
    let mut depth_widths = vec![0.0_f64; max_depth + 1];
    for (node_id, depth) in &node_depths {
        if let Some(dimensions) = node_dimensions.get(node_id) {
            depth_widths[*depth] = depth_widths[*depth].max(dimensions.width);
        }
    }
    let mut depth_offsets = vec![0.0_f64; max_depth + 1];
    for depth in 1..=max_depth {
        depth_offsets[depth] = depth_offsets[depth - 1]
            + depth_widths[depth - 1] / 2.0
            + depth_widths[depth] / 2.0
            + 72.0;
    }
    let outer_width = depth_widths.last().copied().unwrap_or(0.0);
    let canvas_width = (depth_offsets[max_depth] * 2.0 + outer_width + 120.0).max(900.0);
    let center_x = canvas_width / 2.0;

    let mut node_positions: HashMap<String, (f64, f64)> = HashMap::new();
    for (node, _, _) in &nodes {
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
            (center_x + side * depth_offsets[depth], y),
        );
    }

    let node_data: HashMap<&str, &GraphNodeData> = nodes
        .iter()
        .map(|(node, _, _)| (node.id.as_str(), node))
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
            let to = node_positions.get(&edge.to).copied().unwrap_or((0.0, 0.0));
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
                .map(|node| {
                    let depth = node_depths.get(&node.id).copied().unwrap_or(0);
                    let background = if depth == 2 {
                        Some("")
                    } else {
                        node.background_color.as_deref()
                    };
                    let shape = hierarchy_node_shape(node.shape, depth, background);
                    let dimensions = node_dimensions
                        .get(&node.id)
                        .copied()
                        .unwrap_or_else(|| {
                            let (width, height) = shape.dimensions();
                            NodeDimensions { width, height }
                        });
                    shape_connection_point(shape, dimensions, to, target_toward)
                })
                .unwrap_or(to);
            let root_side = (to.0 - from.0).signum();
            let from_normal = if root_side.abs() < f64::EPSILON {
                rectangular_normal(from, from_point, core_width, core_height)
            } else {
                (root_side, 0.0)
            };
            let to_normal = node_data
                .get(edge.to.as_str())
                .map(|node| {
                    let depth = node_depths.get(&node.id).copied().unwrap_or(0);
                    let background = if depth == 2 {
                        Some("")
                    } else {
                        node.background_color.as_deref()
                    };
                    let shape = hierarchy_node_shape(node.shape, depth, background);
                    let dimensions = node_dimensions
                        .get(&node.id)
                        .copied()
                        .unwrap_or_else(|| {
                            let (width, height) = shape.dimensions();
                            NodeDimensions { width, height }
                        });
                    shape_connection_normal(shape, dimensions, to, to_point)
                })
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
                let background = if parent_depth == 2 {
                    Some("")
                } else {
                    node.background_color.as_deref()
                };
                let shape = hierarchy_node_shape(node.shape, parent_depth, background);
                let dimensions = node_dimensions
                    .get(&node.id)
                    .copied()
                    .unwrap_or_else(|| {
                        let (width, height) = shape.dimensions();
                        NodeDimensions { width, height }
                    });
                shape_connection_point(
                    shape,
                    dimensions,
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
                            let depth = node_depths.get(&node.id).copied().unwrap_or(0);
                            let background = if depth == 2 {
                                Some("")
                            } else {
                                node.background_color.as_deref()
                            };
                            let shape = hierarchy_node_shape(node.shape, depth, background);
                            let dimensions = node_dimensions
                                .get(&node.id)
                                .copied()
                                .unwrap_or_else(|| {
                                    let (width, height) = shape.dimensions();
                                    NodeDimensions { width, height }
                                });
                            shape_connection_point(
                                shape,
                                dimensions,
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
            .or_else(|| {
                node_data
                    .get(parent_id.as_str())
                    .and_then(|node| node.color.clone())
            })
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

    let rendered_nodes = nodes.iter().map(|(node, node_element, label)| {
        let (x, y) = node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
        let is_selected = active_node_id.as_ref() == Some(&node.id) || node.selected;
        let click_node_id = node.id.clone();
        let enter_node_id = node.id.clone();
        let leave_node_id = node.id.clone();
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
            (
                node.color.clone(),
                Some("none".to_string()),
                Some("transparent".to_string()),
            )
        } else if depth == 1 {
            (
                base_color.clone(),
                node.border.clone().or_else(|| Some("none".to_string())),
                node.background_color.clone().or_else(|| base_color.clone()),
            )
        } else if depth == 2 {
            let accent = if node.shape == crate::components::graph::node::NodeShape::Plain {
                base_color.clone()
            } else {
                level_color.clone()
            };
            (
                accent,
                node.border.clone().or_else(|| Some("none".to_string())),
                node.background_color
                    .clone()
                    .filter(|background| background != "transparent")
                    .or(level_color),
            )
        } else {
            (
                base_color.clone(),
                node.border.clone().or_else(|| Some("none".to_string())),
                node.background_color
                    .clone()
                    .or_else(|| Some("transparent".to_string())),
            )
        };
        let effective_shape = hierarchy_node_shape(node.shape, depth, node_background.as_deref());
        let node_measurement_key = measurement_key(label, effective_shape, depth);
        let measure_node_id = node.id.clone();
        let mount_measurement_key = node_measurement_key.clone();
        let resize_node_id = node.id.clone();
        let resize_measurement_key = node_measurement_key.clone();

        rsx! {
            Node {
                key: "{node.id}-{node_measurement_key}",
                id: node.id.clone(),
                x,
                y,
                color: accent,
                border: node_border,
                background_color: node_background,
                shape: effective_shape,
                selected: is_selected,
                onclick: move |_| on_node_click.call(click_node_id.clone()),
                onmouseenter: move |_| hovered_node_id.set(Some(enter_node_id.clone())),
                onmouseleave: move |_| {
                    if hovered_node_id.peek().as_ref() == Some(&leave_node_id) {
                        hovered_node_id.set(None);
                    }
                },
                onmounted: move |event: MountedEvent| {
                    let data = event.data().clone();
                    let node_id = measure_node_id.clone();
                    let key = mount_measurement_key.clone();
                    spawn(async move {
                        if let Ok(size) = data.get_scroll_size().await {
                            let dimensions = NodeDimensions {
                                width: size.width,
                                height: size.height,
                            };
                            measured_dimensions.with_mut(|items| {
                                items.insert(node_id, (key, dimensions));
                            });
                        }
                    });
                },
                onresize: move |(width, height)| {
                    measured_dimensions.with_mut(|items| {
                        items.insert(
                            resize_node_id.clone(),
                            (
                                resize_measurement_key.clone(),
                                NodeDimensions { width, height },
                            ),
                        );
                    });
                },
                div { class: "uikit-hierarchy-graph-node-content",
                    div {
                        class: "uikit-hierarchy-graph-node-label uikit-hierarchy-graph-level-{depth}",
                        {node_element.clone()}
                    }
                }
            }
        }
    });

    let rendered_collapse_buttons = nodes.iter().filter_map(|(node, _, _)| {
        if !children.get(&node.id).is_some_and(|items| !items.is_empty()) {
            return None;
        }
        let (x, y) = node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
        let dimensions = node_dimensions.get(&node.id).copied().unwrap_or(NodeDimensions {
            width: 0.0,
            height: 0.0,
        });
        let side = layout_positions
            .get(&node.id)
            .map(|(_, _, side)| *side)
            .unwrap_or(1.0);
        let direction = if side < 0.0 { -1.0 } else { 1.0 };
        let button_x = x + direction * (dimensions.width / 2.0 + 14.0);
        let is_collapsed = collapsed.contains(&node.id);
        let is_visible = is_collapsed || hovered_node_id.read().as_ref() == Some(&node.id);
        let hidden_child_count = descendant_ids(&node.id, &children).len();
        let button_label = if is_collapsed {
            hidden_child_count.to_string()
        } else {
            "-".to_string()
        };
        let toggle_node_id = node.id.clone();
        let enter_button_id = node.id.clone();
        let leave_button_id = node.id.clone();

        Some(rsx! {
            button {
                key: "collapse-{node.id}",
                class: if is_visible {
                    "uikit-hierarchy-collapse-button is-visible"
                } else {
                    "uikit-hierarchy-collapse-button"
                },
                style: "left: {button_x}px; top: {y}px;",
                r#type: "button",
                aria_label: "Toggle child topics",
                onmouseenter: move |_| hovered_node_id.set(Some(enter_button_id.clone())),
                onmouseleave: move |_| {
                    if hovered_node_id.peek().as_ref() == Some(&leave_button_id) {
                        hovered_node_id.set(None);
                    }
                },
                onclick: move |event| {
                    event.stop_propagation();
                    collapsed_node_ids.with_mut(|items| {
                        if let Some(index) = items.iter().position(|item| item == &toggle_node_id) {
                            items.remove(index);
                        } else {
                            items.push(toggle_node_id.clone());
                        }
                    });
                },
                "{button_label}"
            }
        })
    });

    let navigation_nodes = nodes
        .iter()
        .map(|(node, _, _)| {
            let (x, y) = node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
            let depth = node_depths.get(&node.id).copied().unwrap_or(0);
            let shape = hierarchy_node_shape(node.shape, depth, Some(""));
            let dimensions = node_dimensions
                .get(&node.id)
                .copied()
                .unwrap_or_else(|| {
                    let (width, height) = shape.dimensions();
                    NodeDimensions { width, height }
                });
            NavigationNode {
                x: x - dimensions.width / 2.0,
                y: y - dimensions.height / 2.0,
                width: dimensions.width,
                height: dimensions.height,
                shape,
            }
        })
        .collect();

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
        GraphNavigator {
            canvas_width,
            canvas_height,
            nodes: navigation_nodes,
            center_on_mount: true,
            canvas_class: "uikit-graph-container uikit-hierarchy-graph",
            canvas_style,
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
                {rendered_collapse_buttons}
            }
        }
    }
}

fn next_node_id(graph: &HierarchyGraphModel) -> String {
    let ids: HashSet<&str> = graph
        .nodes
        .iter()
        .map(|node| node.data.id.as_str())
        .collect();
    (1..)
        .map(|index| format!("node-{index}"))
        .find(|id| !ids.contains(id.as_str()))
        .expect("an available hierarchy node id")
}

fn add_child(graph: &HierarchyGraphModel, parent_id: &str) -> (HierarchyGraphModel, String) {
    let mut updated = graph.clone();
    let id = next_node_id(graph);
    let parents = graph
        .edges
        .iter()
        .map(|edge| (edge.to.clone(), edge.from.clone()))
        .collect::<HashMap<_, _>>();
    let child_depth = hierarchy_depth(parent_id, &parents) + 1;
    let is_root_child = child_depth == 1;
    let root_child_count = graph
        .edges
        .iter()
        .filter(|edge| edge.from == parent_id)
        .count();
    let branch_color = is_root_child.then(|| {
        let hue = ((root_child_count + 1) as f64 * 137.508 + 280.0) % 360.0;
        format!("hsl({hue:.0}deg 68% 52%)")
    });

    updated.nodes.push(HierarchyNode {
        data: GraphNodeData {
            id: id.clone(),
            x: 0.0,
            y: 0.0,
            color: branch_color.clone(),
            border: Some("none".to_string()),
            background_color: None,
            shape: if child_depth <= 2 {
                NodeShape::Box
            } else {
                NodeShape::Plain
            },
            selected: false,
        },
        label: "New node".to_string(),
    });
    updated.edges.push(GraphEdgeData {
        from: parent_id.to_string(),
        to: id.clone(),
        label: None,
        edge_type: EdgeType::OrganicCurved,
        color: branch_color,
        animated: false,
        arrow: ArrowHead::None,
    });
    (updated, id)
}

fn delete_subtree(graph: &HierarchyGraphModel, node_id: &str) -> HierarchyGraphModel {
    let mut deleted = HashSet::new();
    let mut pending = vec![node_id.to_string()];
    while let Some(parent_id) = pending.pop() {
        if deleted.insert(parent_id.clone()) {
            pending.extend(
                graph
                    .edges
                    .iter()
                    .filter(|edge| edge.from == parent_id)
                    .map(|edge| edge.to.clone()),
            );
        }
    }

    HierarchyGraphModel {
        nodes: graph
            .nodes
            .iter()
            .filter(|node| !deleted.contains(&node.data.id))
            .cloned()
            .collect(),
        edges: graph
            .edges
            .iter()
            .filter(|edge| !deleted.contains(&edge.from) && !deleted.contains(&edge.to))
            .cloned()
            .collect(),
    }
}

#[component]
pub fn HierarchyGraphEditor(
    graph: HierarchyGraphModel,
    onchange: EventHandler<HierarchyGraphModel>,
    #[props(into, default)] border: Option<String>,
    #[props(into, default)] background_color: Option<String>,
    #[props(default = 100.0)] core_width: f64,
    #[props(default = 28.0)] core_height: f64,
    #[props(default)] root_edge_type: Option<EdgeType>,
) -> Element {
    let initial_selection = graph.nodes.first().map(|node| node.data.id.clone());
    let mut selected_node_id = use_signal(|| initial_selection);

    let node_elements = graph
        .nodes
        .iter()
        .map(|node| {
            let node_id = node.data.id.clone();
            let select_node_id = node_id.clone();
            let label = node.label.clone();
            let edit_graph = graph.clone();
            let element = rsx! {
                div {
                    class: "uikit-hierarchy-graph-editor-node",
                    onclick: move |event| {
                        event.stop_propagation();
                        selected_node_id.set(Some(select_node_id.clone()));
                    },
                    EditableText {
                        value: label,
                        variant: EditableTextVariant::Inline,
                        placeholder: "Node label",
                        onchange: move |label| {
                            let mut updated = edit_graph.clone();
                            if let Some(node) = updated
                                .nodes
                                .iter_mut()
                                .find(|node| node.data.id == node_id)
                            {
                                node.label = label;
                                onchange.call(updated);
                            }
                        }
                    }
                }
            };
            (node.data.id.clone(), element)
        })
        .collect();

    let selected = selected_node_id.read().clone();
    rsx! {
        div {
            class: "uikit-hierarchy-graph-editor",
            tabindex: "0",
            onkeydown: {
                let key_graph = graph.clone();
                move |event: KeyboardEvent| {
                    if event.key() == Key::Delete {
                        event.prevent_default();
                        event.stop_propagation();
                        let node_id = selected_node_id.read().clone();
                        if let Some(node_id) = node_id {
                            onchange.call(delete_subtree(&key_graph, &node_id));
                            selected_node_id.set(None);
                        }
                    }
                }
            },
            div {
                class: "uikit-hierarchy-graph-context-menu",
                role: "toolbar",
                aria_label: "Selected node actions",
                Button {
                    size: ButtonSize::Small,
                    disabled: selected.is_none(),
                    onclick: {
                        let selected = selected.clone();
                        let add_graph = graph.clone();
                        move |_| {
                            if let Some(parent_id) = selected.as_deref() {
                                let (updated, new_node_id) = add_child(&add_graph, parent_id);
                                selected_node_id.set(Some(new_node_id));
                                onchange.call(updated);
                            }
                        }
                    },
                    "Add"
                }
                Button {
                    size: ButtonSize::Small,
                    color: "var(--uikit-error)",
                    disabled: selected.is_none(),
                    onclick: {
                        let selected = selected.clone();
                        let delete_graph = graph.clone();
                        move |_| {
                            if let Some(node_id) = selected.as_deref() {
                                onchange.call(delete_subtree(&delete_graph, node_id));
                                selected_node_id.set(None);
                            }
                        }
                    },
                    "Delete"
                }
            }
            HierarchyGraphViewer {
                graph: graph.clone(),
                node_elements,
                active_node_id: selected,
                on_node_click: move |id| selected_node_id.set(Some(id)),
                border,
                background_color,
                core_width,
                core_height,
                root_edge_type
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        add_child, brighter, delete_subtree, layout_branch, leaf_count, rectangular_connection,
        rectangular_normal, rounded_t_paths, measurement_key, ArrowHead, EdgeType, GraphEdgeData,
        GraphNodeData, HierarchyGraphModel, HierarchyNode, NodeShape,
    };
    use std::collections::{HashMap, HashSet};

    fn model_node(id: &str) -> HierarchyNode {
        HierarchyNode {
            data: GraphNodeData {
                id: id.to_string(),
                x: 0.0,
                y: 0.0,
                color: None,
                border: None,
                background_color: None,
                shape: NodeShape::Plain,
                selected: false,
            },
            label: id.to_string(),
        }
    }

    fn model_edge(from: &str, to: &str) -> GraphEdgeData {
        GraphEdgeData {
            from: from.to_string(),
            to: to.to_string(),
            label: None,
            edge_type: EdgeType::Bezier,
            color: None,
            animated: false,
            arrow: ArrowHead::None,
        }
    }

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
        layout_branch("branch", 1, -1.0, &mut y, 20.0, &children, &mut positions);
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
        assert!(brighter("#8844cc", 2).contains("30%"));
    }

    #[test]
    fn measurement_cache_distinguishes_depth_and_shape() {
        assert_ne!(
            measurement_key("Database", NodeShape::Box, 1),
            measurement_key("Database", NodeShape::Box, 2)
        );
        assert_ne!(
            measurement_key("Database", NodeShape::Box, 2),
            measurement_key("Database", NodeShape::Plain, 2)
        );
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

    #[test]
    fn root_children_receive_a_branch_color() {
        let graph = HierarchyGraphModel {
            nodes: vec![model_node("root")],
            edges: Vec::new(),
        };
        let (updated, id) = add_child(&graph, "root");
        let node = updated
            .nodes
            .iter()
            .find(|node| node.data.id == id)
            .unwrap();
        assert!(node.data.color.as_deref().unwrap().starts_with("hsl("));
        assert_eq!(node.data.shape, NodeShape::Box);
        assert_eq!(updated.edges[0].color, node.data.color);
        assert_eq!(updated.edges[0].edge_type, EdgeType::OrganicCurved);
    }

    #[test]
    fn second_level_children_use_crate_owned_box_styling() {
        let graph = HierarchyGraphModel {
            nodes: vec![model_node("root")],
            edges: Vec::new(),
        };
        let (graph, branch_id) = add_child(&graph, "root");
        let (updated, child_id) = add_child(&graph, &branch_id);
        let child = updated
            .nodes
            .iter()
            .find(|node| node.data.id == child_id)
            .unwrap();

        assert_eq!(child.data.shape, NodeShape::Box);
        assert_eq!(child.data.background_color, None);
        assert_eq!(updated.edges.last().unwrap().edge_type, EdgeType::OrganicCurved);
    }

    #[test]
    fn deleting_a_node_removes_its_entire_subtree() {
        let graph = HierarchyGraphModel {
            nodes: vec![
                model_node("root"),
                model_node("branch"),
                model_node("child"),
                model_node("sibling"),
            ],
            edges: vec![
                model_edge("root", "branch"),
                model_edge("branch", "child"),
                model_edge("root", "sibling"),
            ],
        };
        let updated = delete_subtree(&graph, "branch");
        let remaining: HashSet<&str> = updated
            .nodes
            .iter()
            .map(|node| node.data.id.as_str())
            .collect();
        assert_eq!(remaining, HashSet::from(["root", "sibling"]));
        assert_eq!(updated.edges, vec![model_edge("root", "sibling")]);
    }
}
