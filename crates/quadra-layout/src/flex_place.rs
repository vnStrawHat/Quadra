//! Flexible lengths, placement along the main axis, and absolute children.

use quadra_prim::Rect;

use crate::cache::Request;
use crate::flex_items::{FlexItem, FlexLine, line_outer};
use crate::solve::{Measure, clamp_axis};
use crate::style::{Align, Justify, Style};
use crate::tree::{LayoutId, LayoutTree};

/// Gives a line's free space to its items by grow, or takes the overflow from them by shrink
/// times base, freezing items that reach their minimum or maximum until the result is stable.
///
/// On overflow, items that grow give space back first; fixed-size siblings (a header above a
/// scrolling body) shrink only if that is not enough. This differs from CSS on purpose: it
/// removes the need for `min-height: 0` fixes in every app shell.
pub(crate) fn resolve_flexible(items: &mut [FlexItem], l: &FlexLine, space: f32, gap: f32) {
    let n = l.end - l.start;
    if n == 0 {
        return;
    }
    let free = space - line_outer(items, l, gap, true);
    let line = &mut items[l.start..l.end];
    let fixed = gap * (n - 1) as f32 + line.iter().map(FlexItem::main_margins).sum::<f32>();
    let growing = free > 0.0;
    let flexible_first = !growing && line.iter().any(|it| it.grow > 0.0 && it.shrink > 0.0);
    for it in line.iter_mut() {
        it.main = it.hyp;
        it.frozen = free == 0.0
            || (growing && it.grow == 0.0)
            || (!growing && it.shrink == 0.0)
            || (flexible_first && it.grow == 0.0);
    }
    distribute(line, space - fixed, growing);
    let used: f32 = fixed + line.iter().map(|it| it.main).sum::<f32>();
    if flexible_first && space - used < -0.001 {
        for it in line.iter_mut() {
            it.frozen = it.shrink == 0.0 || it.grow > 0.0;
        }
        distribute(line, space - fixed, false);
    }
}

/// The freeze loop over the unfrozen items of a line.
fn distribute(line: &mut [FlexItem], inner: f32, growing: bool) {
    for _ in 0..=line.len() {
        let (mut rem, mut factor) = (inner, 0.0);
        for it in line.iter() {
            if it.frozen {
                rem -= it.main;
            } else {
                rem -= it.base;
                factor += if growing { it.grow } else { it.shrink * it.base };
            }
        }
        if factor == 0.0 {
            break;
        }
        let mut total = 0.0;
        for it in line.iter_mut().filter(|it| !it.frozen) {
            let share = if growing { it.grow } else { it.shrink * it.base };
            let target = it.base + rem * share / factor;
            let clamped = clamp_axis(target, Some(it.min_main), it.max_main).max(0.0);
            it.violation = clamped - target;
            total += it.violation;
            it.main = clamped;
        }
        if total.abs() < 0.001 {
            break;
        }
        for it in line.iter_mut().filter(|it| !it.frozen) {
            if (total > 0.0 && it.violation > 0.0) || (total < 0.0 && it.violation < 0.0) {
                it.frozen = true;
            }
        }
    }
}

/// Positions a line's items along the main axis (justify, auto margins) and inside the line.
pub(crate) fn place_line(
    items: &mut [FlexItem],
    l: &FlexLine,
    space: f32,
    gap: f32,
    justify: Justify,
    reverse: bool,
) {
    let n = l.end - l.start;
    if n == 0 {
        return;
    }
    let free = space - line_outer(items, l, gap, false);
    let line = &mut items[l.start..l.end];
    let autos = line.iter().map(|it| it.auto_ma as u32 + it.auto_mb as u32).sum::<u32>();
    let auto_share = if autos > 0 && free > 0.0 { free / autos as f32 } else { 0.0 };
    let (mut lead, mut between) = (0.0, gap);
    if autos == 0 {
        match justify {
            Justify::Start => {}
            Justify::End => lead = free,
            Justify::Center => lead = free / 2.0,
            Justify::Between if n > 1 && free > 0.0 => between += free / (n - 1) as f32,
            Justify::Around if free > 0.0 => {
                lead = free / n as f32 / 2.0;
                between += free / n as f32;
            }
            Justify::Evenly if free > 0.0 => {
                lead = free / (n + 1) as f32;
                between += free / (n + 1) as f32;
            }
            _ => {}
        }
    }
    let mut pos = lead;
    for it in line {
        pos += it.m_main_a + if it.auto_ma { auto_share } else { 0.0 };
        it.pos_main = if reverse { space - pos - it.main } else { pos };
        pos += it.main + it.m_main_b + if it.auto_mb { auto_share } else { 0.0 } + between;
        let free_cross = l.cross - it.cross - it.cross_margins();
        let offset = match (it.auto_ca, it.auto_cb, it.align) {
            (true, true, _) => free_cross / 2.0,
            (true, false, _) => free_cross,
            (false, true, _) => 0.0,
            (_, _, Align::Start | Align::Stretch) => 0.0,
            (_, _, Align::End) => free_cross,
            (_, _, Align::Center) => free_cross / 2.0,
        };
        it.pos_cross = l.pos + it.m_cross_a + offset;
    }
}

impl LayoutTree {
    /// Lays out absolutely positioned children against the padding box of their container.
    pub(crate) fn place_absolutes(
        &mut self,
        m: &mut dyn Measure,
        ids: &[LayoutId],
        s: &Style,
        outer_w: f32,
        outer_h: f32,
    ) {
        let (b, f) = (s.border, s.frame());
        let pbw = (outer_w - b.horizontal()).max(0.0);
        let pbh = (outer_h - b.vertical()).max(0.0);
        for &c in ids {
            let cs = self.nodes[c.index()].style.clone();
            let (mg, inset) = (cs.margin, cs.inset);
            let (left, right) = (inset.left.resolve(Some(pbw)), inset.right.resolve(Some(pbw)));
            let (top, bottom) = (inset.top.resolve(Some(pbh)), inset.bottom.resolve(Some(pbh)));
            let mut w = cs.width.resolve(Some(pbw));
            let mut h = cs.height.resolve(Some(pbh));
            if w.is_none()
                && let (Some(l), Some(r)) = (left, right)
            {
                w = Some((pbw - l - r - mg.horizontal()).max(0.0));
            }
            if h.is_none()
                && let (Some(t), Some(b)) = (top, bottom)
            {
                h = Some((pbh - t - b - mg.vertical()).max(0.0));
            }
            let (w, h) = match (w, h) {
                (Some(w), Some(h)) => (w, h),
                _ => {
                    let size = self.layout_node(m, c, Request::new(w, h, pbw, pbh), false);
                    (size.width, size.height)
                }
            };
            self.layout_node(m, c, Request::new(Some(w), Some(h), w, h), true);
            let x = match (left, right) {
                (Some(l), _) => b.left + l + mg.left,
                (None, Some(r)) => outer_w - b.right - r - w - mg.right,
                (None, None) => f.left + mg.left,
            };
            let y = match (top, bottom) {
                (Some(t), _) => b.top + t + mg.top,
                (None, Some(bt)) => outer_h - b.bottom - bt - h - mg.bottom,
                (None, None) => f.top + mg.top,
            };
            self.nodes[c.index()].rect = Rect::new(x, y, w, h);
        }
    }
}
