//! The semantic tree (specs/12): one description of the UI for assistive technology, for
//! tests and for the inspector. What a screen reader can reach, a test can reach, the same
//! way.
//!
//! This crate has no OS code: the bridge to an operating system is in its native crate.

use std::collections::HashMap;
use std::fmt::Write;

use quadra_prim::Rect;

/// What a node is to a user.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Role {
    /// Nothing: the node is skipped, and what it holds belongs to the node around it.
    #[default]
    None,
    Window,
    Group,
    Text,
    Heading,
    Button,
    Link,
    Checkbox,
    Radio,
    Switch,
    TextInput,
    Slider,
    ProgressBar,
    Image,
    List,
    ListItem,
    Menu,
    MenuItem,
    TabList,
    Tab,
    TabPanel,
    Dialog,
    Tooltip,
    Table,
    Row,
    Cell,
    Tree,
    TreeItem,
    ScrollView,
}

impl Role {
    /// Roles whose name, when none is given, is the text they hold: the text is then part of
    /// the node and not a node of its own.
    pub fn named_by_content(self) -> bool {
        use Role::*;
        matches!(
            self,
            Button | Link | Checkbox | Radio | Switch | MenuItem | Tab | ListItem | TreeItem | Heading | Cell
        )
    }
}

/// A value in a range: of a slider, a progress bar. `step` 0: it takes any value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RangeValue {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// How a node stands to another of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// The other names it: a label beside a field.
    LabelledBy,
    /// The other says more of it: a hint under a field.
    DescribedBy,
    /// It changes what the other shows: a tab its panel, a combobox its list.
    Controls,
}

/// What a node says of itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Semantics {
    pub role: Role,
    /// What it is called: a button's label, a field's label.
    pub name: String,
    /// What it holds: the text of an input, the position of a slider.
    pub value: Option<String>,
    pub description: Option<String>,
    /// For what can be checked: on, off, or neither said.
    pub checked: Option<bool>,
    pub selected: bool,
    /// For what opens: open, closed, or neither said.
    pub expanded: Option<bool>,
    pub disabled: bool,
    /// The number of a slider or a progress bar, besides the text of `value`.
    pub range: Option<RangeValue>,
    /// Where an item is in its set, from 0: a tab, an option, a row of a list.
    pub position: Option<usize>,
    /// How many items the set of a container holds, those that are not mounted too: a list
    /// of a million rows says so while it holds forty nodes.
    pub set_size: Option<usize>,
    /// Of a text input: the selection, in characters of `value` from its start, as where it
    /// was begun (the anchor) and where the caret is. Equal: there is only the caret.
    pub selection: Option<(usize, usize)>,
    /// A name that does not change with the language or the content: the automation id for
    /// assistive technology, a selector for tests.
    pub test_id: Option<String>,
    /// How it stands to other nodes, by their ids.
    pub relations: Vec<(Relation, u64)>,
}

/// What assistive technology, or a test, asks of a node. It does not go through pointer
/// events: it reaches a node that is covered or scrolled out of view.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticAction {
    Click,
    Focus,
    SetValue(String),
    Toggle,
    Expand,
    Collapse,
    ScrollIntoView,
    Increment,
    Decrement,
    /// Of a text input: select from `anchor` to `focus`, characters of its value from its
    /// start; the caret is at `focus`.
    Select {
        anchor: usize,
        focus: usize,
    },
}

/// A node of the semantic tree as it is sent.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: u64,
    pub semantics: Semantics,
    /// In the window, in logical pixels.
    pub bounds: Rect,
    pub focused: bool,
    /// What can have the focus.
    pub focusable: bool,
    pub children: Vec<u64>,
}

/// Nodes that are new or changed. The first update of a tree holds all of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeUpdate {
    pub nodes: Vec<SemanticNode>,
    pub root: u64,
    pub focus: Option<u64>,
}

/// What a listener holds after the updates it was sent: the tree, when they were well formed.
/// It takes updates as the adapter of an OS does, and refuses what that adapter refuses:
/// tests follow a tree with one.
#[derive(Default)]
pub struct Mirror {
    nodes: HashMap<u64, SemanticNode>,
}

impl Mirror {
    /// Takes an update. A node that is sent, and the node with the focus, must be the root or
    /// in the children of one node held; a child must have been sent, now or before. What no
    /// node holds any more is forgotten.
    pub fn apply(&mut self, update: TreeUpdate) -> Result<(), String> {
        let sent: Vec<u64> = update.nodes.iter().map(|n| n.id).chain(update.focus).collect();
        self.nodes.extend(update.nodes.into_iter().map(|n| (n.id, n)));
        let (mut held, mut next) = (HashMap::new(), vec![update.root]);
        while let Some(id) = next.pop() {
            let Some(node) = self.nodes.remove(&id) else {
                return Err(format!("node {id:#x} was never sent, or is the child of two nodes"));
            };
            next.extend(&node.children);
            held.insert(id, node);
        }
        self.nodes = held;
        match sent.iter().find(|id| !self.nodes.contains_key(id)) {
            Some(id) => Err(format!("node {id:#x} was sent, or has the focus, and no node holds it")),
            None => Ok(()),
        }
    }

    /// The nodes it holds, in the order of their ids.
    pub fn nodes(&self) -> Vec<SemanticNode> {
        let mut nodes: Vec<SemanticNode> = self.nodes.values().cloned().collect();
        nodes.sort_by_key(|n| n.id);
        nodes
    }
}

/// The tree under `root` as indented text, one node a line: the default snapshot of tests.
pub fn snapshot(nodes: &[SemanticNode], root: u64) -> String {
    fn line(nodes: &[SemanticNode], id: u64, depth: usize, out: &mut String) {
        let Some(node) = nodes.iter().find(|n| n.id == id) else { return };
        let s = &node.semantics;
        let _ = write!(out, "{:indent$}{:?}", "", s.role, indent = depth * 2);
        if !s.name.is_empty() {
            let _ = write!(out, " {:?}", s.name);
        }
        if let Some(value) = &s.value {
            let _ = write!(out, " = {value:?}");
        }
        let flags = [
            (s.checked == Some(true), "checked"),
            (s.checked == Some(false), "unchecked"),
            (s.selected, "selected"),
            (s.expanded == Some(true), "expanded"),
            (s.expanded == Some(false), "collapsed"),
            (s.disabled, "disabled"),
            (node.focused, "focused"),
        ];
        for (_, flag) in flags.iter().filter(|(on, _)| *on) {
            let _ = write!(out, " [{flag}]");
        }
        if let Some(id) = &s.test_id {
            let _ = write!(out, " #{id}");
        }
        out.push('\n');
        for &child in &node.children {
            line(nodes, child, depth + 1, out);
        }
    }
    let mut out = String::new();
    line(nodes, root, 0, &mut out);
    out
}
