//! The menu (specs/behaviour/15, "Menu"): rows that do something, in a popup. The arrows move
//! a highlight, Enter does what the highlighted row does, and doing it closes the menu.

mod rows;
mod search;

use std::rc::Rc;

use quadra_layout::Align;
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_tree::NodeId;
use quadra_view::{AnyView, Div, MountCx, NodeRef, Place, View, by_text, div, row};

pub(crate) use self::rows::rows;
pub use self::search::{menu_beside, searched_icon_button, searched_menu};
use crate::button::{Button, Size, Variant, button, icon_button};
use crate::popup::{below, listing, open_now, popover, popup};
use crate::token;
pub(crate) use quadra_headless::menu::Ends;

/// A row of a menu, or the line between two groups of rows.
#[derive(Clone, Default)]
pub struct MenuItem {
    label: String,
    icon: Option<&'static str>,
    /// A dot of this colour in the place of an icon: a host that is connected.
    dot: Option<quadra_tree::Token>,
    /// A square of this colour in the place of an icon: a host's colour tag.
    swatch: Option<quadra_tree::Paint>,
    /// A few words after the label, in the second text colour.
    detail: Option<String>,
    keys: Option<String>,
    /// The keys' text is in the mono face: an address, a number.
    mono: bool,
    checked: Option<bool>,
    disabled: bool,
    danger: bool,
    /// Its check is in the second text colour: the chosen option of a select.
    quiet: bool,
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

    /// A dot of a colour in the place of an icon: a state, as a host that is connected.
    pub fn dot(mut self, color: quadra_tree::Token) -> Self {
        self.dot = Some(color);
        self
    }

    /// A square of 10 px of a colour or a role in the place of an icon: a tag the user gave
    /// the row.
    pub fn swatch(mut self, color: impl Into<quadra_tree::Paint>) -> Self {
        self.swatch = Some(color.into());
        self
    }

    /// The text at the right end in the mono face: an address, a size.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// A few words after the label in the second text colour: what the row is ("zsh").
    pub fn detail(mut self, words: impl Into<String>) -> Self {
        self.detail = Some(words.into());
        self
    }

    /// The shortcut that does the same, as text after the label: "Ctrl-C".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    /// A row that is on or off: it shows a check while on, in place of its icon.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// A row that destroys something: in the colour of errors.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// What choosing the row does, before the menu closes.
    pub fn on_select(mut self, f: impl Fn() + 'static) -> Self {
        self.select = Some(Rc::new(f));
        self
    }

    pub(crate) fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }
}

/// A menu below `opener` while `open` is true. `items` is asked each time the menu opens,
/// so a row can say what is so then (a check, a disabled row).
pub fn menu<F>(open: Signal<bool>, opener: &NodeRef, items: F) -> impl View + use<F>
where
    F: Fn() -> Vec<MenuItem> + 'static,
{
    let ends = move || Ends { done: Rc::new(move || open.set(false)), leave: None };
    popup(open, listing(below(opener)), Some(opener.clone()), move || rows(items(), ends()))
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
        let (of, items) = (self.of.mount(cx), Rc::new(self.items));
        quadra_headless::menu::on_context(cx, of, move |tree, owner, at| {
            // Under the pointer by the gap of a popup: the pointer is then over none of its rows.
            // No items, no menu: the app's setting may turn it off.
            let (place, items) = (Place::At(Point::new(at.x, at.y + 4.0)), items());
            if !items.is_empty() {
                open_now(tree, owner, place, move |close| rows(items, Ends { done: close, leave: None }))
            }
        });
        of
    }
}

type Opened = Rc<dyn Fn(Signal<bool>, &NodeRef) -> AnyView>;

/// A button that opens something under itself: a menu, a popover.
pub struct MenuButton {
    face: Button,
    under: Opened,
}

/// A ghost button that opens a menu under itself. `items` is asked each time it opens.
pub fn menu_button(label: impl Into<String>, items: impl Fn() -> Vec<MenuItem> + 'static) -> MenuButton {
    let items = Rc::new(items);
    let under = move |open: Signal<bool>, opener: &NodeRef| {
        let items = items.clone();
        AnyView::new(menu(open, opener, move || items()))
    };
    MenuButton { face: button(label.into()).variant(Variant::Ghost), under: Rc::new(under) }
}

/// A ghost button that is only an icon, and opens a menu under itself. `name` is what a
/// reader calls it.
pub fn menu_icon_button(
    body: &'static str,
    name: &'static str,
    items: impl Fn() -> Vec<MenuItem> + 'static,
) -> MenuButton {
    MenuButton { face: icon_button(body, name), ..menu_button("", items) }
}

/// A button that opens a popover under itself, with what `content` builds each time, 12 px
/// in from its edge.
pub fn popover_button<V: View + 'static>(
    label: impl Into<String>,
    content: impl Fn() -> V + 'static,
) -> MenuButton {
    let content = Rc::new(content);
    let under = move |open: Signal<bool>, opener: &NodeRef| {
        let content = content.clone();
        AnyView::new(popover(open, opener, move || div().padding(12.0).child(content())))
    };
    MenuButton { face: button(label.into()), under: Rc::new(under) }
}

impl MenuButton {
    pub fn size(mut self, size: Size) -> Self {
        self.face = self.face.size(size);
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.face = self.face.variant(variant);
        self
    }

    /// An icon before the label.
    pub fn icon(mut self, body: &'static str) -> Self {
        self.face = self.face.icon(body);
        self
    }

    /// In the second text colour, as the icon buttons of a bar around it.
    pub fn muted(mut self) -> Self {
        self.face = self.face.muted();
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.face = self.face.test_id(id);
        self
    }
}

impl View for MenuButton {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (open, opener) = (signal(false), NodeRef::new());
        let face = self.face.on_click(move || open.update(|open| *open = !*open));
        let under = (self.under)(open, &opener);
        div().shrink(0.0).child(div().node_ref(&opener).child(face)).child(under).mount(cx)
    }
}

/// The bar of an app's menus: 32 px on the chrome plane over a strong line.
pub fn menubar(menus: impl IntoIterator<Item = MenuButton>) -> Div {
    let bar = row().align_items(Align::Center).gap(4.0).padding_x(4.0).height(by_text(32.0)).shrink(0.0);
    let bar = bar.bg(token::CHROME).border_bottom(1.0).border_color(token::LINE_STRONG);
    bar.children(menus.into_iter().map(|menu| menu.size(Size::Sm)).collect::<Vec<_>>())
}
