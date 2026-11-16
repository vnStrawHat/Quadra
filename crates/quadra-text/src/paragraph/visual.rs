//! Lines in visual order, for a text with right-to-left characters (specs/08, "Bidirectional
//! text"): the words and spaces of each line reordered by their levels, in a copy of the
//! glyphs.

use super::{Kind, Paragraph};
use crate::bidi::visual_order;

impl Paragraph {
    /// The widths at which [`Paragraph::wrap`] gives the lines it gave last, both ends
    /// included: from the widest line up to the width they were broken at, or any width up
    /// when no line broke for width.
    pub fn kept_widths(&self) -> (f32, f32) {
        let up = if self.width_breaks { self.wrapped_at.unwrap_or(0.0) } else { f32::INFINITY };
        (self.size.width, up)
    }

    /// Places the glyphs of each line again, its tokens in visual order.
    pub(super) fn place_visually(&mut self) {
        let Paragraph { bidi, glyphs, visual, tokens, lines, .. } = self;
        let Some((_, levels)) = bidi else { return };
        visual.clear();
        visual.extend_from_slice(glyphs);
        for line in lines.iter() {
            let first = tokens.partition_point(|t| t.end <= line.start);
            let count = tokens[first..].iter().take_while(|t| t.start < line.end).count();
            let on = &tokens[first..first + count];
            if on.iter().any(|t| t.start < line.start || t.end > line.end) {
                continue; // a word broken inside keeps its order
            }
            // Spaces at the end of a line hang past its width, after what is shown, as in
            // text of either direction.
            let words_end = on.iter().rposition(|t| t.kind == Kind::Word).map_or(0, |n| n + 1);
            let at: Vec<u8> = on[..words_end]
                .iter()
                .map(|t| levels[t.start..t.end].iter().copied().min().unwrap_or(0))
                .collect();
            if at.iter().all(|&level| level == 0) {
                continue;
            }
            let mut x = line.origin;
            for k in visual_order(&at).into_iter().chain(words_end..on.len()) {
                let token = &on[k];
                for g in token.glyphs.clone() {
                    visual[g].x = x + glyphs[g].x - token.x;
                }
                x += token.width;
            }
        }
    }
}
