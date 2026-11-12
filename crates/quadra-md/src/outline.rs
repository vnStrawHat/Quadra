//! The outline of a document: its headings, with their source lines and their anchors, and
//! what an app asks of a text beside (the block of a line, the count of words).

use crate::block::{Block, Kind};
use crate::inline::parse_inline;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    /// Without its marks.
    pub text: String,
    /// The source line.
    pub line: usize,
    /// Which of the blocks it is.
    pub block: usize,
    /// As hosts make them: `## Hello, World` is `hello-world`.
    pub anchor: String,
}

/// The headings of a document, in order.
pub fn outline(blocks: &[Block]) -> Vec<Heading> {
    let heading = |(block, of): (usize, &Block)| match &of.kind {
        Kind::Heading { level, text } => {
            let text = plain(text);
            Some(Heading { level: *level, anchor: anchor(&text), text, line: of.line, block })
        }
        _ => None,
    };
    blocks.iter().enumerate().filter_map(heading).collect()
}

/// The text of inlines without their marks (math keeps its source).
pub fn plain(text: &str) -> String {
    parse_inline(text).into_iter().map(|span| span.text).collect()
}

/// A text in lower case, its spaces dashes and its punctuation dropped.
pub fn anchor(text: &str) -> String {
    let kept = |c: char| match c {
        ' ' | '-' => Some('-'),
        c if c.is_alphanumeric() || c == '_' => Some(c),
        _ => None,
    };
    text.trim().to_lowercase().chars().filter_map(kept).collect()
}

/// Which block a source line is in: the last that starts on it or before it (so the one
/// before a blank line); 0 when there is none.
pub fn block_at(blocks: &[Block], line: usize) -> usize {
    blocks.partition_point(|block| block.line <= line).saturating_sub(1)
}

/// How many words a text has (runs of letters and digits): for the time it takes to read.
pub fn words(source: &str) -> usize {
    source.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).count()
}
