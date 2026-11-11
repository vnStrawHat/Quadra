//! Flex items: their base sizes, and the lines they are collected into.

use crate::cache::Request;
use crate::solve::{Measure, clamp_axis, inner_avail};
use crate::style::{Align, Style};
use crate::tree::{LayoutId, LayoutTree};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FlexItem {
    pub id: LayoutId,
    pub base: f32,
    /// The base size within the item's minimum and maximum.
    pub hyp: f32,
    pub main: f32,
    pub cross: f32,
    /// The cross size the style fixes, if it does.
    pub def_cross: Option<f32>,
    pub min_main: f32,
    pub max_main: Option<f32>,
    pub min_cross: f32,
    pub max_cross: Option<f32>,
    /// Margins at the start (a) and end (b) of each axis, and which of them are auto.
    pub m_main_a: f32,
    pub m_main_b: f32,
    pub m_cross_a: f32,
    pub m_cross_b: f32,
    pub auto_ma: bool,
    pub auto_mb: bool,
    pub auto_ca: bool,
    pub auto_cb: bool,
    pub grow: f32,
    pub shrink: f32,
    pub align: Align,
    pub frozen: bool,
    pub violation: f32,
    pub pos_main: f32,
    pub pos_cross: f32,
}

impl FlexItem {
    pub fn main_margins(&self) -> f32 {
        self.m_main_a + self.m_main_b
    }

    pub fn cross_margins(&self) -> f32 {
        self.m_cross_a + self.m_cross_b
    }

    pub fn clamp_cross(&self, v: f32) -> f32 {
        clamp_axis(v, Some(self.min_cross), self.max_cross)
    }

    /// Takes the line's cross size: nothing fixes its own and no auto margin holds it.
    pub fn stretches(&self) -> bool {
        self.align == Align::Stretch && self.def_cross.is_none() && !self.auto_ca && !self.auto_cb
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FlexLine {
    pub start: usize,
    pub end: usize,
    pub cross: f32,
    pub pos: f32,
}

/// Items and lines of one nesting depth of containers: reused, so a layout allocates nothing
/// once the tree's depth was seen.
#[derive(Clone, Default)]
pub(crate) struct Scratch {
    pub items: Vec<FlexItem>,
    pub lines: Vec<FlexLine>,
}

/// The container as its items see it.
pub(crate) struct Axes {
    pub row: bool,
    pub inner_w: Option<f32>,
    pub inner_h: Option<f32>,
    pub avail_w: f32,
    pub avail_h: f32,
    /// The cross size stretched items will get, when it is known up front.
    pub stretch_into: Option<f32>,
}

impl LayoutTree {
    pub(crate) fn flex_item(
        &mut self,
        m: &mut dyn Measure,
        c: LayoutId,
        cs: &Style,
        ax: &Axes,
        align_items: Align,
        sole_grower: bool,
    ) -> FlexItem {
        let row = ax.row;
        let (mg, auto) = (cs.margin, cs.margin_auto);
        let mut cw = cs.width.resolve(ax.inner_w);
        let mut ch = cs.height.resolve(ax.inner_h);
        if let Some(ratio) = cs.aspect_ratio {
            match (cw, ch) {
                (Some(w), None) => ch = Some(w / ratio),
                (None, Some(h)) => cw = Some(h * ratio),
                _ => {}
            }
        }
        let (inner_main, inner_cross) = if row { (ax.inner_w, ax.inner_h) } else { (ax.inner_h, ax.inner_w) };
        let (min_main, max_main, min_cross, max_cross) = if row {
            (cs.min_width, cs.max_width, cs.min_height, cs.max_height)
        } else {
            (cs.min_height, cs.max_height, cs.min_width, cs.max_width)
        };
        let mut it = FlexItem {
            id: c,
            def_cross: if row { ch } else { cw },
            min_main: min_main.resolve(inner_main).unwrap_or(0.0),
            max_main: max_main.resolve(inner_main),
            min_cross: min_cross.resolve(inner_cross).unwrap_or(0.0),
            max_cross: max_cross.resolve(inner_cross),
            m_main_a: if row { mg.left } else { mg.top },
            m_main_b: if row { mg.right } else { mg.bottom },
            m_cross_a: if row { mg.top } else { mg.left },
            m_cross_b: if row { mg.bottom } else { mg.right },
            auto_ma: if row { auto.left } else { auto.top },
            auto_mb: if row { auto.right } else { auto.bottom },
            auto_ca: if row { auto.top } else { auto.left },
            auto_cb: if row { auto.bottom } else { auto.right },
            grow: cs.grow,
            shrink: cs.shrink,
            align: cs.align_self.unwrap_or(align_items),
            ..FlexItem::default()
        };
        // Clamped now, not only at the end: the content is measured at this size, so text must
        // wrap at the width it ends up with.
        it.def_cross = it.def_cross.map(|d| it.clamp_cross(d));
        let def_main = if row { cw } else { ch };
        it.base = if let Some(basis) = cs.basis.resolve(inner_main) {
            basis
        } else if let Some(size) = def_main {
            size
        } else if sole_grower && cs.shrink > 0.0 && it.min_main == 0.0 && it.max_main.is_none() {
            0.0 // takes the leftover space: no need to measure its content
        } else {
            // Content size. An item that will stretch is measured at the stretched cross size,
            // so wrapping text in a column gets the right height.
            let mut cross_known = it.def_cross;
            if cross_known.is_none()
                && it.align == Align::Stretch
                && let Some(into) = ax.stretch_into
            {
                cross_known = Some(it.clamp_cross((into - it.cross_margins()).max(0.0)));
            }
            let avail_main = inner_avail(if row { ax.avail_w } else { ax.avail_h }, it.main_margins());
            let avail_cross = inner_avail(if row { ax.avail_h } else { ax.avail_w }, it.cross_margins());
            let req = if row {
                Request::new(None, cross_known, avail_main, avail_cross)
            } else {
                Request::new(cross_known, None, avail_cross, avail_main)
            };
            let size = self.layout_node(m, c, req, false);
            if row { size.width } else { size.height }
        };
        it.hyp = clamp_axis(it.base, Some(it.min_main), it.max_main).max(0.0);
        it.main = it.hyp;
        it
    }
}

/// Fills `lines` (empty) with the flex lines of `items`.
pub(crate) fn collect_lines(items: &[FlexItem], wrap: bool, limit: f32, gap: f32, lines: &mut Vec<FlexLine>) {
    let line = |start, end| FlexLine { start, end, ..FlexLine::default() };
    if !wrap || !limit.is_finite() || items.is_empty() {
        lines.push(line(0, items.len()));
        return;
    }
    let (mut start, mut used) = (0, 0.0);
    for (i, it) in items.iter().enumerate() {
        let outer = it.hyp + it.main_margins();
        if i > start && used + gap + outer > limit {
            lines.push(line(start, i));
            start = i;
            used = outer;
        } else {
            used += if i > start { gap + outer } else { outer };
        }
    }
    lines.push(line(start, items.len()));
}

/// The main extent of a line: its items (at their hypothetical or final size), their margins
/// and the gaps between them.
pub(crate) fn line_outer(items: &[FlexItem], l: &FlexLine, gap: f32, hyp: bool) -> f32 {
    let line = &items[l.start..l.end];
    let sum: f32 = line.iter().map(|it| if hyp { it.hyp } else { it.main } + it.main_margins()).sum();
    sum + gap * line.len().saturating_sub(1) as f32
}
