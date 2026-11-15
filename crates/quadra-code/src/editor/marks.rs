//! The marks of a diff in the gutter (behaviour/18, "View"): against a text the editor is
//! told is the base, which lines were added, which changed, and where lines were removed.

use std::hash::{DefaultHasher, Hash, Hasher};

use quadra_design::{Step, diff_ids, diff_lines, token};
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
    marks_of(diff_lines(&old, &new), new.len())
}

/// The marks of the `lines` lines of a new text, from the script to it.
fn marks_of(script: Vec<Step>, lines: usize) -> Vec<Mark> {
    let mut out = vec![Mark::None; lines];
    let mut removed = 0;
    for step in script {
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

/// What the marks compare: a hash of each line of the base, and of each line of the text as
/// of a version of it. Lines whose hashes are equal are taken for equal.
pub(super) struct Base {
    old: Vec<u64>,
    new: Vec<u64>,
    version: Option<u64>,
}

fn hash(line: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    line.hash(&mut hasher);
    hasher.finish()
}

impl Base {
    pub fn new(text: &str) -> Base {
        Base { old: text.split('\n').map(hash).collect(), new: Vec::new(), version: None }
    }
}

impl EditorWidget {
    /// Finds the marks again, when there is a base: the lines changed since are hashed
    /// again, and the script between the hashes is found (what both start and end with
    /// alike costs a comparison of numbers a line).
    pub(super) fn remark(&mut self) {
        let (Some(base), doc) = (&mut self.base, &self.doc) else { return };
        let line = |n: usize| hash(&doc.buffer.line(n));
        let stale = base.version.and_then(|version| {
            doc.changed_lines(version, |lines, now| {
                base.new.splice(lines, std::iter::repeat_n(0, now));
            })
        });
        match stale {
            Some(stale) => stale.into_iter().flatten().for_each(|n| base.new[n] = line(n)),
            None => base.new = (0..doc.buffer.line_count()).map(line).collect(),
        }
        base.version = Some(doc.version);
        self.marks = marks_of(diff_ids(&base.old, &base.new), base.new.len());
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
