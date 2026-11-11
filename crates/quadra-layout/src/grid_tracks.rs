//! Grid track sizing, and the placement of an item inside its cell.

use quadra_prim::Rect;

use crate::cache::Request;
use crate::grid::{Track, TrackSize};
use crate::grid_place::GridItem;
use crate::solve::{Measure, clamp_axis};
use crate::style::{Align, Justify};
use crate::tree::{LayoutId, LayoutTree};

/// One axis of a grid being sized.
pub(crate) struct TrackAxis {
    pub columns: bool,
    /// The container's inner size on this axis, when it is fixed.
    pub inner: Option<f32>,
    pub gap: f32,
}

/// The extent of `span` tracks from `start`, with the gaps between them.
pub(crate) fn span_size(sizes: &[f32], start: usize, span: usize, gap: f32) -> f32 {
    let tracks = sizes.iter().skip(start).take(span).sum::<f32>();
    tracks + gap * span.saturating_sub(1) as f32
}

/// Where each track starts, with one more entry for the end.
pub(crate) fn track_offsets(sizes: &[f32], gap: f32, free: f32, justify: Justify) -> Vec<f32> {
    let mut pos = match justify {
        Justify::Center => (free / 2.0).max(0.0),
        Justify::End => free.max(0.0),
        _ => 0.0,
    };
    let mut out = Vec::with_capacity(sizes.len() + 1);
    for size in sizes {
        out.push(pos);
        pos += size + gap;
    }
    out.push(pos);
    out
}

fn align_offset(align: Align, free: f32) -> f32 {
    match align {
        Align::End => free,
        Align::Center => free / 2.0,
        Align::Start | Align::Stretch => 0.0,
    }
}

impl LayoutTree {
    /// Resolves one axis. For rows, `col_widths` gives each item its final width.
    ///
    /// Two simplifications of CSS: an `fr` track has a minimum of zero, and an item spanning
    /// several tracks spreads what it needs evenly over those that are not fixed.
    pub(crate) fn size_tracks(
        &mut self,
        m: &mut dyn Measure,
        tracks: &[Track],
        items: &[GridItem],
        ax: &TrackAxis,
        col_widths: &[f32],
    ) -> Vec<f32> {
        let mut sizes = vec![0.0; tracks.len()];
        let mut fixed = vec![false; tracks.len()];
        for (i, track) in tracks.iter().enumerate() {
            if track.min.is_fixed_unit() && track.max.is_fixed_unit() {
                sizes[i] = track.max.fixed(ax.inner).unwrap_or(0.0);
                fixed[i] = true;
            } else if let Some(min) = track.min.fixed(ax.inner) {
                sizes[i] = min;
            }
        }
        // Content: items in one track first, then items spanning several.
        for spanning in [false, true] {
            for it in items {
                let (start, span) = if ax.columns { (it.col, it.cols) } else { (it.row, it.rows) };
                if spanning != (span > 1) {
                    continue;
                }
                // Skipped: fixed tracks, and fr tracks of a container with a fixed size, which
                // take the leftover and not their content.
                let skip =
                    |k: usize| fixed[k] || (matches!(tracks[k].max, TrackSize::Fr(_)) && ax.inner.is_some());
                let free: Vec<usize> = (start..start + span).filter(|&k| !skip(k)).collect();
                if free.is_empty() {
                    continue;
                }
                let first = tracks[start];
                let min_only = first.min == TrackSize::MinContent || first.max == TrackSize::MinContent;
                let need = self.contribution(m, it, ax, min_only, col_widths);
                let have = span_size(&sizes, start, span, ax.gap);
                if need > have {
                    let extra = (need - have) / free.len() as f32;
                    for k in free {
                        sizes[k] = clamp_axis(sizes[k] + extra, None, tracks[k].max.fixed(ax.inner));
                    }
                }
            }
        }
        // fr tracks share the space left over, never below what they already have.
        if let Some(space) = ax.inner {
            let mut used = ax.gap * tracks.len().saturating_sub(1) as f32;
            let mut shares = 0.0;
            for (track, size) in tracks.iter().zip(&sizes) {
                match track.max {
                    TrackSize::Fr(share) => shares += share,
                    _ => used += size,
                }
            }
            if shares > 0.0 {
                let unit = (space - used).max(0.0) / shares;
                for (track, size) in tracks.iter().zip(&mut sizes) {
                    if let TrackSize::Fr(share) = track.max {
                        *size = size.max(unit * share);
                    }
                }
            }
        }
        sizes
    }

    /// How much space an item needs along the axis, margins included.
    fn contribution(
        &mut self,
        m: &mut dyn Measure,
        it: &GridItem,
        ax: &TrackAxis,
        min_only: bool,
        col_widths: &[f32],
    ) -> f32 {
        let margin = self.nodes[it.id.index()].style.margin;
        if ax.columns {
            let avail = if min_only { 0.0 } else { f32::INFINITY };
            let size = self.layout_node(m, it.id, Request::new(None, None, avail, f32::INFINITY), false);
            return size.width + margin.horizontal();
        }
        let w = (span_size(col_widths, it.col, it.cols, ax.gap) - margin.horizontal()).max(0.0);
        let size = self.layout_node(m, it.id, Request::new(Some(w), None, w, f32::INFINITY), false);
        size.height + margin.vertical()
    }

    /// Sizes an item inside its cell (stretched by default) and positions it.
    pub(crate) fn place_in_cell(
        &mut self,
        m: &mut dyn Measure,
        id: LayoutId,
        cell: Rect,
        justify_items: Align,
        align_items: Align,
    ) {
        let cs = self.nodes[id.index()].style.clone();
        let justify = cs.grid.as_ref().and_then(|g| g.justify_self).unwrap_or(justify_items);
        let align = cs.align_self.unwrap_or(align_items);
        let inner = cell.inset(cs.margin);
        let mut w = cs.width.resolve(Some(cell.width));
        let mut h = cs.height.resolve(Some(cell.height));
        if w.is_none() && justify == Align::Stretch {
            w = Some(inner.width);
        }
        if h.is_none() && align == Align::Stretch {
            h = Some(inner.height);
        }
        let (w, h) = match (w, h) {
            (Some(w), Some(h)) => (w, h),
            _ => {
                let size = self.layout_node(m, id, Request::new(w, h, inner.width, inner.height), false);
                (size.width, size.height)
            }
        };
        self.layout_node(m, id, Request::new(Some(w), Some(h), w, h), true);
        let x = inner.x + align_offset(justify, inner.width - w);
        let y = inner.y + align_offset(align, inner.height - h);
        self.nodes[id.index()].rect = Rect::new(x, y, w, h);
    }
}
