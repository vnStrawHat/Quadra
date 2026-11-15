//! The bridge to assistive technology (specs/12, "Accessibility"): a window's semantic tree,
//! told to UI Automation by the adapter of `accesskit`, and what a screen reader asks of a
//! node, brought back as semantic actions.

mod text;

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, Sender, channel};

use accesskit::{
    Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, Affine, Node, NodeId, Toggled,
    TreeId, TreeInfo,
};
use accesskit_windows::Adapter;
use quadra_a11y::{Relation, Role, SemanticAction, SemanticNode, TreeUpdate};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Posted to a window: assistive technology asked something of a node.
pub(crate) const WM_SEMANTIC: u32 = WM_APP + 2;

fn role(role: Role) -> accesskit::Role {
    use accesskit::Role as To;
    match role {
        Role::None => To::GenericContainer,
        Role::Window => To::Window,
        Role::Group => To::Group,
        Role::Text => To::Label,
        Role::Heading => To::Heading,
        Role::Button => To::Button,
        Role::Link => To::Link,
        Role::Checkbox => To::CheckBox,
        Role::Radio => To::RadioButton,
        Role::Switch => To::Switch,
        Role::TextInput => To::TextInput,
        Role::Slider => To::Slider,
        Role::ProgressBar => To::ProgressIndicator,
        Role::Image => To::Image,
        Role::List => To::List,
        Role::ListItem => To::ListItem,
        Role::Menu => To::Menu,
        Role::MenuItem => To::MenuItem,
        Role::TabList => To::TabList,
        Role::Tab => To::Tab,
        Role::TabPanel => To::TabPanel,
        Role::Dialog => To::Dialog,
        Role::Tooltip => To::Tooltip,
        Role::Table => To::Table,
        Role::Row => To::Row,
        Role::Cell => To::Cell,
        Role::Tree => To::Tree,
        Role::TreeItem => To::TreeItem,
        Role::ScrollView => To::ScrollView,
    }
}

fn rect(b: quadra_prim::Rect) -> accesskit::Rect {
    accesskit::Rect { x0: b.x as f64, y0: b.y as f64, x1: b.right() as f64, y1: b.bottom() as f64 }
}

fn node(from: &SemanticNode) -> Node {
    let says = &from.semantics;
    let mut node = Node::new(role(says.role));
    match says.role {
        Role::Text => node.set_value(says.name.as_str()), // a static text is read by its value
        _ if !says.name.is_empty() => node.set_label(says.name.as_str()),
        _ => {}
    }
    if let Some(value) = &says.value {
        node.set_value(value.as_str());
    }
    if let Some(description) = &says.description {
        node.set_description(description.as_str());
    }
    if let Some(checked) = says.checked {
        node.set_toggled(Toggled::from(checked));
    }
    // What is one of a set says so when it is not the chosen one too.
    if says.selected || matches!(says.role, Role::Tab | Role::TreeItem) {
        node.set_selected(says.selected);
    }
    if let Some(expanded) = says.expanded {
        node.set_expanded(expanded);
        node.add_action(Action::Expand);
        node.add_action(Action::Collapse);
    }
    if says.disabled {
        node.set_disabled();
    }
    if let Some(range) = says.range {
        node.set_numeric_value(range.value);
        node.set_min_numeric_value(range.min);
        node.set_max_numeric_value(range.max);
        if range.step > 0.0 {
            node.set_numeric_value_step(range.step);
        }
    }
    if let Some(at) = says.position {
        node.set_position_in_set(at);
    }
    if let Some(count) = says.set_size {
        node.set_size_of_set(count);
    }
    if let Some(level) = says.level {
        node.set_level(level.saturating_sub(1)); // the adapter's levels are from 0
    }
    if let Some(id) = &says.test_id {
        node.set_author_id(id.as_str());
    }
    for &(relation, to) in &says.relations {
        match relation {
            Relation::LabelledBy => node.push_labelled_by(NodeId(to)),
            Relation::DescribedBy => node.push_described_by(NodeId(to)),
            Relation::Controls => node.push_controlled(NodeId(to)),
        }
    }
    node.set_bounds(rect(from.bounds));
    let mut children: Vec<NodeId> = from.children.iter().map(|&c| NodeId(c)).collect();
    if let Some((runs, selection)) = text::children_and_selection(from) {
        // Its text is its runs'; the caret and the selection are places in them.
        children.extend(runs);
        node.set_text_selection(selection);
        node.add_action(Action::SetTextSelection);
    }
    node.set_children(children);
    if from.focusable {
        node.add_action(Action::Focus);
    }
    // ponytail: by role; from the click handlers on the node when a custom control needs it.
    use Role::*;
    if matches!(says.role, Button | Link | Checkbox | Radio | Switch | MenuItem | Tab | ListItem | TreeItem) {
        node.add_action(Action::Click);
    }
    if says.role == TextInput {
        node.add_action(Action::SetValue);
    }
    if says.role == Slider {
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);
    }
    node
}

/// A semantic update as the adapter takes it. Bounds are in logical pixels of the window:
/// the root carries the scale to device pixels (the tree sends it again when the scale
/// changes).
pub(crate) fn tree_update(update: &TreeUpdate, scale: f32, runs: &mut text::Runs) -> accesskit::TreeUpdate {
    let convert = |from: &SemanticNode| {
        let mut node = node(from);
        if from.id == update.root {
            node.set_transform(Affine::scale(scale as f64));
        }
        // An input is sent with the runs of its text, always: they change together.
        std::iter::once((NodeId(from.id), node)).chain(text::runs(from, runs))
    };
    accesskit::TreeUpdate {
        nodes: update.nodes.iter().flat_map(convert).collect(),
        tree: Some(TreeInfo::new(NodeId(update.root))),
        tree_id: TreeId::ROOT,
        focus: NodeId(update.focus.unwrap_or(update.root)),
    }
}

/// What assistive technology asks, as a semantic action on a node: `None` for what Quadra
/// has no action for. What is asked of a run of an input is asked of the input.
fn request(request: ActionRequest, runs: &text::Runs) -> Option<(u64, SemanticAction)> {
    let action = match (request.action, request.data) {
        (Action::Click, _) => SemanticAction::Click,
        (Action::Focus, _) => SemanticAction::Focus,
        (Action::SetValue, Some(ActionData::Value(value))) => SemanticAction::SetValue(value.into()),
        (Action::Expand, _) => SemanticAction::Expand,
        (Action::Collapse, _) => SemanticAction::Collapse,
        (Action::ScrollIntoView, _) => SemanticAction::ScrollIntoView,
        (Action::Increment, _) => SemanticAction::Increment,
        (Action::Decrement, _) => SemanticAction::Decrement,
        (Action::SetTextSelection, Some(ActionData::SetTextSelection(chosen))) => {
            let place = |at: accesskit::TextPosition| text::of(runs, at.node, at.character_index).1;
            SemanticAction::Select { anchor: place(chosen.anchor), focus: place(chosen.focus) }
        }
        _ => return None,
    };
    Some((text::of(runs, request.target_node, 0).0, action))
}

/// Requests arrive on a thread of the adapter's choosing: they are queued, and the window
/// is posted a message to come and take them on its own thread.
struct Requests {
    hwnd: isize,
    to: Sender<ActionRequest>,
}

impl ActionHandler for Requests {
    fn do_action(&mut self, request: ActionRequest) {
        if self.to.send(request).is_ok() {
            // SAFETY: posting to a window that is gone fails, and that is all it does.
            let _ = unsafe { PostMessageW(Some(HWND(self.hwnd as _)), WM_SEMANTIC, WPARAM(0), LPARAM(0)) };
        }
    }
}

/// Notes that the tree was asked for. It is given later, in an update: the app may be in
/// the middle of something when the question comes.
struct Asked(bool);

impl ActivationHandler for Asked {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        self.0 = true;
        None
    }
}

/// A window's side of the bridge. Nothing is borrowed across a call that can send the window
/// a message: answering one, and raising events, can bring `WM_GETOBJECT` back in.
pub(crate) struct Access {
    adapter: RefCell<Adapter>,
    requests: Receiver<ActionRequest>,
    /// The runs of the inputs told so far.
    runs: RefCell<text::Runs>,
}

impl Access {
    /// Must not be called while the window handles `WM_GETOBJECT`.
    pub fn new(hwnd: HWND) -> Access {
        let (to, requests) = channel();
        let adapter = Adapter::new(hwnd, false, Requests { hwnd: hwnd.0 as isize, to });
        Access { adapter: RefCell::new(adapter), requests, runs: RefCell::default() }
    }

    /// Answers `WM_GETOBJECT`: the result, if the message was one for the adapter, and
    /// whether this was the first time the tree was asked for.
    pub fn object(&self, wparam: WPARAM, lparam: LPARAM) -> (Option<LRESULT>, bool) {
        let mut asked = Asked(false);
        let result = self.adapter.borrow_mut().handle_wm_getobject(wparam, lparam, &mut asked);
        (result.map(Into::into), asked.0)
    }

    pub fn window_focused(&self, focused: bool) {
        let events = self.adapter.borrow_mut().update_window_focus_state(focused);
        if let Some(events) = events {
            events.raise();
        }
    }

    pub fn update(&self, update: &TreeUpdate, scale: f32) {
        let mut runs = self.runs.borrow_mut();
        let events = self.adapter.borrow_mut().update_if_active(|| tree_update(update, scale, &mut runs));
        drop(runs);
        if let Some(events) = events {
            events.raise();
        }
    }

    /// What was asked since the last time.
    pub fn requests(&self) -> impl Iterator<Item = (u64, SemanticAction)> + '_ {
        self.requests.try_iter().filter_map(|asked| request(asked, &self.runs.borrow()))
    }
}
