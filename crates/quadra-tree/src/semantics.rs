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
    /// How many updates were made, and for each node the last one it was sent whole in.
    update: u32,
    stamps: Vec<u32>,
}

pub(crate) fn semantic_id(id: NodeId) -> u64 {
    (id.generation as u64) << 32 | id.index as u64
}

impl Tree {
    /// Says what a node is to a user: its role, its name, its states.
    pub fn set_semantics(&mut self, id: NodeId, semantics: Semantics) {
        self.update_semantics(id, |says| *says = semantics);
    }

    /// Changes what a node says of itself.
    pub fn update_semantics(&mut self, id: NodeId, f: impl FnOnce(&mut Semantics)) {
        if let Some(node) = self.get_mut(id) {
            let says = node.semantics.get_or_insert_default();
            let role = says.role;
            f(says);
            // A node that starts or stops meaning something changes what its parent holds.
            let around = node.parent.filter(|_| says.role != role);
            self.semantics_changed(around.unwrap_or(id));
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
    pub fn meaning(&self, id: NodeId) -> Semantics {
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

    /// Adds the semantic nodes of a subtree to `out`, and those that belong to the node
    /// around it to `held`. A text inside a node that is named by its content adds its text
    /// to `named` instead of a node.
    fn describe(
        &self,
        id: NodeId,
        mut named: Option<&mut String>,
        held: &mut Vec<u64>,
        out: &mut Vec<SemanticNode>,
    ) {
        if self.hidden(id) {
            return;
        }
        let node = &self.nodes[id.index as usize];
        let mut says = self.meaning(id);
        if says.role == Role::Text
            && let Some(named) = named.as_deref_mut()
        {
            named.push_str(&says.name);
            named.push(' ');
            return;
        }
        if says.role == Role::None {
            for &child in &node.children {
                self.describe(child, named.as_deref_mut(), held, out);
            }
            return;
        }
        let named_by_content = says.name.is_empty() && says.role.named_by_content();
        let (mut content, mut children) = (String::new(), Vec::new());
        // Under the window; the preview of a drag is not there for a reader.
        let above = self.overlays().filter(|&over| self.root == Some(id) && !self.is_drag_preview(over));
        for child in node.children.iter().copied().chain(above) {
            let named = if named_by_content { Some(&mut content) } else { None };
            self.describe(child, named, &mut children, out);
        }
        if named_by_content {
            says.name = content.trim_end().to_string();
        }
        self.resolve_relations(&mut says.relations);
        let focused = self.focus == Some(id) && self.active;
        let (bounds, focusable) = (self.window_rect(id), node.focusable);
        held.push(semantic_id(id));
        out.push(SemanticNode { id: semantic_id(id), semantics: says, bounds, focused, focusable, children });
    }

    /// Whether a node, and so what is under it, is left out: not displayed, or inert. The
    /// window never is: a listener always has that much.
    pub(crate) fn hidden(&self, id: NodeId) -> bool {
        let node = &self.nodes[id.index as usize];
        let out = self.layout.style(node.layout).display == Display::None
            || node.state.contains(crate::State::INERT);
        self.root != Some(id) && out
    }

    /// Whether a node is one of the semantic tree, and not skipped or part of a name.
    pub(crate) fn means(&self, id: NodeId) -> bool {
        let says = self.meaning(id);
        says.role != Role::None && !(says.role == Role::Text && self.names_its_parent(id))
    }

    /// What a node is under: its parent, and the window for an overlay.
    fn above(&self, id: NodeId) -> Option<NodeId> {
        let parent = self.nodes[id.index as usize].parent;
        parent.or_else(|| self.root.filter(|_| self.overlays().any(|overlay| overlay == id)))
    }

    /// The node of the semantic tree a node is, or is part of: itself, or the nearest around
    /// it. `None` for a node that is in no window. Something not displayed on the way means
    /// `None` too when `shown`; otherwise the nearest above it: where the focus is said to be.
    pub(crate) fn semantic_around(&self, id: NodeId, shown: bool) -> Option<NodeId> {
        let (mut top, mut at) = (None, id);
        loop {
            if self.hidden(at) {
                if shown {
                    return None;
                }
                top = None;
            } else if top.is_none() && self.means(at) {
                top = Some(at);
            }
            let Some(up) = self.above(at) else { break };
            at = up;
        }
        top.filter(|_| self.root == Some(at))
    }

    /// The node of the semantic tree that has the focus, while the window does.
    fn semantic_focus(&self) -> Option<u64> {
        let focus = self.focus.filter(|&id| self.active && self.is_alive(id))?;
        self.semantic_around(focus, false).map(semantic_id)
    }

    /// The whole semantic tree, as tests and the first update of a listener get it.
    pub fn semantic_tree(&self) -> TreeUpdate {
        let mut nodes = Vec::new();
        let Some(root) = self.root else { return TreeUpdate::default() };
        self.describe(root, None, &mut Vec::new(), &mut nodes);
        TreeUpdate { nodes, root: semantic_id(root), focus: self.semantic_focus(), scale: Some(self.scale) }
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
        let focus = self.semantic_focus();
        if !std::mem::replace(&mut self.listening.sent, true) {
            self.listening.focus = focus;
            return Some(self.semantic_tree());
        }
        // ponytail: a change is sent as the subtree of the semantic node it is in; a diff
        // per node if a listener with a large tree shows the cost.
        // The nodes to send are stamped with this update's number: asking whether one is
        // among them costs nothing, however many there are.
        let mut stamps = std::mem::take(&mut self.listening.stamps);
        stamps.resize(self.nodes.len(), 0);
        self.listening.update += 1;
        let (update, mut tops) = (self.listening.update, Vec::new());
        for id in std::mem::take(&mut self.listening.changed) {
            // What is not displayed, or is under something that is not, is in nobody's list
            // of children: nothing of it is sent.
            let top = if self.is_alive(id) { self.semantic_around(id, true) } else { None };
            if let Some(top) = top.filter(|top| stamps[top.index as usize] != update) {
                stamps[top.index as usize] = update;
                tops.push(top);
            }
        }
        let mut nodes: Vec<SemanticNode> = Vec::new();
        for top in tops {
            // What is under a node that is sent goes with it.
            let mut around = std::iter::successors(self.above(top), |&at| self.above(at));
            if !around.any(|at| stamps[at.index as usize] == update) {
                self.describe(top, None, &mut Vec::new(), &mut nodes);
            }
        }
        self.listening.stamps = stamps;
        if nodes.is_empty() && focus == self.listening.focus {
            return None;
        }
        self.listening.focus = focus;
        Some(TreeUpdate { nodes, root: self.root.map_or(0, semantic_id), focus, scale: Some(self.scale) })
    }

    /// Whether a text is part of the name of the semantic node around it.
    fn names_its_parent(&self, text: NodeId) -> bool {
        let around = std::iter::successors(self.above(text), |&at| self.above(at));
        let says = around.map(|at| self.meaning(at)).find(|says| says.role != Role::None);
        says.is_some_and(|says| says.name.is_empty() && says.role.named_by_content())
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
                    size: quadra_prim::Size::ZERO,
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
            SemanticAction::ScrollIntoView => {
                self.scroll_into_view(id);
                true
            }
            other => {
                let mut done = false;
                for handler in &mut self.nodes[id.index as usize].handlers {
                    if let Handler::Semantic(run) = handler {
                        done |= run(other);
                    }
                }
                // What nothing registered for is a click, when a click is what makes the
                // node what was asked: a closed one open, an open one closed, a box toggled.
                let says = self.meaning(id);
                let click = match other {
                    SemanticAction::Toggle => says.checked.is_some(),
                    SemanticAction::Expand => says.expanded == Some(false),
                    SemanticAction::Collapse => says.expanded == Some(true),
                    _ => false,
                };
                done || (click && self.act(id, &SemanticAction::Click))
            }
        })
    }
}
