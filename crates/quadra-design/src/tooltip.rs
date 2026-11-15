//! The tooltip (specs/behaviour/15, "Tooltip"): a few words about what the pointer rests on,
//! shown after half a second and gone when the pointer leaves or presses. It never takes
//! the focus, and it adds no node around the view it is about: the layout does not know.

use quadra_headless::hint;
use quadra_tree::{Along, Anchor, NodeId, Role, Side};
use std::rc::Rc;

use quadra_view::{Div, IntoProp, MountCx, Prop, View, div, row, text};

use crate::popup::{hang, surface};
use crate::token;

fn tip_line(words: &str) -> Div {
    row().gap(16.0).padding_x(8.0).padding_y(4.0).child(text(words.to_string()))
}

fn tip_bubble(line: Div) -> Div {
    surface(1).role(Role::Tooltip).max_width(288.0).child(line)
}

/// The bubble of a tooltip with a few words: for a tooltip opened by hand, over a part of a
/// view (`quadra_headless::popup::hang_with`).
pub fn tip(words: &str) -> Div {
    tip_bubble(tip_line(words))
}

pub struct Tooltip<V> {
    tip: Prop<String>,
    keys: Option<String>,
    meta: Option<String>,
    side: Side,
    of: V,
}

/// Gives a view a tooltip: `tip` shows above it once the pointer has rested on it. It may
/// change (a closure or a signal): the bubble and what a reader is told follow.
pub fn tooltip<V: View + 'static, M>(tip: impl IntoProp<String, M>, of: V) -> Tooltip<V> {
    Tooltip { tip: tip.into_prop(), keys: None, meta: None, side: Side::Above, of }
}

impl<V> Tooltip<V> {
    /// The shortcut that does the same, muted, after the text: "Ctrl-S".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }
}

impl<V> Tooltip<V> {
    /// The side of the view the tooltip shows on: at the right of what stands in a column
    /// at the edge of the window.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// A line under the text that says more, smaller and muted.
    pub fn meta(mut self, words: impl Into<String>) -> Self {
        self.meta = Some(words.into());
        self
    }
}

impl<V: View + 'static> View for Tooltip<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let of = self.of.mount(cx);
        let shown = quadra_headless::tooltip(cx, of, hint::TOOLTIP_REST);
        // To a reader the words are the description of what they are about, there or not.
        let tip: Rc<dyn Fn() -> String> = match self.tip {
            Prop::Value(said) => {
                let told = said.clone();
                cx.tree.update_semantics(of, move |says| says.description = Some(told));
                Rc::new(move || said.clone())
            }
            Prop::Reactive(said) => {
                let (said, writer): (Rc<dyn Fn() -> String>, _) = (Rc::from(said), cx.handle.clone());
                let told = said.clone();
                cx.handle.bind(move || {
                    let words = told();
                    writer.with(move |tree| {
                        tree.update_semantics(of, move |says| says.description = Some(words))
                    });
                });
                said
            }
        };

        // The bubble is an overlay opened by hand, above the view and centred on it.
        let words = (tip, self.keys, self.meta);
        let anchor = Anchor::Beside { node: of, side: self.side, along: Along::Center, gap: 6.0 };
        hang(cx, of, shown, anchor, move || {
            let said = words.0.clone();
            let line = row().gap(16.0).padding_x(8.0).padding_y(4.0).child(text(move || said()));
            let line = match &words.1 {
                Some(keys) => line.child(text(keys.clone()).color(token::TEXT_2)),
                None => line,
            };
            let bubble = tip_bubble(line);
            match &words.2 {
                Some(meta) => {
                    let more = text(meta.clone()).size(12.0).color(token::TEXT_2);
                    bubble.child(div().padding_x(8.0).padding_bottom(4.0).child(more))
                }
                None => bubble,
            }
        });
        of
    }
}
