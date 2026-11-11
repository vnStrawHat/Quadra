//! The semantic side of the tree (specs/12): what its nodes mean to a user, as a tree of its
//! own in which nodes that mean nothing are skipped. Assistive technology reads it through a
//! bridge, tests find and drive controls through it, and both act through semantic actions.

use quadra_a11y::{Role, SemanticAction, SemanticNode, Semantics, TreeUpdate};
use quadra_layout::Display;
use quadra_platform::{Mods, MouseButton};
use quadra_reactive::batch;

use crate::input::{Handler, PointerEvent, PointerKind};
use crate::node::NodeId;
use crate::tree::Tree;

/// What the tree keeps of whoever listens to its semantics.
#[derive(Default)]
pub(crate) struct Listening {
    on: bool,
    /// Nothing was sent yet: the first update is the whole tree.
    sent: bool,
    /// Nodes that changed in a way a listener can tell.
    changed: Vec<NodeId>,
    focus: Option<u64>,
}

fn semantic_id(id: NodeId) -> u64 {
    (id.generation as u64) << 32 | id.index as u64
}

impl Tree {
    /// Says what a node is to a user: its role, its name, its states.
    pub fn set_semantics(&mut self, id: NodeId, semantics: Semantics) {
        if let Some(node) = self.get_mut(id) {
            node.semantics = Some(Box::new(semantics));
            self.semantics_changed(id);
        }
    }

    /// Changes what a node says of itself.
    pub fn update_semantics(&mut self, id: NodeId, f: impl FnOnce(&mut Semantics)) {
        if let Some(node) = self.get_mut(id) {
            f(node.semantics.get_or_insert_default());
            self.semantics_changed(id);
        }
    }

    /// Notes that a listener would hear of a change at a node: its meaning, its text, its
    /// children, its place. Nothing is noted while nobody listens.
    pub(crate) fn semantics_changed(&mut self, id: NodeId) {
        if self.listening.on && self.listening.sent {
            self.listening.changed.push(id);
        }
    }

    /// What a node says of itself: what was set on it, completed by its widget (a text names
    /// itself by its text, an input gives its value). The root is the window.
    fn meaning(&self, id: NodeId) -> Semantics {
        let node = &self.nodes[id.index as usize];
        let mut says = node.semantics.as_deref().cloned().unwrap_or_default();
        if let Some(widget) = &node.widget {
            widget.semantics(&mut says);
        }
        if says.role == Role::None && self.root == Some(id) {
            says.role = Role::Window;
        }
        says
    }

    /// Adds the semantic nodes of a subtree to `out` and returns those that belong to the
    /// node around it. A text inside a node that is named by its content adds its text to
    /// `named` instead of a node.
    fn describe(&self, id: NodeId, mut named: Option<&mut String>, out: &mut Vec<SemanticNode>) -> Vec<u64> {
        let node = &self.nodes[id.index as usize];
        if self.layout.style(node.layout).display == Display::None {
            return Vec::new();
        }
        let mut says = self.meaning(id);
        if says.role == Role::Text
            && let Some(named) = named.as_deref_mut()
        {
            named.push_str(&says.name);
            named.push(' ');
            return Vec::new();
        }
        let named_by_content = says.name.is_empty() && says.role.named_by_content();
        let mut content = String::new();
        let mut children = Vec::new();
        let above: Vec<NodeId> = if self.root == Some(id) { self.overlays().collect() } else { Vec::new() };
        for &child in node.children.iter().chain(&above) {
            let into = match says.role {
                Role::None => named.as_deref_mut(),
                _ if named_by_content => Some(&mut content),
                _ => None,
            };
            children.extend(self.describe(child, into, out));
        }
        if says.role == Role::None {
            return children;
        }
        if named_by_content {
            says.name = content.trim_end().to_string();
        }
        let focused = self.focus == Some(id) && self.active;
        let (bounds, focusable) = (self.window_rect(id), node.focusable);
        out.push(SemanticNode { id: semantic_id(id), semantics: says, bounds, focused, focusable, children });
        vec![semantic_id(id)]
    }

    /// The whole semantic tree, as tests and the first update of a listener get it.
    pub fn semantic_tree(&self) -> TreeUpdate {
        let mut nodes = Vec::new();
        let Some(root) = self.root else { return TreeUpdate::default() };
        self.describe(root, None, &mut nodes);
        let focus = self.focus.filter(|_| self.active).map(semantic_id);
        TreeUpdate { nodes, root: semantic_id(root), focus }
    }

    /// The node of a semantic id, if it is still there.
    pub fn node_of(&self, id: u64) -> Option<NodeId> {
        let node = NodeId { index: id as u32, generation: (id >> 32) as u32 };
        self.is_alive(node).then_some(node)
    }

    /// Assistive technology started listening, or stopped. Nothing is built for it before the
    /// first, and nothing after the second.
    pub fn listen(&mut self, on: bool) {
        self.listening = Listening { on, ..Listening::default() };
    }

    /// What a listener has not heard yet: everything the first time, then the parts that
    /// changed, or `None`. Call it after a frame.
    pub fn take_semantic_update(&mut self) -> Option<TreeUpdate> {
        if !self.listening.on {
            return None;
        }
        let focus = self.focus.filter(|_| self.active).map(semantic_id);
        if !std::mem::replace(&mut self.listening.sent, true) {
            self.listening.focus = focus;
            return Some(self.semantic_tree());
        }
        let changed = std::mem::take(&mut self.listening.changed);
        let mut nodes: Vec<SemanticNode> = Vec::new();
        for id in changed.into_iter().filter(|&id| self.is_alive(id)) {
            // ponytail: the change is sent as the subtree of the semantic node it is in; a
            // diff per node if a listener with a large tree shows the cost.
            let around = self.path(id).into_iter().find(|&n| {
                let says = self.meaning(n);
                let inside_name = says.role == Role::Text && self.names_its_parent(n);
                says.role != Role::None && !inside_name
            });
            if let Some(top) = around.filter(|&top| !nodes.iter().any(|n| n.id == semantic_id(top))) {
                self.describe(top, None, &mut nodes);
            }
        }
        if nodes.is_empty() && focus == self.listening.focus {
            return None;
        }
        self.listening.focus = focus;
        Some(TreeUpdate { nodes, root: self.root.map_or(0, semantic_id), focus })
    }

    /// Whether a text is part of the name of the semantic node around it.
    fn names_its_parent(&self, text: NodeId) -> bool {
        let around =
            self.path(text).into_iter().skip(1).map(|n| self.meaning(n)).find(|s| s.role != Role::None);
        around.is_some_and(|says| says.name.is_empty() && says.role.named_by_content())
    }

    /// Runs `f` for the semantic actions a node is asked to do, other than a click and the
    /// focus, which every node knows. `f` says whether it did what was asked.
    pub fn on_semantic(&mut self, id: NodeId, f: impl FnMut(&SemanticAction) -> bool + 'static) {
        self.add_handler(id, Handler::Semantic(Box::new(f)));
    }

    /// Does what assistive technology, or a test, asks of a node. A click runs the click
    /// handlers on the node's path, without the pointer: the node may be covered or out of
    /// view. Returns whether anything was done.
    pub fn act(&mut self, id: NodeId, action: &SemanticAction) -> bool {
        if !self.is_alive(id) {
            return false;
        }
        batch(|| match action {
            SemanticAction::Focus => {
                self.focus(id);
                self.focus == Some(id)
            }
            SemanticAction::Click => {
                self.focus_on_press(&self.path(id));
                let at = self.window_rect(id).center();
                let click = PointerEvent {
                    kind: PointerKind::Click,
                    button: MouseButton::Left,
                    at,
                    local: at,
                    mods: Mods::NONE,
                    clicks: 1,
                    delta: quadra_prim::Point::ZERO,
                    target: id,
                    node: id,
                    stopped: false,
                    captured: None,
                };
                self.send(click);
                true
            }
            other => {
                let mut done = false;
                for handler in &mut self.nodes[id.index as usize].handlers {
                    if let Handler::Semantic(run) = handler {
                        done |= run(other);
                    }
                }
                done
            }
        })
    }
}
