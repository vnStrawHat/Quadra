//! The value viewer (specs/13, "Data"): a value as the kit's tree, in the colours of code,
//! under a bar that says the path of the selected row and copies it.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_design::action::Copy;
use quadra_design::{Size, TreeNode, button, fonts, token, tree};
use quadra_layout::Align;
use quadra_platform::Clipboard;
use quadra_reactive::{memo, signal, untrack, use_context};
use quadra_tree::{NodeId, Paint};
use quadra_ui::{IntoProp, MountCx, Prop, View, by_text, column, div, row, text};

use crate::value::Value;

/// The colour of a value on its row: that of its kind in code, and the faint text for what
/// holds things.
fn tone(value: &Value) -> Paint {
    match value {
        Value::Text(_) => token::SYNTAX_STRING.into(),
        Value::Number(_) => token::SYNTAX_NUMBER.into(),
        Value::Null | Value::Bool(_) => token::SYNTAX_CONSTANT.into(),
        Value::List(_) | Value::Map(_) => token::TEXT_3.into(),
    }
}

/// The rows of what a value holds: each is known by its path. The ids of what holds things
/// down to `open` levels are noted in `opened`.
fn rows(value: &Value, path: &str, open: usize, opened: &mut HashSet<String>) -> Vec<TreeNode> {
    let row = |(label, step, inside): (String, String, &Value)| {
        let path = format!("{path}{step}");
        let children = rows(inside, &path, open.saturating_sub(1), opened);
        if open > 0 && !children.is_empty() {
            opened.insert(path.clone());
        }
        // A key is a property of code; a place in a list is not.
        let key: Paint =
            if label.ends_with(':') { token::SYNTAX_PROPERTY.into() } else { token::TEXT_3.into() };
        TreeNode::new(path, label).color(key).detail(inside.summary(), tone(inside)).children(children)
    };
    value.inside().into_iter().map(row).collect()
}

pub struct ValueView {
    value: Prop<Rc<Value>>,
}

/// A value as rows that open and close: a constant, or something reactive, in which case
/// what is open stays open while it is still there.
pub fn value_view<M>(value: impl IntoProp<Rc<Value>, M>) -> ValueView {
    ValueView { value: value.into_prop() }
}

impl View for ValueView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let read: Rc<dyn Fn() -> Rc<Value>> = match self.value {
            Prop::Value(value) => Rc::new(move || value.clone()),
            Prop::Reactive(read) => Rc::from(read),
        };
        // The first two levels start open.
        let mut opened = HashSet::new();
        untrack(|| rows(&read(), "", 2, &mut opened));
        let (chosen, open) = (signal(None::<String>), signal(opened));
        let all = memo(move || Rc::new(rows(&read(), "", 0, &mut HashSet::new())));
        let clipboard = use_context::<Rc<dyn Clipboard>>();
        let copy = move || {
            if let (Some(path), Some(clipboard)) = (chosen.get_untracked(), &clipboard) {
                clipboard.write_text(&path);
            }
        };

        let path_text = text(move || chosen.get().unwrap_or_else(|| "Select a value".to_string()));
        let path = div().grow(1.0).text_size(12.0).font_family(fonts::MONO);
        let path = path.child(path_text.color(token::TEXT_2).selectable());
        let button = button("Copy path").size(Size::Sm).disabled(move || chosen.with(Option::is_none));
        let bar = row().align_items(Align::Center).gap(6.0).padding_x(8.0).height(by_text(28.0)).shrink(0.0);
        let bar =
            bar.border_bottom(1.0).border_color(token::LINE).child(path).child(button.on_click(copy.clone()));
        let rows = tree(move || (*all.get()).clone(), chosen).open(open).row_height(24.0).indent(16.0);
        let rows = column().grow(1.0).font_family(fonts::MONO).child(rows);
        let view = column().grow(1.0).on_action::<Copy>(move |_| copy());
        view.child(bar).child(rows).mount(cx)
    }
}
