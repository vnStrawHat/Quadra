//! The tab strip (specs/behaviour/16, "Tab strip"): the open views of a pane, one of them
//! showing. A press chooses a tab, a drag moves it among the others, the middle button or the
//! look's cross closes it; docked, a tab dragged up or down is torn off and dropped elsewhere.
//! The look gives a tab's face, its cross, the box the tabs stand in, the bar and the chip
//! that follows the pointer.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use quadra_platform::MouseButton;
use quadra_prim::Size;
use quadra_reactive::{Signal, on_cleanup};
use quadra_tree::{NodeId, PointerEvent, PointerKind, Role};
use quadra_ui::{Div, MountCx, View};

use super::dock::{Dock, TabDrag, take_tabs};

/// How far a pressed tab moves before it goes where the pointer is among the others, and how
/// far up or down before a docked one is torn off.
pub const SLIDE: f32 = 4.0;
pub const TEAR: f32 = 12.0;

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
type Chip = Rc<dyn Fn(&Tab) -> Div>;
/// How wide each mounted tab is, by its id: where a dragged tab is among the others.
pub(crate) type Widths = Rc<RefCell<HashMap<String, Signal<Size>>>>;

/// The behaviour of a strip of `tabs`, the one whose id `active` holds showing. Copies are
/// the same one.
#[derive(Clone)]
pub struct Strip {
    pub tabs: Signal<Vec<Tab>>,
    pub active: Signal<String>,
    pub close: Option<Close>,
    dock: Option<(Dock, String, Chip)>,
    widths: Widths,
    lane: Rc<Cell<Option<NodeId>>>,
}

/// A press writes `active`, a drag the order of `tabs`.
pub fn strip(tabs: Signal<Vec<Tab>>, active: Signal<String>) -> Strip {
    Strip { tabs, active, close: None, dock: None, widths: Widths::default(), lane: Rc::default() }
}

/// A tab, and its width told to the strip while it is mounted.
pub struct Measured {
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

impl Strip {
    /// Tabs can be closed: `f` is told which tab, and takes it out of the tabs if it agrees.
    pub fn on_close(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.close = Some(Rc::new(f));
        self
    }

    /// The strip of a group of a docking workbench: a tab is torn off it and dropped on
    /// another strip or on a pane, and the strip takes the tabs dropped on it. `chip` is what
    /// follows the pointer.
    pub fn dock(
        mut self,
        dock: Dock,
        group: impl Into<String>,
        chip: impl Fn(&Tab) -> Div + 'static,
    ) -> Self {
        self.dock = Some((dock, group.into(), Rc::new(chip)));
        self
    }

    /// The cross of a tab on its face: a press on it neither chooses the tab nor starts a
    /// drag, a click closes the tab.
    pub fn closes(&self, face: Div, id: &str) -> Div {
        let Some(close) = self.close.clone() else { return face };
        let id = id.to_string();
        face.on_pointer(PointerKind::Down, |e| e.stop()).on_click(move |e| {
            e.stop();
            close(&id);
        })
    }

    /// A tab on its face: a tab to a reader, selected while it shows. A click chooses it; a
    /// press takes the pointer, and moved, the tab goes where the pointer is among the others.
    pub fn tab(&self, face: Div, tab: Signal<Tab>) -> Measured {
        let (tabs, active, id) = (self.tabs, self.active, tab.with_untracked(|tab| tab.id.clone()));
        let me = id.clone();
        let face = face.role(Role::Tab).label(move || tab.with(|tab| tab.title.clone()));
        let face = face.selected(move || active.with(|active| *active == me));
        let held: Rc<Cell<Option<(f32, bool)>>> = Rc::default();
        let (press, drag, widths, close) =
            (held.clone(), held.clone(), self.widths.clone(), self.close.clone());
        let (torn, docked) = (held.clone(), self.dock.is_some());
        let (chosen, pressed, moved) = (id.clone(), id.clone(), id.clone());
        let face = face.on_click(move |_| active.set(chosen.clone()));
        let face = face.on_pointer(PointerKind::Down, move |e: &mut PointerEvent| match (e.button, &close) {
            (MouseButton::Left, _) => {
                press.set(Some((e.at.x, false)));
                e.capture_pointer();
                // A tab that can be torn off may be dropped elsewhere: it is chosen by a click.
                if !docked {
                    active.set(pressed.clone());
                }
            }
            (MouseButton::Middle, Some(close)) => close(&pressed),
            _ => {}
        });
        let known = widths.clone();
        let face = face.on_pointer(PointerKind::Move, move |e| {
            let Some((from, far)) = drag.get() else { return };
            if !far && (e.at.x - from).abs() < SLIDE {
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
        let face = match &self.dock {
            Some((_, group, chip)) => {
                let (group, dragged, chip) = (group.clone(), id.clone(), chip.clone());
                // Torn off, the tab is the drag's: the strip moves it no more.
                let carried = move || {
                    torn.set(None);
                    TabDrag { group: group.clone(), tab: dragged.clone() }
                };
                face.draggable_off(TEAR, carried, move || chip(&tab.get_untracked()))
            }
            None => face,
        };
        Measured { face, id, widths }
    }

    /// The box the tabs stand in: where a dropped tab goes is found from its start.
    pub fn lane(&self, lane: Div) -> Div {
        let found = self.lane.clone();
        lane.on_mount(move |_, id| found.set(Some(id)))
    }

    /// The mounted bar of a docked strip takes the tabs dropped on it, where they are dropped.
    pub fn mounted(&self, cx: &mut MountCx, bar: NodeId) {
        if let Some((dock, group, _)) = &self.dock {
            take_tabs(cx, bar, (*dock, group.clone()), self.tabs, (self.widths.clone(), self.lane.clone()));
        }
    }
}
