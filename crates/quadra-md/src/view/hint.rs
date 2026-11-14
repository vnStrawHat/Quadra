//! The hint of where a link of a document goes: its target under it, once the pointer has
//! rested on it, Ctrl held or not.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_async::{Timer, timeout};
use quadra_base::hint::TOOLTIP_REST;
use quadra_base::popup::hang_with;
use quadra_prim::Point;
use quadra_reactive::{Scope, effect, signal};
use quadra_tree::{Anchor, NodeId};
use quadra_ui::{MountCx, RichTextView, View};

pub(super) struct Hinted(pub RichTextView);

impl View for Hinted {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (over, shown) = (signal(None::<(Point, Rc<str>)>), signal(false));
        let id = self.0.on_link_hover(move |link| over.set(link)).mount(cx);
        // Shown after a rest on one link; another link, or none, starts again.
        let (rest, owner): (Rc<RefCell<Option<Timer>>>, _) = (Rc::default(), Scope::current());
        effect(move || {
            let on = over.with(Option::is_some);
            shown.set(false);
            if let Some(timer) = rest.borrow_mut().take() {
                timer.cancel();
            }
            *rest.borrow_mut() = on.then(|| owner.run(|| timeout(TOOLTIP_REST, move || shown.set(true))));
        });
        let under = move || {
            over.with(|link| link.as_ref().map_or(Point::ZERO, |(at, _)| Point::new(at.x, at.y + 4.0)))
        };
        let target =
            move || over.with(|link| link.as_ref().map(|(_, to)| to.to_string()).unwrap_or_default());
        hang_with(cx, id, shown, move || Anchor::At(under()), move || quadra_kit::tip(&target()));
        id
    }
}
