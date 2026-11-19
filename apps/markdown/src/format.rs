//! What the toolbar does to the text (specs/21, "Toolbar"): pure functions from the text and
//! the selection to the edits and the selection after them.

use std::ops::Range;

/// A button of the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Strike,
    Code,
    Heading(u8),
    Bullet,
    Numbered,
    Task,
    Quote,
    Link,
    Image,
    CodeBlock,
    Table,
    Rule,
}

/// The edits, in the order of the text and not over each other, and the selection after
/// them (anchor, head).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub edits: Vec<(Range<usize>, String)>,
    pub selection: Range<usize>,
}

/// What `what` makes of `text` with `selection` (byte offsets).
pub fn format(text: &str, selection: Range<usize>, what: Format) -> Change {
    match what {
        Format::Bold => wrap(text, selection, "**"),
        Format::Italic => wrap(text, selection, "*"),
        Format::Strike => wrap(text, selection, "~~"),
        Format::Code => wrap(text, selection, "`"),
        Format::Heading(level) => lines(text, selection, Prefix::Heading(level)),
        Format::Bullet => lines(text, selection, Prefix::Bullet),
        Format::Numbered => lines(text, selection, Prefix::Numbered),
        Format::Task => lines(text, selection, Prefix::Task),
        Format::Quote => lines(text, selection, Prefix::Quote),
        Format::Link => link(text, selection, "", "text", "url"),
        Format::Image => link(text, selection, "!", "alt", "path"),
        Format::CodeBlock => fence(text, selection),
        Format::Table => {
            let table = "| Column 1 | Column 2 | Column 3 |\n|---|---|---|\n|  |  |  |\n|  |  |  |";
            block(text, selection.end, table, 2..10)
        }
        Format::Rule => block(text, selection.end, "---", 3..3),
    }
}

/// Wraps the selection in `mark`, or the word at the caret, or puts an empty pair with the
/// caret between; a selection already wrapped, inside or just outside, is unwrapped.
fn wrap(text: &str, sel: Range<usize>, mark: &str) -> Change {
    let m = mark.len();
    let sel = if sel.is_empty() { word_at(text, sel.start) } else { sel };
    let inner = &text[sel.clone()];
    if inner.len() >= 2 * m && inner.starts_with(mark) && inner.ends_with(mark) {
        let kept = inner[m..inner.len() - m].to_string();
        let end = sel.start + kept.len();
        return Change { edits: vec![(sel.clone(), kept)], selection: sel.start..end };
    }
    let before = sel.start >= m && text[..sel.start].ends_with(mark);
    if before && text[sel.end..].starts_with(mark) {
        let edits = vec![(sel.start - m..sel.start, String::new()), (sel.end..sel.end + m, String::new())];
        return Change { edits, selection: sel.start - m..sel.end - m };
    }
    let edits = vec![(sel.start..sel.start, mark.to_string()), (sel.end..sel.end, mark.to_string())];
    Change { edits, selection: sel.start + m..sel.end + m }
}

/// The word around `at` (letters, digits, `_`), or an empty range at it.
fn word_at(text: &str, at: usize) -> Range<usize> {
    let part = |c: char| c.is_alphanumeric() || c == '_';
    let start = text[..at].char_indices().rev().take_while(|&(_, c)| part(c)).last().map_or(at, |(i, _)| i);
    let end = text[at..].char_indices().find(|&(_, c)| !part(c)).map_or(text.len(), |(i, _)| at + i);
    start..end
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Prefix {
    Heading(u8),
    Bullet,
    Numbered,
    Task,
    Quote,
}

/// The prefix a line has after its indent, and its length: a heading's marks, a task's, a
/// bullet's, a number's, a quote's.
fn prefix_of(line: &str) -> Option<(Prefix, usize)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        return Some((Prefix::Heading(hashes as u8), hashes + 1));
    }
    for task in ["- [ ] ", "- [x] ", "- [X] "] {
        if line.starts_with(task) {
            return Some((Prefix::Task, task.len()));
        }
    }
    if ["- ", "* ", "+ "].iter().any(|bullet| line.starts_with(bullet)) {
        return Some((Prefix::Bullet, 2));
    }
    if line.starts_with("> ") {
        return Some((Prefix::Quote, 2));
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0 && line[digits..].starts_with(". ")).then_some((Prefix::Numbered, digits + 2))
}

/// Sets `kind` on every line the selection touches, replacing another prefix, or takes it
/// off when every one of them has it already.
fn lines(text: &str, sel: Range<usize>, kind: Prefix) -> Change {
    let first = text[..sel.start].rfind('\n').map_or(0, |i| i + 1);
    let last_end = text[sel.end..].find('\n').map_or(text.len(), |i| sel.end + i);
    let mut starts = vec![first];
    starts.extend(text[first..last_end].match_indices('\n').map(|(i, _)| first + i + 1));
    let had: Vec<(usize, Option<(Prefix, usize)>)> = starts
        .iter()
        .map(|&start| {
            let line = &text[start..text[start..].find('\n').map_or(text.len(), |i| start + i)];
            let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
            (start + indent, prefix_of(&line[indent..]))
        })
        .collect();
    let off = had.iter().all(|(_, it)| it.is_some_and(|(p, _)| p == kind));
    let edits: Vec<(Range<usize>, String)> = had
        .iter()
        .enumerate()
        .map(|(n, &(at, it))| {
            let old = it.map_or(0, |(_, len)| len);
            let new = match kind {
                _ if off => String::new(),
                Prefix::Heading(level) => format!("{} ", "#".repeat(level.into())),
                Prefix::Bullet => "- ".into(),
                Prefix::Numbered => format!("{}. ", n + 1),
                Prefix::Task => "- [ ] ".into(),
                Prefix::Quote => "> ".into(),
            };
            (at..at + old, new)
        })
        .collect();
    let edits: Vec<_> =
        edits.into_iter().filter(|(range, new)| !(range.is_empty() && new.is_empty())).collect();
    let selection = through(sel.start, &edits)..through(sel.end, &edits);
    Change { edits, selection }
}

/// Where an offset goes once `edits` are made: past an edit before it, by its change of
/// length; inside one, to the end of its new text.
fn through(offset: usize, edits: &[(Range<usize>, String)]) -> usize {
    let mut shift = 0isize;
    for (range, new) in edits {
        if range.end <= offset {
            shift += new.len() as isize - range.len() as isize;
        } else if range.start < offset {
            return (range.start as isize + shift) as usize + new.len();
        }
    }
    offset.saturating_add_signed(shift)
}

/// `[selection](url)` with `url` selected, or `[text](url)` with `text` selected; `bang` makes
/// it an image.
fn link(text: &str, sel: Range<usize>, bang: &str, label: &str, target: &str) -> Change {
    let words = &text[sel.clone()];
    let (shown, picked) = if words.is_empty() { (label, true) } else { (words, false) };
    let made = format!("{bang}[{shown}]({target})");
    let at = sel.start + bang.len() + 1;
    let selection = match picked {
        true => at..at + shown.len(),
        false => {
            let url = at + shown.len() + 2;
            url..url + target.len()
        }
    };
    Change { edits: vec![(sel, made)], selection }
}

/// Fences the lines the selection touches, the caret after the opening fence; with nothing
/// selected, an empty block.
fn fence(text: &str, sel: Range<usize>) -> Change {
    if sel.is_empty() {
        return block(text, sel.end, "```\n\n```", 3..3);
    }
    let first = text[..sel.start].rfind('\n').map_or(0, |i| i + 1);
    let last_end = text[sel.end..].find('\n').map_or(text.len(), |i| sel.end + i);
    let edits = vec![(first..first, "```\n".to_string()), (last_end..last_end, "\n```".to_string())];
    Change { edits, selection: first + 3..first + 3 }
}

/// Puts `body` on lines of its own after the line of `at`, with a blank line before and
/// after it where its neighbours are not blank; `picked` is selected in it.
fn block(text: &str, at: usize, body: &str, picked: Range<usize>) -> Change {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let blank = |line: &str| line.trim().is_empty();
    let before = match blank(&text[start..end]) {
        true if start > 0 && !blank(text[..start - 1].rsplit('\n').next().unwrap_or("")) => "\n",
        true => "",
        false => "\n\n",
    };
    let next = (end < text.len()).then(|| text[end + 1..].split('\n').next().unwrap_or(""));
    let after = if next.is_some_and(|line| !blank(line)) { "\n" } else { "" };
    let inserted = format!("{before}{body}{after}");
    let base = end + before.len();
    Change { edits: vec![(end..end, inserted)], selection: base + picked.start..base + picked.end }
}

#[cfg(test)]
mod tests;
