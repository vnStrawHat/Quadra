//! The hover card (specs/13, "Overlays in the kit"): more about a name or a link, shown once
//! the pointer has rested on it. It stays while the pointer is on the name or on the card, so
//! what the card holds can be pressed. Like the tooltip it never takes the focus, and adds no
//! node around the view it is about.

use quadra_headless::hint;
use quadra_tree::{Along, Anchor, NodeId, Role, Side};
use quadra_ui::{MountCx, View};

use crate::popup::{hang, surface};

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
        let card = hint::card(cx, of, hint::CARD_REST, hint::CARD_GRACE);
        let (content, shown) = (self.content, card.shown);
        let anchor = Anchor::Beside { node: of, side: Side::Below, along: Along::Start, gap: 6.0 };
        hang(cx, of, shown, anchor, move || {
            card.keep(surface(1).role(Role::Group).padding(12.0).gap(8.0).child(content()))
        });
        of
    }
}
