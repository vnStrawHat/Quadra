//! The menu (specs/behaviour/15, "Menu"): rows that do something, in a popup. The arrows move
//! a highlight, Enter does what the highlighted row does, and doing it closes the menu.

mod rows;

use std::rc::Rc;

use quadra_platform::MouseButton;
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal};
use quadra_tree::{NodeId, PointerKind};
use quadra_ui::{MountCx, NodeRef, Place, View};

pub(crate) use self::rows::{Ends, rows};
use crate::popup::{open_now, popover};

/// A row of a menu, or the line between two groups of rows.
#[derive(Clone, Default)]
pub struct MenuItem {
    label: String,
    icon: Option<&'static str>,
    keys: Option<String>,
    checked: Option<bool>,
    disabled: bool,
    separator: bool,
    heading: bool,
    /// The rows of the menu the row opens beside itself.
    items: Vec<MenuItem>,
    select: Option<Rc<dyn Fn()>>,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> MenuItem {
        MenuItem { label: label.into(), ..MenuItem::default() }
    }

    /// A hairline between two groups of rows.
    pub fn separator() -> MenuItem {
        MenuItem { separator: true, ..MenuItem::default() }
    }

    /// The name of the group of rows under it: muted, and not chosen.
    pub fn heading(label: impl Into<String>) -> MenuItem {
        MenuItem { label: label.into(), heading: true, ..MenuItem::default() }
    }

    /// Makes the row one that opens a menu of these rows beside itself: under the pointer,
    /// by a click, or by the right arrow.
    pub fn submenu(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    /// An icon before the label: one of [`crate::icons`].
    pub fn icon(mut self, body: &'static str) -> Self {
        self.icon = Some(body);
        self
    }

    /// The shortcut that does the same, as text after the label: "Ctrl-C".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    /// A row that is on or off: it shows a check while on.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// What choosing the row does, before the menu closes.
    pub fn on_select(mut self, f: impl Fn() + 'static) -> Self {
        self.select = Some(Rc::new(f));
        self
    }

    fn chosen(&self) -> bool {
        !self.separator && !self.heading && !self.disabled
    }
}

/// A menu below `opener` while `open` is true. `items` is asked each time the menu opens,
/// so a row can say what is so then (a check, a disabled row).
pub fn menu<F>(open: Signal<bool>, opener: &NodeRef, items: F) -> impl View + use<F>
where
    F: Fn() -> Vec<MenuItem> + 'static,
{
    let ends = move || Ends { done: Rc::new(move || open.set(false)), leave: None };
    popover(open, opener, move || rows(items(), ends()))
}

pub struct ContextMenu<V, F> {
    of: V,
    items: F,
}

/// Gives a view a menu that opens at the pointer on a click with the other button. `items`
/// is asked each time it opens. No node is added around the view.
pub fn context_menu<V, F>(of: V, items: F) -> ContextMenu<V, F>
where
    V: View + 'static,
    F: Fn() -> Vec<MenuItem> + 'static,
{
    ContextMenu { of, items }
}

impl<V, F> View for ContextMenu<V, F>
where
    V: View + 'static,
    F: Fn() -> Vec<MenuItem> + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        // ponytail: the pointer opens it; the menu key and Shift-F10 when keys do too.
        let (of, owner, handle, items) =
            (self.of.mount(cx), Scope::current(), cx.handle.clone(), Rc::new(self.items));
        cx.tree.on_pointer(of, PointerKind::Click, move |e| {
            if e.button != MouseButton::Right {
                return;
            }
            // A pixel off the pointer, which is then over none of its rows.
            let (place, items) = (Place::At(Point::new(e.at.x + 1.0, e.at.y + 1.0)), items.clone());
            handle.with(move |tree| {
                open_now(tree, owner, place, move |close| rows(items(), Ends { done: close, leave: None }))
            });
            e.stop();
        });
        of
    }
}
