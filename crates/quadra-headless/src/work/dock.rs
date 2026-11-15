//! Docking (specs/behaviour/16): a tab torn off its strip is dropped on another strip, on the
//! middle of a pane, or on an edge of one, which splits the pane there. `Dock` holds the
//! groups of tabs of a workbench and its pane tree, and does what a drop asks.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use quadra_prim::{Point, Rect};
use quadra_reactive::{Scope, Signal, signal};
use quadra_tree::NodeId;
use quadra_view::MountCx;

use super::pane::{Axis, Pane};
use super::tabs::{Tab, Widths};

/// A tab that is dragged: the group it is dragged out of, and its id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabDrag {
    pub group: String,
    pub tab: String,
}

/// The part of a pane a tab is dropped on: its middle, or one of its edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

/// Which part of a box a point is over: the outer quarter on each side is an edge, the rest
/// the middle.
pub fn zone_at(rect: Rect, at: Point) -> Zone {
    let (fx, fy) = ((at.x - rect.x) / rect.width.max(1.0), (at.y - rect.y) / rect.height.max(1.0));
    let edge = fx.min(1.0 - fx).min(fy).min(1.0 - fy);
    match edge {
        _ if edge >= 0.25 => Zone::Center,
        _ if edge == fx => Zone::Left,
        _ if edge == 1.0 - fx => Zone::Right,
        _ if edge == fy => Zone::Top,
        _ => Zone::Bottom,
    }
}

/// The part of a box that a drop on a zone would take: all of it, or the half at that edge.
pub fn zone_rect(rect: Rect, zone: Zone) -> Rect {
    let (w, h) = (rect.width / 2.0, rect.height / 2.0);
    match zone {
        Zone::Center => rect,
        Zone::Left => Rect::new(rect.x, rect.y, w, rect.height),
        Zone::Right => Rect::new(rect.x + w, rect.y, w, rect.height),
        Zone::Top => Rect::new(rect.x, rect.y, rect.width, h),
        Zone::Bottom => Rect::new(rect.x, rect.y + h, rect.width, h),
    }
}

impl Pane {
    /// Puts a new group beside `target`, on the side a drop on a zone of it asks for.
    pub fn dock_split(&mut self, target: &str, zone: Zone, new_group: &str) {
        match zone {
            Zone::Center => {}
            Zone::Left => self.split(target, Axis::Row, new_group, true),
            Zone::Right => self.split(target, Axis::Row, new_group, false),
            Zone::Top => self.split(target, Axis::Column, new_group, true),
            Zone::Bottom => self.split(target, Axis::Column, new_group, false),
        }
    }
}

/// A tab moved: out of the group `from` into the group `to` (at an index, or at its end), or
/// into a new group split off a side of `to`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockEvent {
    pub tab: String,
    pub from: String,
    pub to: String,
    pub zone: Zone,
    pub index: Option<usize>,
}

/// The tabs of a group, and the id of the one that shows.
#[derive(Clone, Copy)]
pub struct Group {
    pub tabs: Signal<Vec<Tab>>,
    pub active: Signal<String>,
}

/// A docking workbench: a pane tree, and the tabs of the group at each of its leaves. Copies
/// are the same one.
#[derive(Clone, Copy)]
pub struct Dock {
    pub root: Signal<Pane>,
    groups: Signal<HashMap<String, Group>>,
    /// Numbers the groups that splits make.
    made: Signal<u32>,
    owner: Scope,
}

/// A workbench with these groups at the leaves of `root`: the first tab of each shows.
pub fn dock<'a>(root: Pane, groups: impl IntoIterator<Item = (&'a str, Vec<Tab>)>) -> Dock {
    let dock =
        Dock { root: signal(root), groups: signal(HashMap::new()), made: signal(0), owner: Scope::current() };
    for (name, tabs) in groups {
        let first = tabs.first().map(|tab| tab.id.clone()).unwrap_or_default();
        let group = Group { tabs: signal(tabs), active: signal(first) };
        dock.groups.update(|all| _ = all.insert(name.to_string(), group));
    }
    dock
}

impl Dock {
    /// The tabs of a group, if there is such a group.
    pub fn group(&self, name: &str) -> Option<Group> {
        self.groups.with_untracked(|all| all.get(name).copied())
    }

    fn make(&self, name: &str) -> Group {
        let group = self.owner.run(|| Group { tabs: signal(Vec::new()), active: signal(String::new()) });
        self.groups.update(|all| _ = all.insert(name.to_string(), group));
        group
    }

    /// Does what a drop asks: the tab leaves its group for another, or for a new one split
    /// off a side of that one. A group left with no tab goes, and its pane with it.
    pub fn apply(&self, e: &DockEvent) {
        let Some(from) = self.group(&e.from) else { return };
        let Some(i) = from.tabs.with_untracked(|all| all.iter().position(|tab| tab.id == e.tab)) else {
            return;
        };
        let alone = from.tabs.with_untracked(Vec::len) == 1;
        // Into the middle of its own pane, or off it when it is all the pane has: nothing.
        if e.from == e.to && (e.zone == Zone::Center || alone) {
            return;
        }
        let mut tab = None;
        from.tabs.update(|all| tab = Some(all.remove(i)));
        let Some(tab) = tab else { return };
        // The tab after it shows in its place, or the one before the last.
        if let Some(next) =
            from.tabs.with_untracked(|all| all.get(i).or(all.last()).map(|tab| tab.id.clone()))
            && from.active.with_untracked(|active| *active == e.tab)
        {
            from.active.set(next);
        }
        let to = match e.zone {
            Zone::Center => self.group(&e.to).unwrap_or_else(|| self.make(&e.to)),
            zone => {
                self.made.update(|n| *n += 1);
                let name = format!("group-{}", self.made.get_untracked());
                let group = self.make(&name);
                self.root.update(|tree| tree.dock_split(&e.to, zone, &name));
                group
            }
        };
        to.active.set(tab.id.clone());
        to.tabs.update(|all| all.insert(e.index.filter(|&at| at <= all.len()).unwrap_or(all.len()), tab));
        if alone {
            self.root.update(|tree| tree.remove(&e.from));
            self.groups.update(|all| _ = all.remove(&e.from));
        }
    }
}

/// The index a tab dropped at `x` from the start of the tabs goes to: before the first tab
/// whose middle is past it.
fn before(tabs: &[Tab], widths: &Widths, x: f32) -> usize {
    let (widths, mut end) = (widths.borrow(), 0.0);
    let past = |tab: &Tab| {
        let wide = widths.get(&tab.id).map_or(0.0, |size| size.get_untracked().width);
        end += wide;
        x < end - wide / 2.0
    };
    tabs.iter().position(past).unwrap_or(tabs.len())
}

/// Makes the strip of a group take the tabs dropped on it, where they are dropped: one of
/// its own is moved, one of another group comes over. `lane` is the box the tabs stand in.
pub(crate) fn take_tabs(
    cx: &mut MountCx,
    bar: NodeId,
    (dock, group): (Dock, String),
    tabs: Signal<Vec<Tab>>,
    (known, lane): (Widths, Rc<Cell<Option<NodeId>>>),
) {
    let handle = cx.handle.clone();
    cx.tree.on_drop::<TabDrag>(bar, move |drag: &TabDrag, at| {
        let (drag, group, known, lane) = (drag.clone(), group.clone(), known.clone(), lane.clone());
        handle.with(move |tree| {
            let Some(lane) = lane.get().filter(|&lane| tree.is_alive(lane)) else { return };
            let order = tabs.get_untracked();
            let index = before(&order, &known, at.x - tree.window_rect(lane).x);
            if drag.group != group {
                let (tab, from) = (drag.tab, drag.group);
                return dock.apply(&DockEvent {
                    tab,
                    from,
                    to: group,
                    zone: Zone::Center,
                    index: Some(index),
                });
            }
            let Some(from) = order.iter().position(|tab| tab.id == drag.tab) else { return };
            let to = if from < index { index - 1 } else { index };
            if to != from {
                tabs.update(|all| {
                    let tab = all.remove(from);
                    all.insert(to, tab);
                });
            }
        });
    });
}
