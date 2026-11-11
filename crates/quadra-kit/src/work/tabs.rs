//! The tab strip (specs/behaviour/16, "Tab strip"): the open views of a pane, one of them
//! showing. A press chooses a tab, a drag moves it among the others, a cross or the middle
//! button closes it, and a list at the end of the strip names them all.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use quadra_layout::{Align, Display, Justify};
use quadra_platform::{Cursor, MouseButton};
use quadra_prim::Size;
use quadra_reactive::{Signal, memo, on_cleanup, signal};
use quadra_tree::{NodeId, Paint, PointerEvent, PointerKind, Role};
use quadra_ui::{AnyView, Div, MountCx, View, div, each, row, scroll_strip, text};

use crate::icon::icon;
use crate::menu::{MenuItem, menu_icon_button};
use crate::{icons, size, token};

/// A view open in a pane: what it is known by, its title, its icon, and whether it holds
/// changes that are not saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub icon: Option<&'static str>,
    pub dirty: bool,
}

impl Tab {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Tab {
        Tab { id: id.into(), title: title.into(), icon: None, dirty: false }
    }

    pub fn icon(mut self, body: &'static str) -> Tab {
        self.icon = Some(body);
        self
    }

    pub fn dirty(mut self, dirty: bool) -> Tab {
        self.dirty = dirty;
        self
    }
}

type Close = Rc<dyn Fn(&str)>;
/// How wide each mounted tab is, by its id: where a dragged tab is among the others.
type Widths = Rc<RefCell<HashMap<String, Signal<Size>>>>;

pub struct TabStrip {
    tabs: Signal<Vec<Tab>>,
    active: Signal<String>,
    close: Option<Close>,
    actions: Vec<AnyView>,
}

/// A strip of 32 px with a tab for each of `tabs`, the one whose id `active` holds joined to
/// what is under the strip. A press writes `active`, a drag the order of `tabs`.
pub fn tab_strip(tabs: Signal<Vec<Tab>>, active: Signal<String>) -> TabStrip {
    TabStrip { tabs, active, close: None, actions: Vec::new() }
}

impl TabStrip {
    /// Tabs can be closed, by the cross that shows under the pointer or a click with the
    /// middle button: `f` is told which tab, and takes it out of the tabs if it agrees.
    pub fn on_close(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.close = Some(Rc::new(f));
        self
    }

    /// Something at the end of the strip, before the list of the tabs: split, more.
    pub fn action(mut self, view: impl View + 'static) -> Self {
        self.actions.push(AnyView::new(view));
        self
    }
}

/// A tab, and its width told to the strip while it is mounted.
struct Measured {
    face: Div,
    id: String,
    widths: Widths,
}

impl View for Measured {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let node = self.face.mount(cx);
        self.widths.borrow_mut().insert(self.id.clone(), cx.tree.size_signal(node));
        let (widths, id) = (self.widths, self.id);
        on_cleanup(move || _ = widths.borrow_mut().remove(&id));
        node
    }
}

/// The index of the tab the pointer is over, `x` from the start of the tab at `from`.
fn slot(tabs: &[Tab], widths: &Widths, from: usize, x: f32) -> usize {
    let widths = widths.borrow();
    let wide = |tab: &Tab| widths.get(&tab.id).map_or(0.0, |size| size.get_untracked().width);
    let at = tabs[..from].iter().map(wide).sum::<f32>() + x;
    let mut end = 0.0;
    let over = |tab: &Tab| {
        end += wide(tab);
        at < end
    };
    tabs.iter().position(over).unwrap_or(tabs.len() - 1)
}

/// One tab: 32 px high on the plane of tabs, or on the canvas with no line under it while
/// it is the active one; the seams are on the side away from the active tab.
fn face(strip: &Rc<TabStrip>, widths: &Widths, tab: Signal<Tab>) -> Measured {
    let (tabs, active, id) = (strip.tabs, strip.active, tab.with_untracked(|tab| tab.id.clone()));
    let (me, mine) = (id.clone(), id.clone());
    let on = memo(move || active.with(|active| *active == me));
    // Where the tab and the active one are in the strip.
    let place = memo(move || {
        let (i, at) = (tabs.with(|all| all.iter().position(|tab| tab.id == mine)), active.get());
        (i.unwrap_or(0), tabs.with(|all| all.iter().position(|tab| tab.id == at)))
    });
    let dot = div().size(6.0, 6.0).radius(3.0).bg(token::ACCENT);
    let dot = dot.display(move || if tab.with(|tab| tab.dirty) { Display::Flex } else { Display::None });
    let mark = row().size(12.0, 12.0).align_items(Align::Center).justify(Justify::Center).child(dot);
    let content = row().align_items(Align::Center).gap(4.0).padding_x(4.0).grow(1.0).child(mark);
    let content = match tab.with_untracked(|tab| tab.icon) {
        Some(body) => content.child(icon(body).color(token::TEXT_2)),
        None => content,
    };
    let tone = move || -> Paint { if on.get() { token::TAB_ACTIVE_FG } else { token::TAB_FG }.into() };
    let title = div().text_color(tone).child(text(move || tab.with(|tab| tab.title.clone())));
    let content = content.child(title);
    let content = match &strip.close {
        // The cross has no colour of its own: it takes that of the tab's text, which is the
        // tab's own fill until the pointer is over the tab.
        Some(close) => {
            let (close, id) = (close.clone(), id.clone());
            let cross =
                row().size(18.0, 18.0).shrink(0.0).align_items(Align::Center).justify(Justify::Center);
            let cross = cross.radius(size::RADIUS_MD).hover(|s| s.bg(token::PANEL_HOVER));
            let cross = cross.role(Role::Button).label("Close").child(icon(icons::X));
            // A press on the cross neither chooses the tab nor starts a drag.
            let cross = cross.on_pointer(PointerKind::Down, |e| e.stop());
            let cross = cross.on_click(move |e| {
                e.stop();
                close(&id);
            });
            let room = row().size(14.0, 14.0).align_items(Align::Center).justify(Justify::Center);
            content.child(room.child(cross))
        }
        None => content.padding_right(8.0),
    };
    let fill = move || -> Paint { if on.get() { token::CANVAS } else { token::TAB_BG }.into() };
    let face = row().shrink(0.0).cursor(Cursor::Pointer).border_color(token::LINE_STRONG);
    let face = face.bg(fill).text_color(fill).hover(|s| s.text_color(token::TEXT));
    let face =
        face.role(Role::Tab).label(move || tab.with(|tab| tab.title.clone())).selected(move || on.get());
    let face = face.border_bottom(move || if on.get() { 0.0 } else { 1.0 });
    let seam = |there: bool| if there { 1.0 } else { 0.0 };
    let face = face.border_left(move || {
        let (i, at) = place.get();
        seam(i > 0 && at.is_none_or(|at| i <= at))
    });
    let face = face.border_right(move || {
        let (i, at) = place.get();
        seam(at.is_some_and(|at| i >= at))
    });

    // A press chooses the tab and takes the pointer; moved 4 px, the tab goes where the
    // pointer is among the others.
    let held: Rc<Cell<Option<(f32, bool)>>> = Rc::default();
    let (press, drag, widths, close) = (held.clone(), held.clone(), widths.clone(), strip.close.clone());
    let (chosen, pressed, moved) = (id.clone(), id.clone(), id.clone());
    let face = face.on_click(move |_| active.set(chosen.clone()));
    let face = face.on_pointer(PointerKind::Down, move |e: &mut PointerEvent| match (e.button, &close) {
        (MouseButton::Left, _) => {
            press.set(Some((e.at.x, false)));
            e.capture_pointer();
            active.set(pressed.clone());
        }
        (MouseButton::Middle, Some(close)) => close(&pressed),
        _ => {}
    });
    let known = widths.clone();
    let face = face.on_pointer(PointerKind::Move, move |e| {
        let Some((from, far)) = drag.get() else { return };
        if !far && (e.at.x - from).abs() < 4.0 {
            return;
        }
        drag.set(Some((from, true)));
        let order = tabs.get_untracked();
        let Some(i) = order.iter().position(|tab| tab.id == moved) else { return };
        let to = slot(&order, &known, i, e.local.x);
        if to != i {
            tabs.update(|all| {
                let tab = all.remove(i);
                all.insert(to, tab);
            });
        }
    });
    let face = face.on_pointer(PointerKind::Up, move |_| held.set(None));
    Measured { face: face.child(content), id, widths }
}

impl View for TabStrip {
    fn mount(mut self, cx: &mut MountCx) -> NodeId {
        let (tabs, active, actions) = (self.tabs, self.active, std::mem::take(&mut self.actions));
        let (strip, widths) = (Rc::new(self), Widths::default());
        let faces = each(move || tabs.get(), |tab| tab.id.clone(), move |tab| face(&strip, &widths, tab));
        // The line of the bar runs under the tabs that are not active, the room that is left
        // and what stands at the end.
        let rest = div().grow(1.0).border_bottom(1.0).border_color(token::LINE_STRONG);
        let all = row().role(Role::TabList).child(faces.container(row().shrink(0.0))).child(rest);
        // Every tab by name, the active one checked.
        let list = move || {
            let named = |tab: &Tab| {
                let id = tab.id.clone();
                let item = MenuItem::new(tab.title.clone()).checked(active.with_untracked(|on| *on == id));
                let item = item.on_select(move || active.set(id.clone()));
                tab.icon.map_or(item.clone(), |body| item.icon(body))
            };
            tabs.with_untracked(|all| all.iter().map(named).collect())
        };
        let end = row().align_items(Align::Center).gap(4.0).padding_x(6.0).shrink(0.0);
        let end = end.border_bottom(1.0).border_left(1.0).border_color(token::LINE_STRONG);
        let end = end.children(actions).child(menu_icon_button(icons::ELLIPSIS, "Tabs", list));
        let bar = row().height(32.0).shrink(0.0).bg(token::TAB_BG);
        bar.child(scroll_strip(signal(0.0), all)).child(end).mount(cx)
    }
}
