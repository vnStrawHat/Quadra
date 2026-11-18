//! The terminal view's focus (specs/18, "Input"): told to the program when it asked, the
//! view's and the window's keyboard while the view has it; and the cursor's blink.

use std::cell::Cell;
use std::rc::Rc;

use quadra_async::{Timers, interval};
use quadra_reactive::{Scope, use_context};
use quadra_tree::NodeId;
use quadra_view::MountCx;

use crate::session::Session;
use crate::view::CARET_BLINK;
use crate::widget::TerminalWidget;

fn tell_focus(s: &Session, gained: bool) {
    s.write(s.engine().term.encode_focus(gained).unwrap_or_default());
}

/// The focus (the view's, and the window's while the view has it): told to the program when it
/// asked; and the cursor's blink while it is there.
pub(crate) fn focus(cx: &mut MountCx, id: NodeId, session: &Session) {
    let s = session.clone();
    cx.tree.on_active(id, move |active| tell_focus(&s, active));
    let (owner, blinks) = (Scope::current(), use_context::<Rc<Timers>>().is_some());
    let blinking: Cell<Option<Scope>> = Cell::new(None);
    let (writer, s) = (cx.handle.clone(), session.clone());
    cx.tree.on_focus(id, move |gained| {
        if let Some(ended) = blinking.take() {
            ended.dispose();
        }
        if gained && blinks {
            let scope = owner.run(Scope::child);
            let writer = writer.clone();
            scope.run(|| {
                interval(CARET_BLINK, move || {
                    writer.with(move |tree| {
                        if let Some(w) = tree.widget_mut::<TerminalWidget>(id)
                            && w.cursor_style.blinking
                        {
                            w.caret_on = !w.caret_on;
                            crate::widget::repaint(tree, id);
                        }
                    })
                })
            });
            blinking.set(Some(scope));
        }
        tell_focus(&s, gained);
        writer.with(move |tree| {
            if let Some(w) = tree.widget_mut::<TerminalWidget>(id) {
                (w.focused, w.caret_on, w.preedit) = (gained, true, None);
                crate::widget::repaint(tree, id);
            }
        });
    });
}
