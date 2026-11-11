//! Virtual lists (specs/05, "Virtual lists"): any number of rows, of which only those in the
//! viewport are mounted.
//!
//! The rows are children of a content node as tall as the whole list. That node is a layer cut
//! to the viewport, so scrolling within the mounted rows changes its offset and nothing else.
//! When a row leaves, its nodes are given to the row entering: the list writes the row's
//! index signal and moves it. Steady scrolling mounts nothing.

mod heights;

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use quadra_layout::{Direction, Insets, Length, Overflow, Position};
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, memo, signal, untrack};
use quadra_tree::{NodeId, PointerKind, Style, Tree};

use self::heights::Heights;
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

/// What the bindings of a list share.
struct List<V> {
    content: NodeId,
    owner: Scope,
    heights: Rc<RefCell<Heights>>,
    slots: RefCell<Vec<Slot>>,
    row: Box<dyn Fn(Signal<usize>) -> V>,
    /// Told the height a row was laid out with, in a list that measures its rows.
    measured: Option<Rc<dyn Fn(usize, f32)>>,
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
    list(count, Heights::Uniform { height: row_height, count: 0 }, scroll_y, row)
}

/// A list of `count` rows, each as tall as what it holds. A row is measured when it is
/// mounted; one that never was counts with `estimate`. When a row above the viewport turns
/// out taller or shorter than it counted, `scroll_y` moves by the difference: what shows
/// stays where it is. The rest is as for [`uniform_list`], which costs less where it fits.
pub fn virtual_list<V: View + 'static>(
    count: impl Fn() -> usize + 'static,
    estimate: f32,
    scroll_y: Signal<f32>,
    row: impl Fn(Signal<usize>) -> V + 'static,
) -> Region {
    list(count, Heights::measured(estimate), scroll_y, row)
}

fn list<V: View + 'static>(
    count: impl Fn() -> usize + 'static,
    heights: Heights,
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
        let measures = heights.fixed().is_none();
        let heights = Rc::new(RefCell::new(heights));
        // Written when a row turned out to have another height than it counted with: what
        // depends on where rows are runs again.
        let moved = signal(0u32);
        let known = heights.clone();
        let extent = memo(move || {
            moved.get();
            let mut heights = known.borrow_mut();
            heights.resize(total.get());
            heights.before(usize::MAX) as f32
        });
        // Where the list really is: never past its end.
        let end = memo(move || (extent.get() - size.get().height).max(0.0));
        let top = memo(move || scroll_y.get().clamp(0.0, end.get()));
        tree.on_pointer(viewport, PointerKind::Wheel, move |e| {
            wheel_scrolls(e, scroll_y, top.get(), end.get())
        });
        let known = heights.clone();
        let range = memo(move || {
            let (y, n) = (top.get() as f64, total.get()); // `top` has made the heights `n` long
            let (bottom, _) = (y + size.get().height as f64, moved.get());
            let heights = known.borrow();
            let first = heights.row_at(y).saturating_sub(OVERSCAN).min(n);
            (first, (heights.rows_before(bottom) + OVERSCAN).min(n))
        });

        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let y = top.get();
            writer.with(move |tree| tree.set_layer_offset(content, Point::new(0.0, -y)));
        });
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let height = Length::Px(extent.get());
            writer.with(move |tree| {
                if tree.is_alive(content) {
                    let style = quadra_layout::Style { height, ..tree.layout_style(content).clone() };
                    tree.set_layout_style(content, style);
                }
            });
        });
        let known = heights.clone();
        let record = move |row: usize, height: f32| {
            let y = untrack(|| top.get()) as f64; // asked first: it may make the heights longer
            let starts_above = known.borrow().before(row) < y;
            let taller = known.borrow_mut().set(row, height);
            if taller != 0.0 {
                if starts_above {
                    scroll_y.update(|y| *y += taller); // what shows below it stays where it is
                }
                moved.update(|n| *n += 1);
            }
        };
        let measured = measures.then(|| Rc::new(record) as Rc<dyn Fn(usize, f32)>);
        let slots = RefCell::new(Vec::new());
        let list = Rc::new(List { content, owner, heights, slots, row: Box::new(row), measured });
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let ((first, last), _) = (range.get(), moved.get());
            let list = list.clone();
            writer.with(move |tree| {
                if list.realize(tree, first..last) && list.measured.is_some() {
                    // A box given to another row is as tall as that row only after the
                    // layout that is now due, and its size may not even change then.
                    tree.after_layout(move |tree| list.measure(tree));
                }
            });
        });
    };
    Region::new(div().grow(1.0), bind)
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
    fn measure(&self, tree: &mut Tree) {
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
    fn realize(&self, tree: &mut Tree, range: Range<usize>) -> bool {
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
