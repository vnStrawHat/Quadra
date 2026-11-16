//! Roots and the incremental pass (specs/06, "What a change costs"): a changed node is asked
//! again what it had answered, and layout climbs only until an ancestor answers as before.

use quadra_prim::{Rect, Size};

use crate::cache::{Answers, Request};
use crate::solve::Measure;
use crate::style::finite;
use crate::tree::{LayoutId, LayoutTree, NOT_PENDING};

/// How a root is sized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RootSize {
    /// Takes its style's size, or fills the given size where that is auto: a window.
    Fill(Size),
    /// Sizes to its content, limited by the given size: popovers, menus, tooltips.
    Fit(Size),
    /// A given width and the height of its content: rows of variable height.
    Width(f32),
    /// An exact size.
    Fixed(Size),
}

/// What the layout calls did since the last [`LayoutTree::take_stats`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Requests a node had to work out (not served from its cache).
    pub asked: u32,
    /// Calls to the measurer.
    pub measured: u32,
    /// Ancestors laid out again because a node below them answered differently.
    pub climbed: u32,
}

impl LayoutTree {
    /// Lays out the tree under `root`. Later calls do only what changed: a node whose request
    /// and subtree are unchanged answers from its cache. Returns the root's size.
    pub fn layout(&mut self, root: LayoutId, size: RootSize, m: &mut dyn Measure) -> Size {
        self.update(m);
        let node = self.node(root);
        if node.root != Some(size) || !node.answers.done {
            self.nodes[root.index()].root = Some(size);
            self.run_root(root, size, m);
        }
        self.nodes[root.index()].rect.size()
    }

    /// Brings every root that was laid out before up to date with the changes made since.
    pub fn update(&mut self, m: &mut dyn Measure) {
        if self.saved.is_empty() {
            return;
        }
        // Deepest first: a node below is settled before its ancestors are asked.
        let mut order: Vec<(usize, usize)> = (0..self.saved.len())
            .filter_map(|i| self.saved[i].as_ref().map(|s| (self.depth_of(s.id), i)))
            .collect();
        order.sort_unstable_by(|a, b| b.cmp(a));
        let mut work = 0;
        for (_, i) in order {
            // Taken by an earlier climb: that climb asked it again with these answers.
            let Some(saved) = self.saved[i].take() else {
                continue;
            };
            self.nodes[saved.id.index()].pending = NOT_PENDING;
            work += self.climb(saved.id, saved.answers, saved.item_changed, m);
            if work > self.live {
                // Many changes that each reach far: one layout from the roots is cheaper.
                self.relayout_pending_roots(m);
                break;
            }
        }
        self.saved.clear();
    }

    fn depth_of(&self, id: LayoutId) -> usize {
        std::iter::successors(self.nodes[id.index()].parent, |p| self.nodes[p.index()].parent).count()
    }

    /// Asks `id` again what `old` had answered and, while the answers differ, does the same to
    /// its parent. Returns the work done, counted in children of the ancestors visited.
    fn climb(&mut self, id: LayoutId, old: Answers, item_changed: bool, m: &mut dyn Measure) -> usize {
        let (mut cur, mut old, mut work) = (id, old, 0);
        // A value the parent reads changed: the parent runs whatever this node answers, and
        // asks it other things than before, so the old requests are not worth answering.
        let mut same = !item_changed && self.replay(cur, &old, m);
        while !same {
            let Some(parent) = self.nodes[cur.index()].parent else {
                if let Some(size) = self.nodes[cur.index()].root {
                    self.run_root(cur, size, m);
                }
                break;
            };
            let node = &mut self.nodes[parent.index()];
            let mut parent_changed = false;
            old = if node.pending == NOT_PENDING {
                std::mem::take(&mut node.answers)
            } else {
                let saved = self.saved[std::mem::replace(&mut node.pending, NOT_PENDING) as usize].take();
                let saved = saved.expect("a pending node has its saved answers");
                parent_changed = saved.item_changed;
                saved.answers
            };
            work += self.nodes[parent.index()].children.len() + 1;
            self.stats.climbed += 1;
            cur = parent;
            same = !parent_changed && self.replay(cur, &old, m);
        }
        work
    }

    /// Asks `id` the requests of `old` again, in the order they were asked and the perform
    /// last, and reports whether every answer is the same. It stops at the first that
    /// differs: the parent is then laid out again and asks what it needs. A request made with
    /// the size a node answered before is not one its parent repeats once that size changed,
    /// so a node whose size changes every frame is not laid out at its old sizes.
    fn replay(&mut self, id: LayoutId, old: &Answers, m: &mut dyn Measure) -> bool {
        if !old.done || old.lost {
            return false;
        }
        for entry in old.measures() {
            // The parent may have been given the old answer for any request it held for.
            let out = self.layout_node(m, id, entry.req, false);
            let span = self.nodes[id.index()].answers.span(&entry.req);
            if out != entry.out || !span.is_some_and(|s| s.covers(&entry.span)) {
                return false;
            }
        }
        old.perform().is_some_and(|entry| self.layout_node(m, id, entry.req, true) == entry.out)
    }

    fn run_root(&mut self, root: LayoutId, size: RootSize, m: &mut dyn Measure) {
        let style = self.nodes[root.index()].style.clone();
        let out = match size {
            RootSize::Fill(avail) => {
                let w = style.width.resolve(finite(avail.width)).unwrap_or(avail.width);
                let h = style.height.resolve(finite(avail.height)).unwrap_or(avail.height);
                self.layout_node(m, root, Request::new(Some(w), Some(h), avail.width, avail.height), true)
            }
            RootSize::Fit(avail) => {
                let w = style.width.resolve(finite(avail.width));
                let h = style.height.resolve(finite(avail.height));
                let fit = self.layout_node(m, root, Request::new(w, h, avail.width, avail.height), false);
                self.exact(root, fit, m)
            }
            RootSize::Width(w) => {
                let fit = self.layout_node(m, root, Request::new(Some(w), None, w, f32::INFINITY), false);
                self.exact(root, Size::new(w, fit.height), m)
            }
            RootSize::Fixed(size) => self.exact(root, size, m),
        };
        self.set_rect(root, Rect::new(0.0, 0.0, out.width, out.height));
    }

    fn exact(&mut self, root: LayoutId, size: Size, m: &mut dyn Measure) -> Size {
        let req = Request::new(Some(size.width), Some(size.height), size.width, size.height);
        self.layout_node(m, root, req, true)
    }

    /// The fallback of `update`: forgets everything under the roots that still have pending
    /// nodes and lays those roots out again.
    fn relayout_pending_roots(&mut self, m: &mut dyn Measure) {
        let mut roots = Vec::new();
        for i in 0..self.saved.len() {
            let Some(saved) = self.saved[i].take() else {
                continue;
            };
            self.nodes[saved.id.index()].pending = NOT_PENDING;
            let top = std::iter::successors(Some(saved.id), |p| self.nodes[p.index()].parent).last();
            roots.extend(top.filter(|r| !roots.contains(r)));
        }
        for root in roots {
            self.forget_under(root);
            if let Some(size) = self.nodes[root.index()].root {
                self.run_root(root, size, m);
            }
        }
    }

    fn forget_under(&mut self, id: LayoutId) {
        self.nodes[id.index()].answers = Answers::default();
        for i in 0..self.nodes[id.index()].children.len() {
            self.forget_under(self.nodes[id.index()].children[i]);
        }
    }

    /// The counters since the last call, which resets them.
    pub fn take_stats(&mut self) -> Stats {
        std::mem::take(&mut self.stats)
    }

    /// Lays a copy of the tree out from scratch and returns the first node whose rect or
    /// content size differs from this tree's, or `None`: the check that incremental layout is
    /// exact (specs/06, Tests). Call it after `layout` or `update`, with nothing pending.
    pub fn verify(&self, m: &mut dyn Measure) -> Option<String> {
        let mut scratch = self.clone();
        let roots: Vec<LayoutId> = (0..self.nodes.len())
            .map(LayoutId::from_index)
            .filter(|&id| self.nodes[id.index()].alive && self.nodes[id.index()].root.is_some())
            .collect();
        for &root in &roots {
            scratch.forget_under(root);
        }
        for &root in &roots {
            let size = scratch.nodes[root.index()].root.expect("a root has its size");
            scratch.run_root(root, size, m);
        }
        roots.iter().find_map(|&root| self.differs(&scratch, root))
    }

    fn differs(&self, fresh: &LayoutTree, id: LayoutId) -> Option<String> {
        let (a, b) = (&self.nodes[id.index()], &fresh.nodes[id.index()]);
        if a.rect != b.rect || a.content != b.content {
            return Some(format!(
                "node {}: rect {:?} content {:?}, from scratch {:?} {:?}",
                id.index(),
                a.rect,
                a.content,
                b.rect,
                b.content
            ));
        }
        a.children.iter().find_map(|&child| self.differs(fresh, child))
    }
}
