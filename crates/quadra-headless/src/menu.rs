//! The menu (specs/17, "The components"): rows that do something. The arrows move a highlight
//! over the rows that can be chosen, round the ends, and bring it into view; Enter or Space
//! does what the highlighted row does, and doing it ends the menu. A row may open a menu of
//! its own beside itself: under the pointer, by a click, by the right arrow; the left arrow
//! gives it up, and a choice in it ends the menus together. The look gives each row's face,
//! the parts that are no rows (a line, a heading), the box of the rows, and where a row's own
//! menu stands.

use std::rc::Rc;

use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, signal};
use quadra_tree::{NodeId, PointerKind, Role, Tree};
use quadra_ui::{AnyView, Div, MountCx, NodeRef, Region};

use crate::action::{Confirm, MENU, MoveInto, MoveNext, MoveOut, MovePrevious};
use crate::popup::Shown;

/// What ends a menu: `done` after a row was chosen, and `leave` for the menu of a row, which
/// the left arrow gives up.
#[derive(Clone)]
pub struct Ends {
    pub done: Rc<dyn Fn()>,
    pub leave: Option<Rc<dyn Fn()>>,
}

/// The popup of a row's own menu, which the look places: given the row, when it shows, and
/// what ends the menu in it.
pub type Inner = Rc<dyn Fn(&NodeRef, Shown, Ends) -> Region>;

/// A row of a menu: its face, what a reader calls it, and what it does.
pub struct Row {
    pub face: Div,
    /// The box around the face, if the look has one: room at its sides.
    pub line: Option<Div>,
    pub label: String,
    pub disabled: bool,
    /// A row that is on or off: checked to a reader.
    pub checked: Option<bool>,
    pub select: Option<Rc<dyn Fn()>>,
    pub inner: Option<Inner>,
}

impl Row {
    pub fn new(face: Div, label: impl Into<String>) -> Row {
        Row {
            face,
            line: None,
            label: label.into(),
            disabled: false,
            checked: None,
            select: None,
            inner: None,
        }
    }
}

/// A line of a menu: a row, or a part that is none (a line between groups, a heading).
pub enum Line {
    Row(Box<Row>),
    Part(Box<Div>),
}

/// The lines of a menu in `list`, its box: a menu to a reader, which takes the focus.
pub fn rows(list: Div, lines: Vec<Line>, ends: Ends) -> Div {
    // The row the keyboard is on, if any (the pointer has its own mark, the hover), and the
    // row whose menu is open beside it.
    let (on, beside): (Signal<Option<usize>>, Signal<Option<usize>>) = (signal(None), signal(None));
    let count = lines.len();
    let refs: Rc<Vec<NodeRef>> = Rc::new(lines.iter().map(|_| NodeRef::new()).collect());
    let (mut chosen, mut inner, mut does) = (vec![false; count], vec![false; count], Vec::new());
    let mut parts: Vec<AnyView> = Vec::new();
    for (i, line) in lines.into_iter().enumerate() {
        let row = match line {
            Line::Part(part) => {
                parts.push(AnyView::new(*part));
                does.push(None);
                continue;
            }
            Line::Row(row) => *row,
        };
        (chosen[i], inner[i]) = (!row.disabled, row.inner.is_some());
        does.push(row.select.clone());
        let face = row.face.role(Role::MenuItem).label(row.label).disabled(row.disabled);
        let face = match row.checked {
            Some(checked) => face.checked(checked),
            None => face,
        };
        let face = face.selected(move || on.get() == Some(i)).node_ref(&refs[i]);
        let opens = row.inner.is_some();
        // The pointer over a row opens its menu, and closes that of any other row.
        let face = face.on_pointer(PointerKind::Enter, move |_| beside.set(opens.then_some(i)));
        let (select, done) = (row.select, ends.done.clone());
        let face = face.on_click(move |_| {
            if opens {
                return beside.set(Some(i));
            }
            if let Some(select) = &select {
                select();
            }
            done();
        });
        let line = match row.line {
            Some(line) => line.child(face),
            None => face,
        };
        let line = match row.inner {
            None => line,
            Some(inner) => {
                let shown = Shown {
                    is: Rc::new(move || beside.get() == Some(i)),
                    close: Rc::new(move || beside.update(|open| *open = open.filter(|open| *open != i))),
                };
                let leave: Rc<dyn Fn()> = Rc::new(move || beside.set(None));
                let ends = Ends { done: ends.done.clone(), leave: Some(leave) };
                line.child(inner(&refs[i], shown, ends))
            }
        };
        parts.push(AnyView::new(line));
    }
    let (chosen, inner, does) = (Rc::new(chosen), Rc::new(inner), Rc::new(does));
    // The arrows go to the next row that can be chosen, round the ends, and bring it into view.
    let step = {
        let (chosen, refs) = (chosen.clone(), refs.clone());
        move |forward: bool| {
            let from = on.get_untracked().unwrap_or(if forward { count.saturating_sub(1) } else { 0 });
            let order =
                (1..=count).map(|k| if forward { (from + k) % count } else { (from + count - k) % count });
            on.set({ order }.find(|&i| chosen[i]));
            if let Some(at) = on.get_untracked() {
                refs[at].scroll_into_view();
            }
        }
    };
    let (forward, back, opens) = (step.clone(), step, inner.clone());
    let confirm = move || {
        let Some(at) = on.get_untracked() else { return };
        if inner[at] {
            return beside.set(Some(at));
        }
        if let Some(select) = &does[at] {
            select();
        }
        (ends.done)();
    };
    let list = list.role(Role::Menu).focusable().key_context(MENU);
    let list =
        list.on_action::<MoveNext>(move |_| forward(true)).on_action::<MovePrevious>(move |_| back(false));
    let list = list.on_action::<Confirm>(move |_| confirm());
    // The right arrow opens the menu of the row it is on; the left one gives this menu up.
    let list = list.on_action::<MoveInto>(move |_| {
        if let Some(at) = on.get_untracked().filter(|&at| opens[at]) {
            beside.set(Some(at));
        }
    });
    let leave = ends.leave;
    let list = list.on_action::<MoveOut>(move |_| {
        if let Some(leave) = &leave {
            leave();
        }
    });
    list.children(parts)
}

/// Runs `open` when the view `of` is clicked with the other button: a menu at the pointer,
/// opened with `popup::open_now`, in the scope `of` was mounted in. No node is added.
pub fn on_context(cx: &mut MountCx, of: NodeId, open: impl Fn(&mut Tree, Scope, Point) + 'static) {
    let (owner, handle, open) = (Scope::current(), cx.handle.clone(), Rc::new(open));
    let (keys, by_key) = (cx.handle.clone(), open.clone());
    cx.tree.on_pointer(of, PointerKind::Click, move |e| {
        if e.button != MouseButton::Right {
            return;
        }
        let (at, open) = (e.at, open.clone());
        handle.with(move |tree| open(tree, owner, at));
        e.stop();
    });
    // The menu key and Shift-F10 open it under what has the focus inside `of`.
    cx.tree.on_key(of, true, move |e| {
        if !(e.key == Key::Menu || e.key == Key::F10 && e.mods == Mods::SHIFT) {
            return;
        }
        let (target, open) = (e.target, by_key.clone());
        keys.with(move |tree| {
            let rect = tree.window_rect(target);
            open(tree, owner, Point::new(rect.x, rect.y + rect.height))
        });
        e.stop();
    });
}
