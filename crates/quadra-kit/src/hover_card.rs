//! The hover card (specs/13, "Overlays in the kit"): more about a name or a link, shown once
//! the pointer has rested on it. It stays while the pointer is on the name or on the card, so
//! what the card holds can be pressed. Like the tooltip it never takes the focus, and adds no
//! node around the view it is about.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timer, timeout};
use quadra_reactive::{Scope, signal};
use quadra_tree::{Along, Anchor, NodeId, PointerKind, Role, Side};
use quadra_ui::{MountCx, View};

use crate::popup::{hang, surface};

/// How long the pointer rests before the card shows, and how long the card stays after the
/// pointer left it and its view: the time to cross the gap between them.
const REST: Duration = Duration::from_millis(400);
const GRACE: Duration = Duration::from_millis(150);

pub struct HoverCard<V, F> {
    of: V,
    content: F,
}

/// Gives a view a card with what `content` builds, shown under it while the pointer rests
/// on it or on the card.
pub fn hover_card<V, C, F>(of: V, content: F) -> HoverCard<V, F>
where
    V: View + 'static,
    C: View + 'static,
    F: Fn() -> C + 'static,
{
    HoverCard { of, content }
}

impl<V, C, F> View for HoverCard<V, F>
where
    V: View + 'static,
    C: View + 'static,
    F: Fn() -> C + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let of = self.of.mount(cx);
        let (shown, owner) = (signal(false), Scope::current());
        let waiting: Rc<Cell<Option<Timer>>> = Rc::default();
        // One timer at a time. The pointer came: the card shows after the rest, or stays. It
        // left: the card goes after the grace, unless the pointer is back by then.
        let later = move |on: bool| {
            let timer = owner.run(|| timeout(if on { REST } else { GRACE }, move || shown.set(on)));
            if let Some(before) = waiting.replace(Some(timer)) {
                before.cancel();
            }
        };
        let (came, left) = (later.clone(), later.clone());
        cx.tree.on_pointer(of, PointerKind::Enter, move |_| came(true));
        cx.tree.on_pointer(of, PointerKind::Leave, move |_| left(false));
        let content = self.content;
        let anchor = Anchor::Beside { node: of, side: Side::Below, along: Along::Start, gap: 6.0 };
        hang(cx, of, shown, anchor, move || {
            let (came, left) = (later.clone(), later.clone());
            let card = surface(1).role(Role::Group).padding(12.0).gap(8.0).child(content());
            let card = card.on_pointer(PointerKind::Enter, move |_| came(true));
            card.on_pointer(PointerKind::Leave, move |_| left(false))
        });
        of
    }
}
