//! Virtual lists (specs/05, "Virtual lists"): any number of rows, of which only those in the
//! viewport are mounted.
//!
//! The rows are children of a content node as tall as the whole list. That node is a layer cut
//! to the viewport, so scrolling within the mounted rows changes its offset and nothing else.
//! When a row leaves, its nodes are given to the row entering: the list writes the row's
//! index signal and moves it. Steady scrolling mounts nothing.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use quadra_layout::{Direction, Insets, Length, Overflow, Position};
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, memo, signal};
use quadra_tree::{NodeId, PointerKind, Style, Tree};

use crate::div::div;
use crate::flow::Region;
use crate::scroll::wheel_scrolls;
use crate::view::{MountCx, View};

/// Rows kept mounted above and below the viewport, so a small scroll shows no gap.
const OVERSCAN: usize = 2;

struct Slot {
    /// The box the list places; the row's own view is its child.
    wrapper: NodeId,
    index: Signal<usize>,
    at: usize,
}

fn place(row_height: f32, index: usize) -> quadra_layout::Style {
    let zero = Length::Px(0.0);
    quadra_layout::Style {
        position: Position::Absolute,
        inset: Insets {
            top: Length::Px(index as f32 * row_height),
            left: zero,
            right: zero,
            ..Insets::default()
        },
        height: Length::Px(row_height),
        ..Default::default()
    }
}

/// A list of `count` rows of one height. `row` builds a row from its index, a signal: a row
/// that is recycled for another index is not built again, its bindings run. `scroll_y` is how
/// far the list is scrolled, in pixels; it is kept between zero and the end.
///
/// The list fills its container (see [`Region::container`]); give that a size or let it grow.
pub fn uniform_list<V: View + 'static>(
    count: impl Fn() -> usize + 'static,
    row_height: f32,
    scroll_y: Signal<f32>,
    row: impl Fn(Signal<usize>) -> V + 'static,
) -> Region {
    let bind = move |viewport: NodeId, cx: &mut MountCx| {
        let owner = Scope::current();
        let tree = &mut *cx.tree;
        let scrolls = quadra_layout::Style {
            direction: Direction::Column,
            overflow_y: Overflow::Scroll,
            ..tree.layout_style(viewport).clone()
        };
        tree.set_layout_style(viewport, scrolls);
        let content = tree.insert(Style::default());
        tree.make_layer(content, true);
        tree.set_children(viewport, &[content]);

        let size = tree.size_signal(viewport);
        let total = memo(count);
        // Where the list really is: never past its end.
        let end = memo(move || (total.get() as f32 * row_height - size.get().height).max(0.0));
        let top = memo(move || scroll_y.get().clamp(0.0, end.get()));
        tree.on_pointer(viewport, PointerKind::Wheel, move |e| {
            wheel_scrolls(e, scroll_y, top.get(), end.get())
        });
        let range = memo(move || {
            let (n, y) = (total.get(), top.get());
            let first = ((y / row_height) as usize).saturating_sub(OVERSCAN).min(n);
            let last = (((y + size.get().height) / row_height).ceil() as usize + OVERSCAN).min(n);
            (first, last)
        });

        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let y = top.get();
            writer.with(move |tree| tree.set_layer_offset(content, Point::new(0.0, -y)));
        });
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let height = Length::Px(total.get() as f32 * row_height);
            writer.with(move |tree| {
                if tree.is_alive(content) {
                    let style = quadra_layout::Style { height, ..tree.layout_style(content).clone() };
                    tree.set_layout_style(content, style);
                }
            });
        });
        let slots: Rc<RefCell<Vec<Slot>>> = Rc::default();
        let row = Rc::new(row);
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let (first, last) = range.get();
            let (slots, row) = (slots.clone(), row.clone());
            writer.with(move |tree| {
                realize(tree, content, owner, &mut slots.borrow_mut(), first..last, row_height, &*row)
            });
        });
    };
    Region::new(div().grow(1.0), bind)
}

/// Makes the mounted rows be exactly `range`: rows outside it are given to the indexes that
/// have none, more are mounted if there are not enough, and what is left over is removed.
fn realize<V: View>(
    tree: &mut Tree,
    content: NodeId,
    owner: Scope,
    slots: &mut Vec<Slot>,
    range: Range<usize>,
    row_height: f32,
    row: &dyn Fn(Signal<usize>) -> V,
) {
    if !tree.is_alive(content) || !owner.is_alive() {
        return;
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
            tree.set_layout_style(slots[i].wrapper, place(row_height, index));
            continue;
        }
        // A row's signal and bindings live in a scope that ends with its nodes.
        let scope = owner.run(Scope::child);
        let (signal, wrapper) = scope.run(|| {
            let signal = signal(index);
            let child = row(signal).mount(&mut MountCx::new(tree));
            let layout = place(row_height, index);
            let wrapper = tree.insert(Style { layout, ..Style::default() });
            tree.set_children(wrapper, &[child]);
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
        tree.set_children(content, &wrappers);
    }
}
