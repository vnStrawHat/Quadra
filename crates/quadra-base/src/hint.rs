//! What shows after the pointer rests (specs/17, "The components"): a tooltip and a hover
//! card. Neither takes the focus nor adds a node around the view it is about: the handlers
//! are put on the view's node, and what shows is opened by hand (`popup::hang`) while the
//! signal these give is true. The look gives the bubble and the card, and where they stand.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timer, timeout};
use quadra_reactive::{Scope, Signal, signal};
use quadra_tree::{NodeId, PointerKind};
use quadra_ui::{Div, MountCx};

/// How long the pointer rests before a tooltip shows.
pub const TOOLTIP_REST: Duration = Duration::from_millis(500);
/// How long it rests before a hover card shows, and how long the card stays after the pointer
/// left it and its view: the time to cross the gap between them.
pub const CARD_REST: Duration = Duration::from_millis(400);
pub const CARD_GRACE: Duration = Duration::from_millis(150);

/// Whether the tooltip of the node `of` shows: once the pointer has rested on it for `rest`,
/// and not after it left or pressed.
pub fn tooltip(cx: &mut MountCx, of: NodeId, rest: Duration) -> Signal<bool> {
    let (shown, owner) = (signal(false), Scope::current());
    let waiting: Rc<Cell<Option<Timer>>> = Rc::default();
    let resting = waiting.clone();
    cx.tree.on_pointer(of, PointerKind::Enter, move |_| {
        let timer = owner.run(|| timeout(rest, move || shown.set(true)));
        if let Some(before) = resting.replace(Some(timer)) {
            before.cancel();
        }
    });
    let away = move || {
        if let Some(timer) = waiting.take() {
            timer.cancel();
        }
        shown.set(false);
    };
    let leave = away.clone();
    cx.tree.on_pointer(of, PointerKind::Leave, move |_| leave());
    cx.tree.on_pointer(of, PointerKind::Down, move |_| away());
    shown
}

/// The hover card of a node: whether it shows, and what keeps it while the pointer is on it.
#[derive(Clone)]
pub struct Card {
    pub shown: Signal<bool>,
    later: Rc<dyn Fn(bool)>,
}

impl Card {
    /// The look's card, kept while the pointer is on it.
    pub fn keep(&self, card: Div) -> Div {
        let (came, left) = (self.later.clone(), self.later.clone());
        let card = card.on_pointer(PointerKind::Enter, move |_| came(true));
        card.on_pointer(PointerKind::Leave, move |_| left(false))
    }
}

/// The hover card of the node `of`: it shows once the pointer has rested on the node for
/// `rest`, and goes `grace` after the pointer left both the node and the card.
pub fn card(cx: &mut MountCx, of: NodeId, rest: Duration, grace: Duration) -> Card {
    let (shown, owner) = (signal(false), Scope::current());
    let waiting: Rc<Cell<Option<Timer>>> = Rc::default();
    // One timer at a time: the pointer came, the card shows after the rest, or stays; it left,
    // the card goes after the grace, unless the pointer is back by then.
    let later: Rc<dyn Fn(bool)> = Rc::new(move |on: bool| {
        let timer = owner.run(|| timeout(if on { rest } else { grace }, move || shown.set(on)));
        if let Some(before) = waiting.replace(Some(timer)) {
            before.cancel();
        }
    });
    let (came, left) = (later.clone(), later.clone());
    cx.tree.on_pointer(of, PointerKind::Enter, move |_| came(true));
    cx.tree.on_pointer(of, PointerKind::Leave, move |_| left(false));
    Card { shown, later }
}
