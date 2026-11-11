//! The semantic tree (specs/12): one description of the UI for assistive technology, for
//! tests and for the inspector. What a screen reader can reach, a test can reach, the same
//! way.
//!
//! This crate has no OS code: the bridge to an operating system is in its native crate.

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
    /// A name that does not change with the language or the content: the automation id for
    /// assistive technology, a selector for tests.
    pub test_id: Option<String>,
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

/// Where updates go: the OS's accessibility API, or a recorder in tests.
pub trait Bridge {
    fn update(&mut self, update: &TreeUpdate);
}

/// A bridge that keeps what it is given.
#[derive(Default)]
pub struct Recorder(pub Vec<TreeUpdate>);

impl Bridge for Recorder {
    fn update(&mut self, update: &TreeUpdate) {
        self.0.push(update.clone());
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
