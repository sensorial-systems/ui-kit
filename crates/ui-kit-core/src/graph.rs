//! Deterministic layout shared by DOM and immediate graph renderers.
use crate::Point;
use std::collections::HashMap;
pub struct NetworkNode {
    pub id: String,
    pub position: Option<Point>,
}
pub struct NetworkLayout {
    pub positions: HashMap<String, Point>,
    pub hub_id: Option<String>,
}
/// Highest-degree node occupies the center when degree >= 3. Ties follow input
/// order, so a gallery does not randomly rearrange between renders or backends.
pub fn network_layout(
    nodes: &[NetworkNode],
    edges: &[(String, String)],
    center: Point,
    radius: f32,
) -> NetworkLayout {
    let mut degrees = HashMap::<&str, usize>::new();
    for (a, b) in edges {
        *degrees.entry(a).or_default() += 1;
        *degrees.entry(b).or_default() += 1;
    }
    let mut hub = None;
    let mut maximum = 0;
    for node in nodes {
        let degree = degrees.get(node.id.as_str()).copied().unwrap_or(0);
        if degree > maximum {
            maximum = degree;
            hub = Some(node.id.clone());
        }
    }
    if nodes.len() < 4 || maximum < 3 {
        hub = None;
    }
    let outer: Vec<_> = nodes
        .iter()
        .filter(|n| n.position.is_none() && hub.as_ref() != Some(&n.id))
        .collect();
    let mut positions = HashMap::new();
    for n in nodes {
        if let Some(point) = n.position {
            positions.insert(n.id.clone(), point);
        } else if hub.as_ref() == Some(&n.id) {
            positions.insert(n.id.clone(), center);
        }
    }
    for (i, n) in outer.iter().enumerate() {
        let angle = i as f32 * std::f32::consts::TAU / outer.len() as f32;
        positions.insert(
            n.id.clone(),
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            ),
        );
    }
    NetworkLayout {
        positions,
        hub_id: hub,
    }
}
/// Endpoints on the circumference, shared by edge painting and hit tests.
pub fn circle_connection(from: Point, to: Point, radius: f32) -> Point {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = dx.hypot(dy);
    if len < 0.0001 {
        return from;
    }
    Point::new(from.x + dx / len * radius, from.y + dy / len * radius)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hub_ties_are_stable_and_explicit_positions_survive() {
        let mut nodes: Vec<_> = ["gateway", "auth", "billing", "db", "cache"]
            .iter()
            .map(|id| NetworkNode {
                id: (*id).into(),
                position: None,
            })
            .collect();
        let edges: Vec<_> = [
            ("gateway", "auth"),
            ("gateway", "billing"),
            ("auth", "db"),
            ("billing", "db"),
            ("db", "cache"),
            ("gateway", "cache"),
        ]
        .iter()
        .map(|(a, b)| ((*a).into(), (*b).into()))
        .collect();
        let first = network_layout(&nodes, &edges, Point::new(450.0, 210.0), 135.0);
        assert_eq!(first.hub_id.as_deref(), Some("gateway"));
        assert_eq!(first.positions["gateway"], Point::new(450.0, 210.0));
        nodes[1].position = Some(Point::new(9.0, 12.0));
        assert_eq!(
            network_layout(&nodes, &edges, Point::default(), 135.0).positions["auth"],
            Point::new(9.0, 12.0)
        );
    }
    #[test]
    fn clipped_circle_endpoint_is_on_its_boundary() {
        let p = circle_connection(Point::default(), Point::new(3.0, 4.0), 10.0);
        assert_eq!(p, Point::new(6.0, 8.0));
    }
}
