//! Ligatures (specs/18, "The view"): a token of cells shaped as one word, its glyphs put back
//! on the grid by the character each starts at.

use oneterm_vt::{Attrs, CellWidth, SnapshotContent, SnapshotRow};
use quadra_prim::Color;
use quadra_text::ShapedGlyph;

/// What drawing a token uses and gives back: its text, where each cell's character starts,
/// where each glyph goes, and each cell's colour once worked out.
#[derive(Default)]
pub(crate) struct Scratch {
    pub text: String,
    pub starts: Vec<u32>,
    pub placed: Vec<(usize, usize, f32)>,
    pub inks: Vec<Option<Color>>,
}

/// Whether the cell at `col` may be in a token: a narrow cell of one scalar that is no blank
/// and no drawing.
pub(crate) fn joins(row: &SnapshotRow, col: usize) -> bool {
    let cell = &row.cells[col];
    let SnapshotContent::Scalar(c) = cell.content else { return false };
    let drawing = matches!(c as u32, 0x2500..=0x25FF | 0xE0B0..=0xE0D7 | 0x1FB00..=0x1FBFF);
    cell.width == CellWidth::Narrow && !blank(c) && !drawing
}

/// Whether a cell's scalar draws nothing: a space, or the tab the engine keeps where a tab
/// started (for a copy).
pub(crate) fn blank(c: char) -> bool {
    matches!(c, ' ' | '\t')
}

/// The end of the token that starts at `col`: the cells that join, of its style run, up to
/// `stop` (the block cursor's column, drawn alone).
pub(crate) fn token_end(row: &SnapshotRow, col: usize, stop: Option<usize>) -> usize {
    let run = row.cells[col].run;
    let hidden = row.style_of(&row.cells[col]).attrs.contains(Attrs::HIDDEN);
    if hidden || !joins(row, col) || stop == Some(col) {
        return col + 1;
    }
    let mut end = col + 1;
    while end < row.cells.len() && row.cells[end].run == run && joins(row, end) && stop != Some(end) {
        end += 1;
    }
    end
}

/// Where each glyph of a shaped token goes: `(glyph, cell, x)`, `x` from the token's first
/// cell. `starts` is the byte each cell's character starts at, ascending; `cell` a cell's
/// width. Glyphs fall in groups by their cluster, each group in the middle of the cells from
/// its character to the next group's.
pub(crate) fn place(glyphs: &[ShapedGlyph], starts: &[u32], cell: f32, out: &mut Vec<(usize, usize, f32)>) {
    out.clear();
    let column = |byte: u32| starts.partition_point(|&s| s <= byte).saturating_sub(1);
    let mut g = 0;
    while g < glyphs.len() {
        let cluster = glyphs[g].cluster;
        let end = g + glyphs[g..].iter().take_while(|it| it.cluster == cluster).count();
        let group = &glyphs[g..end];
        let col = column(cluster);
        // The next group's column: the least cluster past this one.
        let next = glyphs.iter().map(|it| it.cluster).filter(|&c| c > cluster).min();
        let span = next.map_or(starts.len(), column).max(col + 1) - col;
        let (first, advance) = (group[0].x, group.iter().map(|it| it.advance).sum::<f32>());
        let shift = (span as f32 * cell - advance) / 2.0;
        for (k, glyph) in group.iter().enumerate() {
            out.push((g + k, col, col as f32 * cell + shift + glyph.x - first));
        }
        g = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(id: u16, x: f32, advance: f32, cluster: u32) -> ShapedGlyph {
        ShapedGlyph { id, x, advance, cluster, ..ShapedGlyph::default() }
    }

    #[test]
    fn a_ligature_spans_the_cells_of_its_characters() {
        // `a->b`, the arrow one glyph of two advances, in cells of 9 with advances of 6.
        let glyphs = [glyph(1, 0.0, 6.0, 0), glyph(2, 6.0, 12.0, 1), glyph(3, 18.0, 6.0, 3)];
        let mut out = Vec::new();
        place(&glyphs, &[0, 1, 2, 3], 9.0, &mut out);
        assert_eq!(out, [(0, 0, 1.5), (1, 1, 12.0), (2, 3, 28.5)], "each in the middle of its cells");
    }

    #[test]
    fn glyphs_of_their_own_clusters_keep_their_cells() {
        // Contextual forms: a glyph a character, of a byte or more each.
        let glyphs = [glyph(1, 0.0, 6.0, 0), glyph(2, 6.0, 6.0, 2), glyph(3, 12.0, 6.0, 3)];
        let mut out = Vec::new();
        place(&glyphs, &[0, 2, 3], 6.0, &mut out);
        assert_eq!(out, [(0, 0, 0.0), (1, 1, 6.0), (2, 2, 12.0)]);
        // A mark on its letter: two glyphs of one cluster in one cell.
        let glyphs = [glyph(1, 0.0, 6.0, 0), glyph(2, 6.0, 0.0, 0), glyph(3, 6.0, 6.0, 1)];
        place(&glyphs, &[0, 1], 6.0, &mut out);
        assert_eq!(out, [(0, 0, 0.0), (1, 0, 6.0), (2, 1, 6.0)]);
    }
}
