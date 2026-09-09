use super::*;
#[derive(Clone, Debug)]
pub struct Node {
    pub id: u64,
    pub parent: Option<u64>,
    pub label: String,
}
#[derive(Clone, Debug)]
pub struct Hierarchy {
    pub nodes: Vec<Node>,
    pub selected: u64,
    pub draft: String,
    next: u64,
}
impl Default for Hierarchy {
    fn default() -> Self {
        let labels = [
            ("Core App", None),
            ("UI Layer", Some(0)),
            ("Database", Some(0)),
            ("GraphQL API", Some(0)),
            ("Views & Pages", Some(1)),
            ("Shared Parts", Some(1)),
            ("Web Views", Some(4)),
            ("Mobile Views", Some(4)),
            ("Form Controls", Some(5)),
            ("Navigation", Some(5)),
        ];
        Self {
            nodes: labels
                .iter()
                .enumerate()
                .map(|(id, (label, parent))| Node {
                    id: id as u64,
                    parent: *parent,
                    label: (*label).into(),
                })
                .collect(),
            selected: 0,
            draft: "Core App".into(),
            next: 10,
        }
    }
}
impl Hierarchy {
    pub fn select(&mut self, id: u64) {
        if let Some(n) = self.nodes.iter().find(|n| n.id == id) {
            self.selected = id;
            self.draft = n.label.clone();
        }
    }
    pub fn add(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        self.nodes.push(Node {
            id,
            parent: Some(self.selected),
            label: "New node".into(),
        });
        self.select(id);
        id
    }
    pub fn rename(&mut self) -> bool {
        let label = self.draft.trim();
        if label.is_empty() {
            return false;
        }
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == self.selected) {
            n.label = label.into();
            return true;
        }
        false
    }
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
    pub fn positions(&self) -> Vec<(u64, usize, f32)> {
        fn walk(
            tree: &Hierarchy,
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
impl Gallery {
    pub(super) fn hierarchy(&mut self, p: &mut Painter, rect: Rect) {
        p.card(rect);
        p.label(
            rect.x + 20.0,
            rect.y + 18.0,
            rect.width - 170.0,
            30.0,
            "Hierarchy Graph (Mind Map / Org Structure)",
            18.0,
            600,
            p.palette.fg,
        );
        p.check(
            "tree-edit",
            r(rect.x + rect.width - 130.0, rect.y + 18.0, 110.0, 30.0),
            "Editable",
            &mut self.tree_edit,
            false,
            false,
        );
        if self.tree_edit {
            p.field(
                "tree-label",
                r(rect.x + 20.0, rect.y + 62.0, rect.width - 400.0, 34.0),
                &mut self.tree.draft,
                "Node label",
                false,
            );
            if p.button(
                "tree-rename",
                r(rect.x + rect.width - 368.0, rect.y + 62.0, 100.0, 34.0),
                "Rename",
                1,
                self.tree.draft.trim().is_empty(),
            ) {
                self.tree.rename();
            }
            if p.button(
                "tree-add",
                r(rect.x + rect.width - 256.0, rect.y + 62.0, 112.0, 34.0),
                "Add child",
                0,
                false,
            ) {
                self.tree.add();
            }
            if p.button(
                "tree-delete",
                r(rect.x + rect.width - 132.0, rect.y + 62.0, 112.0, 34.0),
                "Delete",
                4,
                self.tree.selected == 0,
            ) {
                self.tree.delete();
            }
        }
        let positions = self.tree.positions();
        let depth = positions.iter().map(|(_, d, _)| *d).max().unwrap_or(0);
        let rows = positions.iter().map(|(_, _, y)| *y).fold(0.0, f32::max) + 1.0;
        let area = r(
            rect.x + 24.0,
            rect.y + if self.tree_edit { 116.0 } else { 66.0 },
            rect.width - 48.0,
            rect.height - if self.tree_edit { 140.0 } else { 90.0 },
        );
        let node_w = (area.width / (depth + 1) as f32 - 24.0)
            .min(134.0)
            .max(24.0);
        let node_h = (area.height / rows - 8.0).clamp(18.0, 40.0);
        let points: HashMap<u64, Rect> = positions
            .iter()
            .map(|(id, d, row)| {
                (
                    *id,
                    r(
                        area.x + *d as f32 * (area.width - node_w) / depth.max(1) as f32,
                        area.y + (row + 0.5) * area.height / rows - node_h * 0.5,
                        node_w,
                        node_h,
                    ),
                )
            })
            .collect();
        for node in &self.tree.nodes {
            if let Some(parent) = node.parent {
                let a = points[&parent];
                let b = points[&node.id];
                let mid = (a.x + a.width + b.x) * 0.5;
                p.line(
                    a.x + a.width,
                    a.y + a.height * 0.5,
                    mid,
                    a.y + a.height * 0.5,
                    BLUE,
                    1.5,
                );
                p.line(
                    mid,
                    a.y + a.height * 0.5,
                    mid,
                    b.y + b.height * 0.5,
                    BLUE,
                    1.5,
                );
                p.line(
                    mid,
                    b.y + b.height * 0.5,
                    b.x,
                    b.y + b.height * 0.5,
                    BLUE,
                    1.5,
                );
            }
        }
        let mut select = None;
        for node in &self.tree.nodes {
            if p.node(
                &format!("tree-node-{}", node.id),
                points[&node.id],
                &node.label,
                self.tree.selected == node.id,
                0,
            ) {
                select = Some(node.id);
            }
        }
        if let Some(id) = select {
            self.tree.select(id);
        }
    }
}
