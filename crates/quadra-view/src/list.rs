//! Virtual lists (specs/05, "Virtual lists"): any number of rows, of which only those in the
//! viewport are mounted.
//!
//! The rows are children of a content node as tall as the whole list. That node is a layer cut
//! to the viewport, so scrolling within the mounted rows changes its offset and nothing else.
//! When a row leaves, its nodes are given to the row entering: the list writes the row's
//! index signal and moves it. Steady scrolling mounts nothing.

mod heights;
mod rows;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_layout::{Direction, Length, Overflow};
use quadra_prim::{Point, Rect};
use quadra_reactive::{Scope, Signal, memo, signal, untrack};
use quadra_tree::{NodeId, PointerKind, Style};

use self::heights::Heights;
use self::rows::List;
use crate::div::Div;
use crate::div::div;
use crate::flow::Region;
use crate::scroll::{reveals, wheel_scrolls};
use crate::scrollbar::{self, Scrolled};
use crate::view::{MountCx, View};
use crate::window::TextScale;

/// Rows kept mounted above and below the viewport, so a small scroll shows no gap.
const OVERSCAN: usize = 2;

/// Which row a list is asked to show, if any.
type Wanted = Box<dyn Fn() -> Option<usize>>;

/// Where a list writes how far it scrolls, and how tall its rows are.
type Told = (Option<Signal<f32>>, Option<Signal<f32>>);

/// A virtual list, as a view.
pub struct ListView {
    region: Region,
    wanted: Rc<RefCell<Option<Wanted>>>,
    /// Where the list writes how far it scrolls, and how tall its rows are.
    ends: Rc<Cell<Told>>,
}

impl ListView {
    /// Replaces the box the list fills.
    pub fn container(mut self, container: Div) -> Self {
        self.region = self.region.container(container);
        self
    }

    /// Keeps a row in view: when `row` gives another one, the list scrolls as little as shows
    /// all of it, mounted and measured or not. Scrolling away afterwards is not undone.
    pub fn reveal(self, row: impl Fn() -> Option<usize> + 'static) -> Self {
        *self.wanted.borrow_mut() = Some(Box::new(row));
        self
    }

    /// Writes into `out` how far the list scrolls at most, as its rows' heights are known.
    pub fn end(self, out: Signal<f32>) -> Self {
        self.ends.set((Some(out), self.ends.get().1));
        self
    }

    /// Writes into `out` the height of all its rows, as they are known.
    pub fn extent(self, out: Signal<f32>) -> Self {
        self.ends.set((self.ends.get().0, Some(out)));
        self
    }
}

impl View for ListView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        self.region.mount(cx)
    }
}

/// A list of `count` rows of one height. `row` builds a row from its index, a signal: a row
/// that is recycled for another index is not built again, its bindings run. `scroll_y` is how
/// far the list is scrolled, in pixels; it is kept between zero and the end.
///
/// The list fills its container (see [`Region::container`]); give that a size or let it grow.
/// The rows are `row_height` at a text scale of 1, and as many times taller as text is
/// larger (`TextScale`): what holds a line of text grows with it (`by_text`).
pub fn uniform_list<V: View + 'static>(
    count: impl Fn() -> usize + 'static,
    row_height: f32,
    scroll_y: Signal<f32>,
    row: impl Fn(Signal<usize>) -> V + 'static,
) -> ListView {
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
) -> ListView {
    list(count, Heights::measured(estimate), scroll_y, row)
}

fn list<V: View + 'static>(
    count: impl Fn() -> usize + 'static,
    heights: Heights,
    scroll_y: Signal<f32>,
    row: impl Fn(Signal<usize>) -> V + 'static,
) -> ListView {
    let wanted: Rc<RefCell<Option<Wanted>>> = Rc::default();
    let (asked, ends) = (wanted.clone(), Rc::new(Cell::new((None, None))));
    let told: Rc<Cell<Told>> = ends.clone();
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
        let (measures, base) = (heights.fixed().is_none(), heights.fixed());
        let scale = quadra_reactive::use_context::<TextScale>();
        let heights = Rc::new(RefCell::new(heights));
        // Written when a row turned out to have another height than it counted with: what
        // depends on where rows are runs again.
        let moved = signal(0u32);
        let known = heights.clone();
        let extent = memo(move || {
            moved.get();
            let mut heights = known.borrow_mut();
            if let Some(base) = base {
                heights.set_fixed(base * scale.map_or(1.0, |scale| scale.0.get()));
            }
            heights.resize(total.get());
            heights.before(usize::MAX) as f32
        });
        // Where the list really is: never past its end.
        let end = memo(move || (extent.get() - size.get().height).max(0.0));
        let top = memo(move || scroll_y.get().clamp(0.0, end.get()));
        tree.on_pointer(viewport, PointerKind::Wheel, move |e| {
            wheel_scrolls(e, scroll_y, top.get(), end.get())
        });
        tree.on_reveal(viewport, move |rect: quadra_prim::Rect| {
            let height = size.get_untracked().height;
            quadra_prim::Point::new(0.0, reveals(rect.y, rect.height, scroll_y, top.get(), end.get(), height))
        });
        let thumb = scrollbar::attach(
            cx,
            viewport,
            Scrolled { scroll: scroll_y, top, end, view: size, sideways: false },
        );
        cx.tree.set_children(viewport, &[content, thumb]);
        let (to_end, to_extent) = told.get();
        if let Some(out) = to_end {
            cx.handle.bind(move || {
                out.try_set(end.get());
            });
        }
        if let Some(out) = to_extent {
            cx.handle.bind(move || {
                out.try_set(extent.get());
            });
        }
        if let Some(wanted) = asked.borrow_mut().take() {
            // Asked again when heights are found out, until the row shows whole and measured:
            // the rows on the way to it were only estimated.
            let (known, pending, last) = (heights.clone(), Cell::new(None), Cell::new(None));
            cx.handle.bind(move || {
                let (row, _, height) = (wanted(), moved.get(), size.get().height);
                if last.replace(row) != row {
                    pending.set(row);
                }
                let Some(row) = pending.get().filter(|&row| row < untrack(|| total.get())) else { return };
                let y = untrack(|| top.get());
                let (from, to, measured) = known.borrow().span(row);
                let rect = Rect::new(0.0, from as f32 - y, 0.0, (to - from) as f32);
                if rect.y >= 0.0 && rect.bottom() <= height && measured {
                    pending.set(None);
                } else if height > 0.0 {
                    untrack(|| reveals(rect.y, rect.height, scroll_y, y, end.get(), height));
                }
            });
        }
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
        let placed = Cell::new(base.unwrap_or(0.0));
        let list = Rc::new(List { content, owner, heights, slots, row: Box::new(row), measured, placed });
        let writer = cx.handle.clone();
        cx.handle.bind(move || {
            let ((first, last), _, _) = (range.get(), moved.get(), extent.get());
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
    ListView { region: Region::new(div().grow(1.0), bind), wanted, ends }
}
