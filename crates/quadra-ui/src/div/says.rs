//! What a box is to a user (specs/12): its role, its name, its states. A state that a style
//! can follow too (checked, selected, disabled, expanded) is set once here for both.

use quadra_tree::{NodeId, RangeValue, Role, SemanticAction, Semantics, State, Tree};

use super::Div;
use crate::prop::{IntoProp, Prop};

impl Div {
    /// Something to do with a value on the node, when it is mounted and, for a reactive
    /// value, each time it changes.
    fn with_value<T: 'static>(mut self, prop: Prop<T>, apply: fn(&mut Tree, NodeId, T)) -> Self {
        match prop {
            Prop::Value(value) => self.on_mount(move |tree, id| apply(tree, id, value)),
            Prop::Reactive(read) => {
                self.binders.push(Box::new(move |id, handle| {
                    let writer = handle.clone();
                    handle.bind(move || {
                        let value = read();
                        writer.with(move |tree| apply(tree, id, value));
                    });
                }));
                self
            }
        }
    }

    /// What the box is: a button, a checkbox, a list. A box without a role means nothing by
    /// itself, and what it holds belongs to the box around it.
    pub fn role(self, role: Role) -> Self {
        self.on_mount(move |tree, id| tree.update_semantics(id, |says| says.role = role))
    }

    /// What the box is called. Buttons, links, items and the like are called by the text
    /// they hold when no name is given.
    pub fn label<M>(self, name: impl IntoProp<String, M>) -> Self {
        self.with_value(name.into_prop(), |tree, id, name| tree.update_semantics(id, |says| says.name = name))
    }

    /// A name that does not change with the language or the content, for tests and tools.
    pub fn test_id(self, id: &'static str) -> Self {
        self.on_mount(move |tree, node| {
            tree.update_semantics(node, |says| says.test_id = Some(id.to_string()))
        })
    }

    /// What the box holds, as a reader is told it: the position of a slider.
    pub fn value<M>(self, value: impl IntoProp<String, M>) -> Self {
        self.with_value(value.into_prop(), |tree, id, value| {
            tree.update_semantics(id, |says| says.value = Some(value))
        })
    }

    /// The number a slider or a progress bar holds, in its range, besides its text.
    pub fn range<M>(self, range: impl IntoProp<RangeValue, M>) -> Self {
        self.with_value(range.into_prop(), |tree, id, range| {
            tree.update_semantics(id, |says| says.range = Some(range))
        })
    }

    /// Where an item is in its set, from 0: a reader says "4 of 10".
    pub fn place_in_set<M>(self, at: impl IntoProp<usize, M>) -> Self {
        self.with_value(at.into_prop(), |tree, id, at| {
            tree.update_semantics(id, |says| says.position = Some(at))
        })
    }

    /// How many items the set of the box holds, mounted or not.
    pub fn set_size<M>(self, count: impl IntoProp<usize, M>) -> Self {
        self.with_value(count.into_prop(), |tree, id, count| {
            tree.update_semantics(id, |says| says.set_size = Some(count))
        })
    }

    /// What a reader asks of the box beyond a click and the focus: to step a value, to set
    /// it. `f` says whether it did.
    pub fn on_semantic(self, f: impl FnMut(&SemanticAction) -> bool + 'static) -> Self {
        self.on_mount(move |tree, id| tree.on_semantic(id, f))
    }

    pub fn checked<M>(self, checked: impl IntoProp<bool, M>) -> Self {
        self.with_value(checked.into_prop(), |tree, id, on| {
            set(tree, id, State::CHECKED, on, |says, on| says.checked = Some(on))
        })
    }

    pub fn selected<M>(self, selected: impl IntoProp<bool, M>) -> Self {
        self.with_value(selected.into_prop(), |tree, id, on| {
            set(tree, id, State::SELECTED, on, |says, on| says.selected = on)
        })
    }

    pub fn disabled<M>(self, disabled: impl IntoProp<bool, M>) -> Self {
        self.with_value(disabled.into_prop(), |tree, id, on| {
            set(tree, id, State::DISABLED, on, |says, on| says.disabled = on)
        })
    }

    /// What the box holds is not accepted, or is: a style can follow it (`State::INVALID`).
    pub fn invalid<M>(self, invalid: impl IntoProp<bool, M>) -> Self {
        self.with_value(invalid.into_prop(), |tree, id, on| tree.set_state(id, State::INVALID, on))
    }

    pub fn expanded<M>(self, expanded: impl IntoProp<bool, M>) -> Self {
        self.with_value(expanded.into_prop(), |tree, id, on| {
            set(tree, id, State::EXPANDED, on, |says, on| says.expanded = Some(on))
        })
    }
}

/// A state of a node, for what it says of itself and for its style at once.
fn set(tree: &mut Tree, id: NodeId, state: State, on: bool, says: fn(&mut Semantics, bool)) {
    tree.update_semantics(id, |s| says(s, on));
    tree.set_state(id, state, on);
}
