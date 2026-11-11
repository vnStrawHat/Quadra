//! Flexbox (specs/behaviour/05, "Style"): CSS flexbox restricted to the style's values.
//!
//! Two deliberate differences from CSS: an item's minimum size is zero unless the style sets
//! one (components opt out of shrinking with shrink 0), and `align-content` is always start.

use quadra_prim::{Rect, Size};

use crate::cache::Request;
use crate::flex_items::{Axes, collect_lines, line_outer};
use crate::flex_place::{place_line, resolve_flexible};
use crate::solve::{Known, Measure, clamp_axis, inner_avail, inner_known};
use crate::style::{Display, Overflow, Position, Style, finite};
use crate::tree::{LayoutId, LayoutTree};

impl LayoutTree {
    pub(crate) fn layout_flex(
        &mut self,
        m: &mut dyn Measure,
        id: LayoutId,
        s: &Style,
        known: Known,
        req: Request,
        perform: bool,
    ) -> Size {
        let f = s.frame();
        let row = s.is_row();
        let inner_w = inner_known(known.width, f.horizontal());
        let inner_h = inner_known(known.height, f.vertical());
        let avail_w = inner_w.unwrap_or_else(|| inner_avail(req.aw, f.horizontal()));
        let avail_h = inner_h.unwrap_or_else(|| inner_avail(req.ah, f.vertical()));
        let (inner_main, inner_cross) = if row { (inner_w, inner_h) } else { (inner_h, inner_w) };
        let (avail_main, avail_cross) = if row { (avail_w, avail_h) } else { (avail_h, avail_w) };
        let (gap_main, gap_cross) = if row { (s.gap_col, s.gap_row) } else { (s.gap_row, s.gap_col) };
        let (frame_main, frame_cross) =
            if row { (f.horizontal(), f.vertical()) } else { (f.vertical(), f.horizontal()) };
        // The node's own limits on each axis, for the axes its size is not fixed on.
        let limits_w = (s.min_width.resolve(finite(req.aw)), s.max_width.resolve(finite(req.aw)));
        let limits_h = (s.min_height.resolve(finite(req.ah)), s.max_height.resolve(finite(req.ah)));
        let (limits_main, limits_cross) = if row { (limits_w, limits_h) } else { (limits_h, limits_w) };
        // Only scrolling counts: `hidden` only clips, its size is still its content. A scrolling
        // cross axis lets items exceed it, so stretching cannot skip measuring them; items of a
        // scrolling main axis keep their size and overflow instead of shrinking.
        let scrolls = |o: Overflow| o == Overflow::Scroll;
        let (scroll_main, clips_cross) = if row {
            (scrolls(s.overflow_x), scrolls(s.overflow_y))
        } else {
            (scrolls(s.overflow_y), scrolls(s.overflow_x))
        };
        let stretch_into = if !s.wrap && !clips_cross { inner_cross } else { None };
        // What a row that scrolls sideways holds is measured with all the width it asks for.
        let avail_w = if row && scroll_main { f32::INFINITY } else { avail_w };
        let ax = Axes { row, inner_w, inner_h, avail_w, avail_h, stretch_into };

        // Taken, not copied: layout never adds nodes or changes child lists. Items and lines
        // live in scratch kept per nesting depth; the children laid out below use the next.
        let children = std::mem::take(&mut self.nodes[id.index()].children);
        let depth = self.depth;
        self.depth += 1;
        if depth == self.scratch.len() {
            self.scratch.push(Default::default());
        }
        let mut items = std::mem::take(&mut self.scratch[depth].items);
        let mut lines = std::mem::take(&mut self.scratch[depth].lines);
        items.clear();
        lines.clear();
        let mut absolutes = Vec::new();

        // The only growing item of a container with a definite main size ends at the leftover
        // space whatever its content, so its base size is never measured: an app shell's body.
        let mut sole = None;
        if inner_main.is_some() && !s.wrap && !scroll_main {
            let in_flow = |c: &&LayoutId| {
                let cs = &self.nodes[c.index()].style;
                cs.display != Display::None && cs.position != Position::Absolute && cs.grow > 0.0
            };
            let mut growers = children.iter().filter(in_flow);
            sole = growers.next().copied().filter(|_| growers.next().is_none());
        }
        for &c in &children {
            let cs = self.nodes[c.index()].style.clone();
            if cs.display == Display::None {
                self.set_rect(c, Rect::default());
            } else if cs.position == Position::Absolute {
                absolutes.push(c);
            } else {
                let mut item = self.flex_item(m, c, &cs, &ax, s.align_items, sole == Some(c));
                if scroll_main {
                    item.shrink = 0.0;
                }
                items.push(item);
            }
        }

        collect_lines(&items, s.wrap, inner_main.unwrap_or(avail_main), gap_main, &mut lines);
        // Main size of an auto container: its widest line. Width is limited by the available
        // space (so text can wrap), and so is a scroll container's height (it scrolls instead);
        // any other auto height is the content height, so tall content never squeezes its own
        // children.
        let space = inner_main.unwrap_or_else(|| {
            let content = lines.iter().map(|l| line_outer(&items, l, gap_main, true)).fold(0.0, f32::max);
            let limited = row || s.overflow_y == Overflow::Scroll;
            let space = if limited && avail_main.is_finite() { content.min(avail_main) } else { content };
            (clamp_axis(space + frame_main, limits_main.0, limits_main.1) - frame_main).max(0.0)
        });
        for l in &lines {
            resolve_flexible(&mut items, l, space, gap_main);
        }

        // Cross sizes: measured at the final main size, then stretched to the line.
        for item in items.iter_mut() {
            let it = *item;
            item.cross = if let Some(size) = it.def_cross {
                it.clamp_cross(size)
            } else if it.stretches()
                && let Some(into) = ax.stretch_into
            {
                // Decided by the stretch below: measuring it would be wasted work.
                it.clamp_cross((into - it.cross_margins()).max(0.0))
            } else {
                let around = inner_avail(avail_cross, it.cross_margins());
                let size = if row {
                    self.layout_node(m, it.id, Request::new(Some(it.main), None, it.main, around), false)
                        .height
                } else {
                    self.layout_node(m, it.id, Request::new(None, Some(it.main), around, it.main), false)
                        .width
                };
                it.clamp_cross(size)
            };
        }
        let (mut total_cross, count) = (0.0, lines.len());
        for (i, l) in lines.iter_mut().enumerate() {
            l.cross =
                items[l.start..l.end].iter().map(|it| it.cross + it.cross_margins()).fold(l.cross, f32::max);
            if count == 1
                && let Some(inner) = inner_cross
            {
                // A scrolling cross axis stretches items to at least the viewport.
                l.cross = if clips_cross { l.cross.max(inner) } else { inner };
            }
            l.pos = total_cross;
            total_cross += l.cross + if i + 1 < count { gap_cross } else { 0.0 };
        }
        let mut cross_size = inner_cross.unwrap_or_else(|| {
            let size = (clamp_axis(total_cross + frame_cross, limits_cross.0, limits_cross.1) - frame_cross)
                .max(0.0);
            if count == 1 {
                lines[0].cross = size;
            }
            size
        });
        for l in &lines {
            for it in items[l.start..l.end].iter_mut().filter(|it| it.stretches()) {
                it.cross = it.clamp_cross(l.cross - it.cross_margins());
            }
            place_line(&mut items, l, space, gap_main, s.justify, s.is_reverse());
        }

        // A container scrolling its cross axis reports at most the available size; its
        // stretched children keep the content size and overflow (that is what it scrolls).
        if inner_cross.is_none() && clips_cross && avail_cross.is_finite() {
            cross_size = cross_size.min(avail_cross);
        }
        let (content_w, content_h) = if row { (space, cross_size) } else { (cross_size, space) };
        let out = Size::new(
            known.width.unwrap_or(content_w + f.horizontal()),
            known.height.unwrap_or(content_h + f.vertical()),
        );
        if perform {
            let mut extent = Size::ZERO;
            for it in &items {
                let (w, h) = if row { (it.main, it.cross) } else { (it.cross, it.main) };
                self.layout_node(m, it.id, Request::new(Some(w), Some(h), w, h), true);
                let (x, y) = if row { (it.pos_main, it.pos_cross) } else { (it.pos_cross, it.pos_main) };
                let (x, y) = (f.left + x, f.top + y);
                self.set_rect(it.id, Rect::new(x, y, w, h));
                let (after_x, after_y) =
                    if row { (it.m_main_b, it.m_cross_b) } else { (it.m_cross_b, it.m_main_b) };
                extent.width = extent.width.max(x + w + after_x);
                extent.height = extent.height.max(y + h + after_y);
            }
            self.nodes[id.index()].content =
                Size::new(extent.width + s.padding.right, extent.height + s.padding.bottom);
            self.place_absolutes(m, &absolutes, s, out.width, out.height);
        }
        self.scratch[depth].items = items;
        self.scratch[depth].lines = lines;
        self.depth -= 1;
        self.nodes[id.index()].children = children;
        out
    }
}
