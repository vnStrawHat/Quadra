//! Splitting and closing spaces (specs/19, "Spaces"): a tab's spaces are the leaves of a
//! pane tree, its groups their ids; the list of spaces is kept in the tree's reading order.

use quadra_design::work::Axis;

use crate::model::{Model, Side, Space};
use crate::shells::Launch;

impl Side {
    fn axis(self) -> Axis {
        match self {
            Side::Right | Side::Left => Axis::Row,
            Side::Down | Side::Up => Axis::Column,
        }
    }

    fn before(self) -> bool {
        matches!(self, Side::Left | Side::Up)
    }
}

impl Model {
    /// A new space to the right of the focused one of the active tab, with the focus.
    pub fn split(&self) {
        self.split_to(Side::Right);
    }

    /// A new local shell in the focused space's cwd on a side of it, with the focus.
    pub fn split_to(&self, side: Side) {
        self.split_to_with(side, Launch { cwd: self.focused_cwd(), ..self.launch(None) });
    }

    /// A new space running a launch to the right of the focused one, with the focus.
    pub fn split_with(&self, launch: Launch) {
        self.split_to_with(Side::Right, launch);
    }

    /// A new space running a launch on a side of the focused one of the active tab, in half
    /// of its room (a run of splits along one axis shared evenly), with the focus.
    pub fn split_to_with(&self, side: Side, launch: Launch) {
        let Some(tab) = self.active_tab() else { return };
        let Some(focused) = quadra::untrack(|| tab.focused()) else { return };
        let space = self.space(launch);
        let (new, at) = (space.id.to_string(), tab.focus.get_untracked() + usize::from(!side.before()));
        tab.spaces.update(|spaces| spaces.insert(at.min(spaces.len()), space));
        tab.layout.update(|tree| {
            tree.split(&focused.id.to_string(), side.axis(), &new, side.before());
            tree.even(&new);
        });
        tab.focus.set(at);
    }

    /// Closes the focused space of the active tab, and the tab with its last space. False
    /// when no tab is left.
    pub fn close_focused(&self) -> bool {
        match self.active_tab().and_then(|tab| tab.focused()) {
            Some(space) => self.close_space(space.id),
            None => false,
        }
    }

    /// Closes a space, and its tab when it was the last. False when no tab is left.
    pub fn close_space(&self, id: u64) -> bool {
        let tabs = self.tabs().get_untracked();
        let found = tabs.iter().find_map(|tab| {
            tab.spaces.with_untracked(|spaces| spaces.iter().position(|it| it.id == id)).map(|at| (*tab, at))
        });
        let Some((tab, at)) = found else { return !tabs.is_empty() };
        let mut gone = None;
        tab.spaces.update(|spaces| gone = Some(spaces.remove(at)));
        // Its view goes before its scope does.
        let left = tab.spaces.with_untracked(Vec::len);
        let kept = if left == 0 {
            self.close_tab(tab.id)
        } else {
            let focus = tab.focus.get_untracked();
            let focus = if focus > at { focus - 1 } else { focus.min(left - 1) };
            let next = tab.spaces.with_untracked(|spaces| spaces[focus].id.to_string());
            tab.layout.update(|tree| {
                tree.remove(&id.to_string());
                tree.even(&next);
            });
            tab.focus.set(focus);
            true
        };
        if let Some(space) = gone {
            end(space);
        }
        kept
    }

    /// Moves the focus to the next space of the active tab, or the one before.
    pub fn focus_next(&self, forward: bool) {
        let Some(tab) = self.active_tab() else { return };
        let count = tab.spaces.with_untracked(Vec::len);
        let at = tab.focus.get_untracked();
        tab.focus.set(if forward { (at + 1).min(count - 1) } else { at.saturating_sub(1) });
    }
}

/// Ends a space: its program, then its scope (and its channel: no tab holds it).
pub(crate) fn end(space: Space) {
    space.session.close();
    space.scope.dispose();
}
