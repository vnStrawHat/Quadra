//! Grid layout: placement, track sizing (columns before rows), alignment.

use quadra_prim::{Rect, Size};

use crate::cache::Request;
use crate::grid::{GridStyle, Track, TrackSize};
use crate::grid_tracks::{TrackAxis, span_size, track_offsets};
use crate::solve::{Known, Measure, clamp_axis, inner_avail, inner_known};
use crate::style::{Display, Position, Style, finite};
use crate::tree::{LayoutId, LayoutTree};

impl LayoutTree {
    pub(crate) fn layout_grid(
        &mut self,
        m: &mut dyn Measure,
        id: LayoutId,
        s: &Style,
        known: Known,
        req: Request,
        perform: bool,
    ) -> Size {
        let f = s.frame();
        let none = GridStyle::default();
        let g = s.grid.as_deref().unwrap_or(&none);
        let inner_w = inner_known(known.width, f.horizontal());
        let inner_h = inner_known(known.height, f.vertical());
        let avail_w = inner_w.unwrap_or_else(|| inner_avail(req.aw, f.horizontal()));
        let mut columns = g.columns.clone();
        if g.fill_min > 0.0 {
            // As many columns of at least fill_min as fit, sharing the width equally.
            let fit = (avail_w + s.gap_col) / (g.fill_min + s.gap_col);
            let n = if avail_w.is_finite() { (fit as usize).max(1) } else { 1 };
            columns = Track::repeat(n, Track::minmax(TrackSize::Px(g.fill_min), TrackSize::Fr(1.0)));
        }

        let children = std::mem::take(&mut self.nodes[id.index()].children);
        let (mut flow, mut absolutes) = (Vec::new(), Vec::new());
        for &c in &children {
            let cs = &self.nodes[c.index()].style;
            if cs.display == Display::None {
                self.set_rect(c, Rect::default());
            } else if cs.position == Position::Absolute {
                absolutes.push(c);
            } else {
                flow.push(c);
            }
        }
        let items = self.place_grid_items(&flow, g, columns.len(), g.rows.len());
        // Items placed beyond the defined tracks get automatic ones.
        let ncols = items.iter().map(|it| it.col + it.cols).fold(columns.len(), usize::max);
        let nrows = items.iter().map(|it| it.row + it.rows).fold(g.rows.len(), usize::max);
        columns.resize(ncols, g.auto_cols);
        let mut rows = g.rows.clone();
        rows.resize(nrows, g.auto_rows);

        let col_axis = TrackAxis { columns: true, inner: inner_w, gap: s.gap_col };
        let row_axis = TrackAxis { columns: false, inner: inner_h, gap: s.gap_row };
        let colw = self.size_tracks(m, &columns, &items, &col_axis, &[]);
        let rowh = self.size_tracks(m, &rows, &items, &row_axis, &colw);
        let used_w = span_size(&colw, 0, colw.len(), s.gap_col);
        let used_h = span_size(&rowh, 0, rowh.len(), s.gap_row);
        let (aw, ah) = (finite(req.aw), finite(req.ah));
        let out = Size::new(
            known.width.unwrap_or_else(|| {
                clamp_axis(used_w + f.horizontal(), s.min_width.resolve(aw), s.max_width.resolve(aw))
            }),
            known.height.unwrap_or_else(|| {
                clamp_axis(used_h + f.vertical(), s.min_height.resolve(ah), s.max_height.resolve(ah))
            }),
        );
        if perform {
            let colx =
                track_offsets(&colw, s.gap_col, out.width - f.horizontal() - used_w, g.justify_content);
            let rowy = track_offsets(&rowh, s.gap_row, out.height - f.vertical() - used_h, g.align_content);
            let mut extent = Size::ZERO;
            for it in &items {
                let cell = Rect::new(
                    f.left + colx[it.col],
                    f.top + rowy[it.row],
                    span_size(&colw, it.col, it.cols, s.gap_col),
                    span_size(&rowh, it.row, it.rows, s.gap_row),
                );
                self.place_in_cell(m, it.id, cell, g.justify_items, s.align_items);
                let rect = self.nodes[it.id.index()].rect;
                extent.width = extent.width.max(rect.right());
                extent.height = extent.height.max(rect.bottom());
            }
            self.nodes[id.index()].content =
                Size::new(extent.width + s.padding.right, extent.height + s.padding.bottom);
            self.place_absolutes(m, &absolutes, s, out.width, out.height);
        }
        self.nodes[id.index()].children = children;
        out
    }
}
