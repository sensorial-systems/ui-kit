//! Node-and-edge graphs: a pipeline flow, a hub-and-spoke network and an
//! editable hierarchy, sharing a pannable, zoomable camera.
use super::button::{Button, ButtonVariant};
use super::card::panel;
use super::input::TextInput;
use super::theme::{with_alpha, INFO};
use super::Label;
use crate::icon::Icon;
use crate::vector::{center, fill_triangles, line};
use crate::{Flex, Justify, Size, Strip, Ui};
use std::collections::HashMap;
use ui_kit_core::graph::{circle_connection, network_layout, NetworkNode};
use ui_kit_core::{Color, Point, Rect, TextAlign};

/// Zoom and pan over a graph, with buttons to zoom and fit and a drag to pan.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphCamera {
    pub zoom: f32,
    pub pan: Point,
    drag: Option<(Point, Point)>,
}

impl Default for GraphCamera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Point::default(),
            drag: None,
        }
    }
}

impl GraphCamera {
    /// How wide the zoom buttons are together.
    pub const CONTROLS_WIDTH: f32 = 128.0;

    /// Zoom out, zoom in and fit buttons, right-aligned in `rect`.
    pub fn controls(&mut self, ui: &mut Ui, id: &str, rect: Rect) {
        let buttons = Flex::row(4.0)
            .justify(Justify::End)
            .place(rect, &[Size::Fixed(40.0); 3]);
        for (i, button) in buttons.into_iter().enumerate() {
            let control = match i {
                0 => Button::icon_only(Icon::Minus),
                1 => Button::icon_only(Icon::Plus),
                _ => Button::new("Fit").font_size(13.0).padding(2.0),
            };
            if control
                .variant(ButtonVariant::Secondary)
                .show(ui, &format!("{id}-zoom-{i}"), button)
                .clicked
            {
                match i {
                    0 => self.zoom = (self.zoom / 1.2).max(0.35),
                    1 => self.zoom = (self.zoom * 1.2).min(3.0),
                    _ => {
                        self.zoom = 1.0;
                        self.pan = Point::default();
                    }
                }
            }
        }
    }

    /// Follow a drag already under way, before anything is placed, so the
    /// graph moves with the pointer in the same frame.
    fn follow(&mut self, ui: &Ui) {
        match (self.drag, ui.input().pointer, ui.input().down) {
            (Some((start, pan)), Some(pt), true) => {
                self.pan = Point::new(pan.x + pt.x - start.x, pan.y + pt.y - start.y);
            }
            (_, _, false) => self.drag = None,
            _ => {}
        }
    }

    /// Pan by dragging the empty parts of `area`. Call after drawing the
    /// nodes, so they take presses first.
    fn drag(&mut self, ui: &mut Ui, id: &str, area: Rect) {
        let response = ui.interact(&format!("{id}-pan"), area);
        match (response.active, ui.input().pointer) {
            (true, Some(pt)) => {
                let (start, pan) = *self.drag.get_or_insert((pt, self.pan));
                self.pan = Point::new(pan.x + pt.x - start.x, pan.y + pt.y - start.y);
            }
            _ => self.drag = None,
        }
    }

    /// Where a point of a `width` × `height` layout lands in `area`.
    fn transform(&self, area: Rect, width: f32, height: f32) -> (f32, impl Fn(Point) -> Point) {
        let scale = (area.width / width).min(area.height / height) * self.zoom;
        let middle = center(area);
        let pan = self.pan;
        (scale, move |pt: Point| {
            Point::new(
                middle.x + (pt.x - width * 0.5) * scale + pan.x,
                middle.y + (pt.y - height * 0.5) * scale + pan.y,
            )
        })
    }
}

/// An arrowhead at `to`, pointing away from `from`.
pub fn arrow(ui: &mut Ui, from: Point, to: Point, color: Color) {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let length = dx.hypot(dy);
    if length < 0.001 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    fill_triangles(
        ui,
        &[
            to,
            Point::new(to.x - ux * 8.0 - uy * 4.0, to.y - uy * 8.0 + ux * 4.0),
            Point::new(to.x - ux * 8.0 + uy * 4.0, to.y - uy * 8.0 - ux * 4.0),
        ],
        color,
    );
}

/// A line with dots travelling along it, `time` seconds in.
pub fn animated_line(ui: &mut Ui, a: Point, b: Point, color: Color, time: f32) {
    line(ui, a, b, 1.5, color);
    let length = (b.x - a.x).hypot(b.y - a.y);
    if length < 1.0 {
        return;
    }
    let offset = (time * 28.0).rem_euclid(18.0);
    for i in 0..(length / 18.0).ceil() as usize {
        let f = ((i as f32 * 18.0 + offset) / length).min(1.0);
        panel(
            ui,
            Rect::new(
                a.x + (b.x - a.x) * f - 1.5,
                a.y + (b.y - a.y) * f - 1.5,
                3.0,
                3.0,
            ),
            color,
            [0.0; 4],
            1.5,
        );
    }
}

/// An elbow connector: across from `from`, down or up at the midpoint, across to `to`.
pub fn elbow(ui: &mut Ui, from: Point, to: Point, width: f32, color: Color) {
    let mid = (from.x + to.x) * 0.5;
    line(ui, from, Point::new(mid, from.y), width, color);
    line(
        ui,
        Point::new(mid, from.y),
        Point::new(mid, to.y),
        width,
        color,
    );
    line(ui, Point::new(mid, to.y), to, width, color);
}

/// The outline of a [`GraphNode`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NodeShape {
    #[default]
    Box,
    Circle,
    Pill,
    Diamond,
}

/// A selectable node with a label.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GraphNode {
    shape: NodeShape,
    selected: bool,
}

impl GraphNode {
    pub fn new(shape: NodeShape) -> Self {
        Self {
            shape,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The node's outline for a `rect`-sized slot: a circle is as tall as the
    /// slot plus half its height again, centred on it.
    pub fn outline(&self, rect: Rect) -> Rect {
        match self.shape {
            NodeShape::Circle => {
                let size = rect.height * 1.55;
                let c = center(rect);
                Rect::new(c.x - size * 0.5, c.y - size * 0.5, size, size)
            }
            _ => rect,
        }
    }

    /// Draw the node; returns whether it was clicked.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, label: &str) -> bool {
        let variant = if self.selected {
            ButtonVariant::Selected
        } else {
            ButtonVariant::Outline
        };
        match self.shape {
            NodeShape::Diamond => {
                let response = ui.interact(id, rect);
                let p = ui.theme.palette;
                let c = center(rect);
                let top = Point::new(c.x, rect.y - 16.0);
                let left = Point::new(rect.x, c.y);
                let bottom = Point::new(c.x, rect.bottom() + 16.0);
                let right = Point::new(rect.right(), c.y);
                fill_triangles(
                    ui,
                    &[top, left, bottom, top, bottom, right],
                    if self.selected { p.accent } else { p.raised },
                );
                Label::new()
                    .weight(500)
                    .color(if self.selected { [1.0; 4] } else { p.text })
                    .align(TextAlign::Center)
                    .show(ui, rect, label);
                response.clicked
            }
            NodeShape::Circle => {
                Button::new(label)
                    .variant(variant)
                    .circular()
                    .font_size(13.0)
                    .padding(4.0)
                    .show(ui, id, self.outline(rect))
                    .clicked
            }
            NodeShape::Pill => {
                Button::new(label)
                    .variant(variant)
                    .circular()
                    .show(ui, id, rect)
                    .clicked
            }
            NodeShape::Box => {
                Button::new(label)
                    .variant(variant)
                    .show(ui, id, rect)
                    .clicked
            }
        }
    }
}

/// A step of a [`FlowGraph`], placed at `position` in the layout's units.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    pub id: String,
    pub title: String,
    pub description: String,
    pub position: Point,
    pub color: Color,
}

impl FlowNode {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        position: Point,
        color: Color,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            position,
            color,
        }
    }
}

/// A connection between two nodes, by index.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub color: Color,
    pub animated: bool,
    pub label: String,
}

impl GraphEdge {
    pub fn new(from: usize, to: usize, color: Color) -> Self {
        Self {
            from,
            to,
            color,
            animated: false,
            label: String::new(),
        }
    }

    pub fn animated(mut self) -> Self {
        self.animated = true;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

/// Steps joined left to right by elbow connectors.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowGraph<'a> {
    nodes: &'a [FlowNode],
    edges: &'a [GraphEdge],
    size: (f32, f32),
}

impl<'a> FlowGraph<'a> {
    /// `nodes` placed in a layout `width` × `height`, scaled to fit.
    pub fn new(nodes: &'a [FlowNode], edges: &'a [GraphEdge], width: f32, height: f32) -> Self {
        Self {
            nodes,
            edges,
            size: (width, height),
        }
    }

    /// Draw the graph in `area`; `selected` is the id of the selected node.
    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        area: Rect,
        camera: &mut GraphCamera,
        selected: &mut String,
        time: f32,
    ) {
        let p = ui.theme.palette;
        camera.follow(ui);
        let (scale, transform) = camera.transform(area, self.size.0, self.size.1);
        let rects: Vec<Rect> = self
            .nodes
            .iter()
            .map(|node| {
                let c = transform(node.position);
                Rect::new(
                    c.x - 85.0 * scale,
                    c.y - 32.0 * scale,
                    170.0 * scale,
                    64.0 * scale,
                )
            })
            .collect();
        ui.with_clip(area, |ui| {
            for edge in self.edges {
                let (a, b) = (rects[edge.from], rects[edge.to]);
                let from = Point::new(a.right(), a.y + a.height * 0.5);
                let to = Point::new(b.x, b.y + b.height * 0.5);
                let mid = (from.x + to.x) * 0.5;
                let segments = [
                    (from, Point::new(mid, from.y)),
                    (Point::new(mid, from.y), Point::new(mid, to.y)),
                    (Point::new(mid, to.y), to),
                ];
                for (start, end) in segments {
                    if edge.animated {
                        animated_line(ui, start, end, edge.color, time);
                    } else {
                        line(ui, start, end, 1.5, edge.color);
                    }
                }
                arrow(ui, Point::new(mid, to.y), to, edge.color);
                if !edge.label.is_empty() {
                    Label::muted().size(10.0).weight(500).show(
                        ui,
                        Rect::new(mid - 20.0, to.y - 22.0, 48.0, 20.0),
                        &edge.label,
                    );
                }
            }
            for (node, rect) in self.nodes.iter().zip(&rects) {
                if ui
                    .interact(&format!("{id}-node-{}", node.id), *rect)
                    .clicked
                {
                    *selected = node.id.clone();
                }
                let stroke = if *selected == node.id {
                    p.accent
                } else {
                    node.color
                };
                panel(ui, *rect, p.surface, stroke, 8.0 * scale);
                let mut lines = Strip::column(
                    rect.inset_by(crate::Insets::new(
                        10.0 * scale,
                        8.0 * scale,
                        10.0 * scale,
                        0.0,
                    )),
                    2.0 * scale,
                );
                Label::new().size(13.0 * scale).weight(600).show(
                    ui,
                    lines.leading(24.0 * scale),
                    &node.title,
                );
                Label::muted().size(11.0 * scale).show(
                    ui,
                    lines.leading(20.0 * scale),
                    &node.description,
                );
                panel(
                    ui,
                    rect.inset_by(crate::Insets::new(0.0, 10.0 * scale, 7.0 * scale, 0.0))
                        .align(
                            5.0 * scale,
                            5.0 * scale,
                            crate::HorizontalAlignment::Right,
                            crate::VerticalAlignment::Top,
                        ),
                    node.color,
                    [0.0; 4],
                    2.5 * scale,
                );
            }
            camera.drag(ui, id, area);
        });
    }
}

/// A node of a [`NetworkGraph`].
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkGraphNode {
    pub id: String,
    pub label: String,
    pub color: Color,
}

impl NetworkGraphNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>, color: Color) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color,
        }
    }
}

/// Round nodes around the most connected one, joined by straight links.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkGraph<'a> {
    nodes: &'a [NetworkGraphNode],
    edges: &'a [GraphEdge],
}

impl<'a> NetworkGraph<'a> {
    pub fn new(nodes: &'a [NetworkGraphNode], edges: &'a [GraphEdge]) -> Self {
        Self { nodes, edges }
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        id: &str,
        area: Rect,
        camera: &mut GraphCamera,
        selected: &mut String,
        time: f32,
    ) {
        let p = ui.theme.palette;
        camera.follow(ui);
        let layout = network_layout(
            &self
                .nodes
                .iter()
                .map(|n| NetworkNode {
                    id: n.id.clone(),
                    position: None,
                })
                .collect::<Vec<_>>(),
            &self
                .edges
                .iter()
                .map(|e| (self.nodes[e.from].id.clone(), self.nodes[e.to].id.clone()))
                .collect::<Vec<_>>(),
            Point::new(450.0, 210.0),
            135.0,
        );
        let (scale, transform) = camera.transform(area, 900.0, 420.0);
        let points: Vec<Point> = self
            .nodes
            .iter()
            .map(|n| transform(layout.positions[&n.id]))
            .collect();
        ui.with_clip(area, |ui| {
            for edge in self.edges {
                let (a, b) = (points[edge.from], points[edge.to]);
                let from = circle_connection(a, b, 35.0 * scale);
                let to = circle_connection(b, a, 35.0 * scale);
                if edge.animated {
                    animated_line(ui, from, to, edge.color, time);
                    arrow(ui, to, from, edge.color);
                } else {
                    line(ui, from, to, 1.5, edge.color);
                }
                arrow(ui, from, to, edge.color);
                if !edge.label.is_empty() {
                    let middle = Point::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5);
                    let tag = Rect::new(middle.x - 24.0, middle.y - 10.0, 48.0, 20.0);
                    panel(ui, tag, p.surface, [0.0; 4], 4.0);
                    Label::new()
                        .size(11.0)
                        .weight(500)
                        .color(edge.color)
                        .align(TextAlign::Center)
                        .show(ui, tag, &edge.label);
                }
            }
            for (node, point) in self.nodes.iter().zip(&points) {
                let size = 70.0 * scale;
                let rect = Rect::new(point.x - size * 0.5, point.y - size * 0.5, size, size);
                let chosen = *selected == node.id;
                let button = Button::new(&node.label)
                    .variant(if chosen {
                        ButtonVariant::Selected
                    } else {
                        ButtonVariant::Outline
                    })
                    .circular()
                    .font_size(12.0 * scale)
                    .padding(3.0 * scale);
                if button
                    .show(ui, &format!("{id}-node-{}", node.id), rect)
                    .clicked
                {
                    *selected = node.id.clone();
                }
                if let Some(command) = ui.last_command_mut() {
                    command.style.stroke = node.color;
                }
            }
            camera.drag(ui, id, area);
        });
    }
}

/// A node of a [`HierarchyGraphModel`].
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyNode {
    pub id: u64,
    pub parent: Option<u64>,
    pub label: String,
}

/// A tree of labelled nodes with one selected, and a draft label for editing it.
#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyGraphModel {
    pub nodes: Vec<HierarchyNode>,
    pub selected: u64,
    pub draft: String,
    next: u64,
}

impl HierarchyGraphModel {
    /// A tree from `(label, parent index)` pairs; the first is the root.
    pub fn new(nodes: &[(&str, Option<usize>)]) -> Self {
        let nodes: Vec<HierarchyNode> = nodes
            .iter()
            .enumerate()
            .map(|(id, (label, parent))| HierarchyNode {
                id: id as u64,
                parent: parent.map(|p| p as u64),
                label: (*label).into(),
            })
            .collect();
        Self {
            draft: nodes.first().map(|n| n.label.clone()).unwrap_or_default(),
            next: nodes.len() as u64,
            nodes,
            selected: 0,
        }
    }

    pub fn select(&mut self, id: u64) {
        if let Some(n) = self.nodes.iter().find(|n| n.id == id) {
            self.selected = id;
            self.draft = n.label.clone();
        }
    }

    /// Add a child under the selected node and select it; returns its id.
    pub fn add(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        self.nodes.push(HierarchyNode {
            id,
            parent: Some(self.selected),
            label: "New node".into(),
        });
        self.select(id);
        id
    }

    /// Give the selected node the draft label.
    pub fn rename(&mut self) -> bool {
        let label = self.draft.trim();
        if label.is_empty() {
            return false;
        }
        match self.nodes.iter_mut().find(|n| n.id == self.selected) {
            Some(n) => {
                n.label = label.into();
                true
            }
            None => false,
        }
    }

    /// Delete the selected node and everything under it; the root stays.
    pub fn delete(&mut self) -> bool {
        if self.selected == 0 {
            return false;
        }
        let parent = self
            .nodes
            .iter()
            .find(|n| n.id == self.selected)
            .and_then(|n| n.parent)
            .unwrap_or(0);
        let mut removed = vec![self.selected];
        let mut i = 0;
        while i < removed.len() {
            let current = removed[i];
            removed.extend(
                self.nodes
                    .iter()
                    .filter(|n| n.parent == Some(current))
                    .map(|n| n.id),
            );
            i += 1;
        }
        self.nodes.retain(|n| !removed.contains(&n.id));
        self.select(parent);
        true
    }

    /// Each node's `(id, depth, row)`: parents centred on their children.
    pub fn positions(&self) -> Vec<(u64, usize, f32)> {
        fn walk(
            tree: &HierarchyGraphModel,
            id: u64,
            depth: usize,
            row: &mut f32,
            out: &mut Vec<(u64, usize, f32)>,
        ) -> f32 {
            let children: Vec<_> = tree
                .nodes
                .iter()
                .filter(|n| n.parent == Some(id))
                .map(|n| n.id)
                .collect();
            let y = if children.is_empty() {
                let y = *row;
                *row += 1.0;
                y
            } else {
                let ys: Vec<_> = children
                    .into_iter()
                    .map(|child| walk(tree, child, depth + 1, row, out))
                    .collect();
                (ys[0] + ys[ys.len() - 1]) * 0.5
            };
            out.push((id, depth, y));
            y
        }
        let mut out = Vec::new();
        walk(self, 0, 0, &mut 0.0, &mut out);
        out
    }
}

/// A tree laid out left to right, with an optional toolbar to rename, add
/// and delete nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HierarchyGraph {
    editable: bool,
}

impl HierarchyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Draw the tree in `rect`, the toolbar along its top when editable.
    pub fn show(&self, ui: &mut Ui, id: &str, rect: Rect, model: &mut HierarchyGraphModel) {
        let mut column = Strip::column(rect, 18.0);
        if self.editable {
            let mut tools = Strip::row(column.leading(34.0), 12.0);
            let delete = tools.trailing(112.0);
            let add = tools.trailing(112.0);
            let rename = tools.trailing(100.0);
            TextInput::new("Node label").show(
                ui,
                &format!("{id}-label"),
                tools.rest(),
                &mut model.draft,
            );
            if Button::new("Rename")
                .variant(ButtonVariant::Secondary)
                .disabled(model.draft.trim().is_empty())
                .show(ui, &format!("{id}-rename"), rename)
                .clicked
            {
                model.rename();
            }
            if Button::new("Add child")
                .icon(Icon::Plus)
                .show(ui, &format!("{id}-add"), add)
                .clicked
            {
                model.add();
            }
            if Button::new("Delete")
                .variant(ButtonVariant::Danger)
                .disabled(model.selected == 0)
                .show(ui, &format!("{id}-delete"), delete)
                .clicked
            {
                model.delete();
            }
        }
        let area = column.rest().inset_by(crate::Insets::symmetric(4.0, 0.0));
        let positions = model.positions();
        let depth = positions.iter().map(|(_, d, _)| *d).max().unwrap_or(0);
        let rows = positions.iter().map(|(_, _, y)| *y).fold(0.0, f32::max) + 1.0;
        let node_w = (area.width / (depth + 1) as f32 - 24.0).clamp(24.0, 134.0);
        let node_h = (area.height / rows - 8.0).clamp(18.0, 40.0);
        let places: HashMap<u64, Rect> = positions
            .iter()
            .map(|(node, d, row)| {
                (
                    *node,
                    Rect::new(
                        area.x + *d as f32 * (area.width - node_w) / depth.max(1) as f32,
                        area.y + (row + 0.5) * area.height / rows - node_h * 0.5,
                        node_w,
                        node_h,
                    ),
                )
            })
            .collect();
        let edge = with_alpha(INFO, 1.0);
        for node in &model.nodes {
            if let Some(parent) = node.parent {
                let (a, b) = (places[&parent], places[&node.id]);
                elbow(
                    ui,
                    Point::new(a.right(), a.y + a.height * 0.5),
                    Point::new(b.x, b.y + b.height * 0.5),
                    1.5,
                    edge,
                );
            }
        }
        let mut select = None;
        for node in &model.nodes {
            if GraphNode::new(NodeShape::Box)
                .selected(model.selected == node.id)
                .show(
                    ui,
                    &format!("{id}-node-{}", node.id),
                    places[&node.id],
                    &node.label,
                )
            {
                select = Some(node.id);
            }
        }
        if let Some(node) = select {
            model.select(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_keeps_ids_and_removes_subtrees() {
        let mut tree = HierarchyGraphModel::new(&[("Root", None), ("A", Some(0)), ("B", Some(1))]);
        tree.select(1);
        tree.draft = "Renamed".into();
        assert!(tree.rename());
        assert_eq!(tree.nodes[1].label, "Renamed");
        assert!(tree.delete());
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.selected, 0);
        assert!(!tree.delete());
        assert_eq!(tree.add(), 3);
        assert_eq!(tree.positions().len(), tree.nodes.len());
    }
}
