//! The mounted rows of a list: the boxes it places, given from the rows that leave to the
//! rows that enter, and measured where the list does not know their height.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use quadra_layout::{Insets, Length, Position};
use quadra_reactive::{Scope, Signal, signal};
use quadra_tree::{NodeId, Style, Tree};

use super::heights::Heights;
use crate::view::{MountCx, View};

pub(super) struct Slot {
    /// The box the list places; the row's own view is its child.
    wrapper: NodeId,
    index: Signal<usize>,
    at: usize,
}

/// What the bindings of a list share.
pub(super) struct List<V> {
    pub content: NodeId,
    pub owner: Scope,
    pub heights: Rc<RefCell<Heights>>,
    pub slots: RefCell<Vec<Slot>>,
    pub row: Box<dyn Fn(Signal<usize>) -> V>,
    /// Told the height a row was laid out with, in a list that measures its rows.
    pub measured: Option<Rc<dyn Fn(usize, f32)>>,
}

impl<V: View> List<V> {
    /// Where the box of a row goes: across the list, at the row's place, as tall as the list
    /// says or as what it holds.
    fn place(&self, row: usize) -> quadra_layout::Style {
        let (heights, zero) = (self.heights.borrow(), Length::Px(0.0));
        quadra_layout::Style {
            position: Position::Absolute,
            inset: Insets {
                top: Length::Px(heights.before(row) as f32),
                left: zero,
                right: zero,
                ..Insets::default()
            },
            height: heights.fixed().map_or(Length::Auto, Length::Px),
            ..Default::default()
        }
    }

    /// Tells the list how tall each mounted row was laid out.
    pub fn measure(&self, tree: &mut Tree) {
        let Some(measured) = &self.measured else { return };
        let rows: Vec<(usize, NodeId)> = self.slots.borrow().iter().map(|s| (s.at, s.wrapper)).collect();
        for (row, wrapper) in rows.into_iter().filter(|&(_, wrapper)| tree.is_alive(wrapper)) {
            // ponytail: a row with no height counts with the estimate.
            let height = tree.rect(wrapper).height;
            if height > 0.0 {
                measured(row, height);
            }
        }
    }

    /// Makes the mounted rows be exactly `range`: rows outside it are given to the indexes
    /// that have none, more are mounted if there are not enough, and what is left over is
    /// removed. Returns whether a box got another row.
    pub fn realize(&self, tree: &mut Tree, range: Range<usize>) -> bool {
        if !tree.is_alive(self.content) || !self.owner.is_alive() {
            return false;
        }
        let slots = &mut *self.slots.borrow_mut();
        if self.measured.is_some() {
            // A row that turned out taller moved the ones after it.
            for slot in slots.iter().filter(|slot| range.contains(&slot.at)) {
                tree.set_layout_style(slot.wrapper, self.place(slot.at));
            }
        }
        let mut spare: Vec<usize> = (0..slots.len()).filter(|&i| !range.contains(&slots[i].at)).collect();
        let mut changed = !spare.is_empty();
        for index in range.clone() {
            if slots.iter().any(|slot| slot.at == index) {
                continue;
            }
            changed = true;
            if let Some(i) = spare.pop() {
                slots[i].at = index;
                slots[i].index.set(index);
                tree.set_layout_style(slots[i].wrapper, self.place(index));
                continue;
            }
            // A row's signal and bindings live in a scope that ends with its nodes.
            let scope = self.owner.run(Scope::child);
            let (signal, wrapper) = scope.run(|| {
                let signal = signal(index);
                let mut cx = MountCx::new(tree);
                let child = (self.row)(signal).mount(&mut cx);
                let wrapper = cx.tree.insert(Style { layout: self.place(index), ..Style::default() });
                cx.tree.set_children(wrapper, &[child]);
                if let Some(measured) = self.measured.clone() {
                    // A row that is laid out taller or shorter later on, by what it holds or
                    // by the list's width: its size is written after that layout, when the
                    // box still holds the row it was laid out for.
                    let size = cx.tree.size_signal(wrapper);
                    cx.handle.bind(move || {
                        let height = size.get().height;
                        if height > 0.0 {
                            measured(signal.get_untracked(), height);
                        }
                    });
                }
                (signal, wrapper)
            });
            tree.set_scope(wrapper, scope);
            slots.push(Slot { wrapper, index: signal, at: index });
        }
        // Largest first: removing by swap then never moves a slot that is still to be removed.
        spare.sort_unstable_by(|a, b| b.cmp(a));
        for i in spare {
            tree.remove(slots.swap_remove(i).wrapper);
        }
        if changed {
            let wrappers: Vec<NodeId> = slots.iter().map(|slot| slot.wrapper).collect();
            tree.set_children(self.content, &wrappers);
        }
        changed
    }
}
