//! The marks of a diff in the gutter (behaviour/18, "View"): against a text the editor is
//! told is the base, which lines were added, which changed, and where lines were removed.

use quadra_kit::{Step, diff_lines, token};
use quadra_prim::{Color, Rect};
use quadra_render::Quad;
use quadra_tree::{PaintCx, Theme};

use super::EditorWidget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    None,
    Added,
    Changed,
    /// Lines were removed before this one.
    Removed,
}

/// The mark of each line of `new` against `old`: removed lines followed by added ones are a
/// change of as many of them as there are of both.
pub fn marks(old: &str, new: &str) -> Vec<Mark> {
    let (old, new): (Vec<&str>, Vec<&str>) = (old.split('\n').collect(), new.split('\n').collect());
    let mut out = vec![Mark::None; new.len()];
    let mut removed = 0;
    for step in diff_lines(&old, &new) {
        match step {
            Step::Removed(_) => removed += 1,
            Step::Added(line) if removed > 0 => {
                out[line] = Mark::Changed;
                removed -= 1;
            }
            Step::Added(line) => out[line] = Mark::Added,
            Step::Same(_, line) => {
                if removed > 0 {
                    out[line] = Mark::Removed;
                }
                removed = 0;
            }
        }
    }
    if removed > 0
        && let Some(last) = out.last_mut()
        && *last == Mark::None
    {
        *last = Mark::Removed;
    }
    out
}

/// The colours of the marks: added, changed, removed.
pub(super) fn colors(theme: &Theme) -> [Color; 3] {
    [theme.color(token::OK), theme.color(token::WARN), theme.color(token::ERROR)]
}

impl EditorWidget {
    /// Finds the marks again, when there is a base.
    // ponytail: the whole text is compared after each change; against the changed lines only
    // if large files with a base make typing slow.
    pub(super) fn remark(&mut self) {
        if let Some(base) = &self.base {
            self.marks = marks(base, &self.doc.text());
        }
    }

    /// Paints the mark of a line on its rows, a bar two cells left of the gutter's edge; a
    /// removal a short bar at the top of the row.
    pub(super) fn paint_mark(&self, cx: &mut PaintCx, line: usize, top: f32) {
        let (rows, x) = (self.line_rows(line), cx.rect.x + self.gutter - 2.0 * self.cell);
        let tall = rows.len() as f32 * self.line_px;
        let [added, changed, removed] = self.mark_colors;
        let (bounds, background) = match self.marks.get(line) {
            Some(Mark::Added) => (Rect::new(x, top, 3.0, tall), added),
            Some(Mark::Changed) => (Rect::new(x, top, 3.0, tall), changed),
            Some(Mark::Removed) => (Rect::new(x - 2.0, top - 1.0, 7.0, 2.0), removed),
            _ => return,
        };
        cx.quad(Quad { bounds, background, ..Quad::default() });
    }
}
