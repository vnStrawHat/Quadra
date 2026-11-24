//! The history under the hand (specs/18, "Input"): the bar down the terminal's right edge and
//! the keys that go through it a page at a time, or to an end.

use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Tree};
use quadra_view::MountCx;

use crate::Session;

/// Where a key takes the view in the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scroll {
    PageUp,
    PageDown,
    Top,
    Bottom,
}

/// What the bar shows, in pixels: how far the view is from the top of the history, and how
/// far the history goes above the screen.
pub(crate) type Told = (Signal<f32>, Signal<f32>);

impl Session {
    /// Tells the bar where the view is: after what scrolled it and as the pointer moves over
    /// the terminal. Output tells it nothing, so a burst costs the bar nothing; and never as
    /// the widget paints.
    pub(crate) fn tell_bar(&self) {
        let (Some((top, end)), line) = (self.0.bar.get(), self.0.line_px.get()) else { return };
        let (history, back) = {
            let engine = self.engine();
            let screen = engine.term.screen();
            (screen.history_len(), screen.scroll_offset())
        };
        end.try_set(history as f32 * line);
        top.try_set(history.saturating_sub(back) as f32 * line);
    }
}

/// Gives the terminal its bar: the kit's thumb, which a drag moves through the history. The
/// widget is a leaf of the layout, so a box of its place holds it and the thumb over it: the
/// node the view is.
pub(crate) fn bar(cx: &mut MountCx, id: NodeId, session: &Session) -> NodeId {
    let told: Told = (signal(0.0), signal(0.0));
    session.0.bar.set(Some(told));
    let layout = cx.tree.layout_style(id).clone();
    let around = cx.tree.insert(quadra_tree::Style { layout, ..Default::default() });
    cx.tree.set_children(around, &[id]);
    quadra_view::bar_by_hand(cx, around, told.0, told.1);
    let (writer, s) = (cx.handle.clone(), session.clone());
    cx.handle.bind(move || {
        let (y, s) = (told.0.get(), s.clone());
        writer.with(move |tree| {
            let line = s.0.line_px.get();
            let (history, back) = {
                let engine = s.engine();
                (engine.term.screen().history_len(), engine.term.screen().scroll_offset())
            };
            // The row the thumb is at; what the view told of itself is where it is already.
            let want = history.saturating_sub((y / line.max(1.0)).round() as u32);
            if line > 0.0 && want != back {
                s.engine().term.scroll_viewport(back as i32 - want as i32);
                crate::widget::repaint(tree, id);
            }
        });
    });
    around
}

/// A page is the rows that show less one, so a line stays to read on from.
pub(crate) fn scroll(tree: &mut Tree, id: NodeId, s: &Session, to: Scroll) {
    let page = (i32::from(s.engine().term.screen().rows()) - 1).max(1);
    let rows = match to {
        Scroll::PageUp => -page,
        Scroll::PageDown => page,
        Scroll::Top => -i32::MAX,
        Scroll::Bottom => i32::MAX,
    };
    scroll_rows(tree, id, s, rows);
}

/// `rows` towards the bottom (negative: back), as far as the history goes.
pub(crate) fn scroll_rows(tree: &mut Tree, id: NodeId, s: &Session, rows: i32) {
    s.engine().term.scroll_viewport(rows);
    crate::widget::repaint(tree, id);
    s.tell_bar();
}
