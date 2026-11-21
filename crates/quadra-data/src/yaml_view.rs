//! YAML lines that fold (specs/13, "Data"): a value as `to_yaml` writes it, on the kit's
//! tree, a line a key or an item, what holds things closed to `{n}` or `[n]`.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_design::action::Copy;
use quadra_design::{TreeNode, fonts, token, tree};
use quadra_platform::Clipboard;
use quadra_reactive::{Signal, memo, signal, untrack, use_context};
use quadra_tree::NodeId;
use quadra_view::{IntoProp, MountCx, Prop, View, column};

use crate::value::{Value, step};
use crate::value_view::tone;
use crate::yaml::{scalar, to_yaml};

/// The line of a value after a label: a scalar, or what is folded.
fn line(row: TreeNode, value: &Value) -> TreeNode {
    match value {
        Value::Map(pairs) if !pairs.is_empty() => row.folded(value.summary(), token::TEXT_3),
        Value::List(items) if !items.is_empty() => row.folded(value.summary(), token::TEXT_3),
        _ => row.detail(scalar(value), tone(value)),
    }
}

/// The lines of what `value` holds, under `path`.
fn rows(value: &Value, path: &str) -> Vec<TreeNode> {
    match value {
        Value::Map(pairs) => pairs.iter().map(|(key, inside)| entry(key, inside, path)).collect(),
        Value::List(items) => {
            items.iter().enumerate().map(|(i, inside)| item(inside, &format!("{path}[{i}]"))).collect()
        }
        _ => Vec::new(),
    }
}

fn entry(key: &str, value: &Value, path: &str) -> TreeNode {
    let at = format!("{path}{}", step(key));
    let label = format!("{}:", scalar(&Value::Text(key.to_string())));
    let row = TreeNode::new(at.clone(), label).color(token::SYNTAX_PROPERTY).children(rows(value, &at));
    line(row, value)
}

/// An item of a list: `- `, and a map whose first value is a scalar on the dash's line.
fn item(value: &Value, at: &str) -> TreeNode {
    match value {
        Value::Map(pairs) if pairs.first().is_some_and(|(_, first)| first.inside().is_empty()) => {
            let (key, first) = &pairs[0];
            let label = format!("- {}:", scalar(&Value::Text(key.clone())));
            let rest = pairs[1..].iter().map(|(key, inside)| entry(key, inside, at));
            let row =
                TreeNode::new(at, label).color(token::SYNTAX_PROPERTY).detail(scalar(first), tone(first));
            let row = row.children(rest.collect::<Vec<_>>());
            if pairs.len() > 1 { row.folded(value.summary(), token::TEXT_3) } else { row }
        }
        _ => line(TreeNode::new(at, "-").color(token::TEXT_3).children(rows(value, at)), value),
    }
}

pub struct YamlView {
    value: Prop<Rc<Value>>,
    open: usize,
    folds: Option<Signal<HashSet<String>>>,
    omit: Rc<Vec<String>>,
}

/// A value as YAML lines that fold: a constant, or something reactive, in which case what
/// is open stays open while it is still there.
pub fn yaml_view<M>(value: impl IntoProp<Rc<Value>, M>) -> YamlView {
    YamlView { value: value.into_prop(), open: 1, folds: None, omit: Rc::new(Vec::new()) }
}

impl YamlView {
    /// How many levels start open: 1, the top, unless told.
    pub fn open(mut self, levels: usize) -> Self {
        self.open = levels;
        self
    }

    /// The paths that are open: the app's, to fold and unfold all.
    pub fn folds(mut self, open: Signal<HashSet<String>>) -> Self {
        self.folds = Some(open);
        self
    }

    /// Paths left out of the lines and of the copy (`metadata.managedFields`).
    pub fn omit<S: Into<String>>(mut self, paths: impl IntoIterator<Item = S>) -> Self {
        self.omit = Rc::new(paths.into_iter().map(Into::into).collect());
        self
    }
}

impl View for YamlView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let read: Rc<dyn Fn() -> Rc<Value>> = match self.value {
            Prop::Value(value) => Rc::new(move || value.clone()),
            Prop::Reactive(read) => Rc::from(read),
        };
        let omit = self.omit;
        let shown = memo(move || Rc::new(read().without(&omit)));
        let levels = self.open;
        let open = self.folds.unwrap_or_else(|| signal(HashSet::new()));
        if untrack(|| open.with(HashSet::is_empty)) {
            open.set(untrack(|| shown.get().holders(levels)));
        }
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let copy = move |_: &Copy| {
            if let Some(clipboard) = &clipboard {
                clipboard.write_text(&to_yaml(&untrack(|| shown.get())));
            }
        };
        let lines =
            tree(move || rows(&shown.get(), ""), signal(None)).open(open).row_height(24.0).indent(18.0);
        let block = column().grow(1.0).font_family(fonts::MONO).text_size(15.0).bg(token::CANVAS).radius(6.0);
        block.on_action::<Copy>(copy).child(lines).mount(cx)
    }
}
