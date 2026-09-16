//! Publishing the immediate host's accessibility tree through AccessKit.

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Node, NodeId,
    Rect as AccessRect, Role as AccessRole, Toggled, Tree, TreeId, TreeUpdate,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use ui_kit_immediate::{AccessAction, AccessNode, Role};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

const ROOT: NodeId = NodeId(0);

#[derive(Default)]
struct Shared {
    latest: Option<TreeUpdate>,
    names: HashMap<u64, String>,
    actions: Vec<AccessAction>,
}

struct Activation(Arc<Mutex<Shared>>, String);
impl ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        let shared = self.0.lock().unwrap();
        Some(
            shared
                .latest
                .clone()
                .unwrap_or_else(|| tree_update(&[], None, &self.1, 1.0).0),
        )
    }
}

struct Actions(Arc<Mutex<Shared>>);
impl ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        let mut shared = self.0.lock().unwrap();
        let Some(id) = shared.names.get(&request.target_node.0).cloned() else {
            return;
        };
        match request.action {
            Action::Click => shared.actions.push(AccessAction::Click(id)),
            Action::Focus => shared.actions.push(AccessAction::Focus(id)),
            _ => {}
        }
    }
}

struct Deactivation;
impl DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {}
}

/// A window's accessibility, kept up to date from each frame's
/// [`AccessNode`]s, with what screen readers ask for coming back as
/// [`AccessAction`]s for the next frame's input.
pub struct AccessibilityBridge {
    adapter: accesskit_winit::Adapter,
    shared: Arc<Mutex<Shared>>,
    title: String,
}

impl AccessibilityBridge {
    /// Create the bridge for `window`, which must not have been shown yet:
    /// create the window invisible, then this, then make it visible.
    pub fn new(event_loop: &ActiveEventLoop, window: &Window, title: impl Into<String>) -> Self {
        let title = title.into();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let adapter = accesskit_winit::Adapter::with_direct_handlers(
            event_loop,
            window,
            Activation(shared.clone(), title.clone()),
            Actions(shared.clone()),
            Deactivation,
        );
        Self {
            adapter,
            shared,
            title,
        }
    }

    /// Pass every window event through here before handling it.
    pub fn process_event(&mut self, window: &Window, event: &WindowEvent) {
        self.adapter.process_event(window, event);
    }

    /// Publish a frame's nodes. `scale` turns their logical pixels into the
    /// window's physical ones.
    pub fn update(&mut self, nodes: &[AccessNode], focus: Option<&str>, scale: f64) {
        let (update, names) = tree_update(nodes, focus, &self.title, scale);
        {
            let mut shared = self.shared.lock().unwrap();
            shared.latest = Some(update.clone());
            shared.names = names;
        }
        self.adapter.update_if_active(|| update);
    }

    /// What assistive technology asked for since the last call.
    pub fn take_actions(&mut self) -> Vec<AccessAction> {
        std::mem::take(&mut self.shared.lock().unwrap().actions)
    }
}

fn node_id(id: &str) -> NodeId {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    // Zero is the window itself.
    NodeId(hasher.finish().max(1))
}

fn role(role: Role) -> AccessRole {
    match role {
        Role::Button => AccessRole::Button,
        Role::CheckBox => AccessRole::CheckBox,
        Role::Switch => AccessRole::Switch,
        Role::RadioButton => AccessRole::RadioButton,
        Role::Slider => AccessRole::Slider,
        Role::TextInput => AccessRole::TextInput,
        Role::ComboBox => AccessRole::ComboBox,
        Role::ListBoxOption => AccessRole::ListBoxOption,
        Role::Menu => AccessRole::Menu,
        Role::MenuItem => AccessRole::MenuItem,
        Role::MenuBar => AccessRole::MenuBar,
        Role::Tab => AccessRole::Tab,
        Role::TabList => AccessRole::TabList,
        Role::Tree => AccessRole::Tree,
        Role::TreeItem => AccessRole::TreeItem,
        Role::Table => AccessRole::Table,
        Role::Row => AccessRole::Row,
        Role::Cell => AccessRole::Cell,
        Role::Dialog => AccessRole::Dialog,
        Role::Tooltip => AccessRole::Tooltip,
        Role::Alert => AccessRole::Alert,
        Role::Label => AccessRole::Label,
        Role::Image => AccessRole::Image,
        Role::ScrollView => AccessRole::ScrollView,
        Role::Group => AccessRole::Group,
    }
}

/// The AccessKit tree for a frame's nodes, and the table from node ids back
/// to control ids.
///
/// A node sits under its [`parent`](AccessNode::parent) when that parent was
/// described this frame too, and under the window otherwise, keeping the
/// order the nodes were drawn in among siblings.
pub fn tree_update(
    nodes: &[AccessNode],
    focus: Option<&str>,
    title: &str,
    scale: f64,
) -> (TreeUpdate, HashMap<u64, String>) {
    let mut names = HashMap::new();
    let ids: HashMap<&str, NodeId> = nodes
        .iter()
        .map(|node| (node.id.as_str(), node_id(&node.id)))
        .collect();
    let mut window = Node::new(AccessRole::Window);
    window.set_label(title);
    let mut built: Vec<(NodeId, Node)> = vec![];
    let mut index: HashMap<NodeId, usize> = HashMap::new();
    for node in nodes {
        let id = ids[node.id.as_str()];
        names.insert(id.0, node.id.clone());
        let mut out = Node::new(role(node.role));
        if !node.label.is_empty() {
            out.set_label(node.label.clone());
        }
        if let Some(value) = &node.value {
            out.set_value(value.clone());
        }
        let r = node.rect;
        out.set_bounds(AccessRect {
            x0: r.x as f64 * scale,
            y0: r.y as f64 * scale,
            x1: (r.x + r.width) as f64 * scale,
            y1: (r.y + r.height) as f64 * scale,
        });
        if let Some(checked) = node.checked {
            out.set_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            });
        }
        if let Some(selected) = node.selected {
            out.set_selected(selected);
        }
        if let Some(expanded) = node.expanded {
            out.set_expanded(expanded);
        }
        if node.disabled {
            out.set_disabled();
        } else {
            out.add_action(Action::Click);
            out.add_action(Action::Focus);
        }
        index.insert(id, built.len());
        built.push((id, out));
    }
    for node in nodes {
        let id = ids[node.id.as_str()];
        let parent = node
            .parent
            .as_deref()
            .and_then(|parent| ids.get(parent))
            .filter(|parent| **parent != id);
        match parent {
            Some(parent) => built[index[parent]].1.push_child(id),
            None => window.push_child(id),
        }
    }
    let focus = focus
        .map(node_id)
        .filter(|id| names.contains_key(&id.0))
        .unwrap_or(ROOT);
    let mut all = vec![(ROOT, window)];
    all.extend(built);
    let mut tree = Tree::new(ROOT);
    tree.toolkit_name = Some("ui-kit".into());
    (
        TreeUpdate {
            nodes: all,
            tree: Some(tree),
            tree_id: TreeId::ROOT,
            focus,
        },
        names,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_kit_core::Rect;

    #[test]
    fn nodes_become_children_of_the_window_and_map_back_to_their_ids() {
        let mut save = AccessNode::new(
            "save",
            Role::Button,
            "Save",
            Rect::new(10.0, 10.0, 80.0, 30.0),
        );
        save.focused = true;
        let (update, names) = tree_update(&[save], Some("save"), "Editor", 2.0);
        assert_eq!(update.nodes.len(), 2);
        let (id, node) = &update.nodes[1];
        assert_eq!(node.role(), AccessRole::Button);
        assert_eq!(node.label(), Some("Save"));
        assert_eq!(node.bounds().map(|b| b.x1), Some(180.0));
        assert_eq!(update.focus, *id);
        assert_eq!(names.get(&id.0).map(String::as_str), Some("save"));
    }

    #[test]
    fn a_node_sits_under_its_parent_and_orphans_under_the_window() {
        let menu = AccessNode::new("file.menu", Role::Menu, "", Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut item = AccessNode::new("file.item.0", Role::MenuItem, "Open", Rect::new(0.0, 0.0, 100.0, 28.0));
        item.parent = Some("file.menu".into());
        let mut orphan = AccessNode::new("lost", Role::Button, "Lost", Rect::default());
        orphan.parent = Some("gone".into());
        let (update, _) = tree_update(&[menu, item, orphan], None, "App", 1.0);
        let find = |id: &str| update.nodes.iter().find(|(n, _)| *n == node_id(id)).unwrap();
        assert_eq!(find("file.menu").1.children(), &[node_id("file.item.0")]);
        let window = &update.nodes[0].1;
        assert_eq!(window.children(), &[node_id("file.menu"), node_id("lost")]);
    }
}
