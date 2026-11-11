//! The tooltip (specs/behaviour/15, "Tooltip"): a few words about what the pointer rests on,
//! shown after half a second and gone when the pointer leaves or presses. It never takes
//! the focus, and it adds no node around the view it is about: the layout does not know.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timer, timeout};
use quadra_reactive::{Scope, on_cleanup, signal};
use quadra_tree::{Along, Anchor, NodeId, PointerEvent, PointerKind, Role, Side};
use quadra_ui::{MountCx, View, div, row, text};

use crate::popup::surface;
use crate::token;

/// How long the pointer rests before the tooltip shows.
const REST: Duration = Duration::from_millis(500);

pub struct Tooltip<V> {
    tip: String,
    keys: Option<String>,
    meta: Option<String>,
    of: V,
}

/// Gives a view a tooltip: `tip` shows above it once the pointer has rested on it.
pub fn tooltip<V: View + 'static>(tip: impl Into<String>, of: V) -> Tooltip<V> {
    Tooltip { tip: tip.into(), keys: None, meta: None, of }
}

impl<V> Tooltip<V> {
    /// The shortcut that does the same, muted, after the text: "Ctrl-S".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }
}

impl<V> Tooltip<V> {
    /// A line under the text that says more, smaller and muted.
    pub fn meta(mut self, words: impl Into<String>) -> Self {
        self.meta = Some(words.into());
        self
    }
}

impl<V: View + 'static> View for Tooltip<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let of = self.of.mount(cx);
        let (shown, owner) = (signal(false), Scope::current());
        let waiting: Rc<Cell<Option<Timer>>> = Rc::default();
        // The pointer came: the tooltip shows if it is still there in half a second.
        let rest = waiting.clone();
        cx.tree.on_pointer(of, PointerKind::Enter, move |_: &mut PointerEvent| {
            let timer = owner.run(|| timeout(REST, move || shown.set(true)));
            if let Some(before) = rest.replace(Some(timer)) {
                before.cancel();
            }
        });
        // It left, or pressed: no tooltip, and none on its way.
        let away = move || {
            if let Some(timer) = waiting.take() {
                timer.cancel();
            }
            shown.set(false);
        };
        let leave = away.clone();
        cx.tree.on_pointer(of, PointerKind::Leave, move |_| leave());
        cx.tree.on_pointer(of, PointerKind::Down, move |_| away());

        // The bubble is an overlay opened by hand, above the view and centred on it.
        let (words, open) = (Rc::new((self.tip, self.keys, self.meta)), Rc::new(Cell::new(None)));
        let (writer, last) = (cx.handle.clone(), open.clone());
        cx.handle.bind(move || {
            let (on, words, open) = (shown.get(), words.clone(), open.clone());
            writer.with(move |tree| {
                if let Some(old) = open.take() {
                    tree.close_overlay(old);
                }
                if !on || !tree.is_alive(of) || !owner.is_alive() {
                    return;
                }
                let scope = owner.run(Scope::child);
                let line = row().gap(16.0).padding_x(8.0).padding_y(4.0).child(text(words.0.clone()));
                let line = match &words.1 {
                    Some(keys) => line.child(text(keys.clone()).color(token::TEXT_2)),
                    None => line,
                };
                let bubble = surface(1).role(Role::Tooltip).max_width(288.0).child(line);
                let bubble = match &words.2 {
                    Some(meta) => {
                        let more = text(meta.clone()).size(12.0).color(token::TEXT_2);
                        bubble.child(div().padding_x(8.0).padding_bottom(4.0).child(more))
                    }
                    None => bubble,
                };
                let root = scope.run(|| bubble.mount(&mut MountCx::new(tree)));
                tree.set_scope(root, scope);
                let anchor = Anchor::Beside { node: of, side: Side::Above, along: Along::Center, gap: 6.0 };
                tree.open_overlay(root, anchor);
                open.set(Some(root));
            });
        });
        let handle = cx.handle.clone();
        on_cleanup(move || {
            handle.with(move |tree| {
                if let Some(root) = last.take() {
                    tree.close_overlay(root);
                }
            })
        });
        of
    }
}
