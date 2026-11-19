//! What a line is, for the parser of blocks. Each takes a line without the spaces around it.

use crate::block::Align;

/// "- x", "* x", "+ x", "1. x" and "1) x": the number of a numbered item, and its text.
pub fn list_item(t: &str) -> Option<(Option<u32>, &str)> {
    let b = t.as_bytes();
    if b.len() >= 2 && matches!(b[0], b'-' | b'*' | b'+') && b[1] == b' ' {
        return Some((None, &t[2..]));
    }
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    let marked = digits + 1 < b.len() && matches!(b[digits], b'.' | b')') && b[digits + 1] == b' ';
    // Nine digits at most: a number that fits.
    ((1..10).contains(&digits) && marked).then(|| (t[..digits].parse().ok(), &t[digits + 2..]))
}

/// "[ ] text" and "[x] text": whether the task is done, and its text. Other items have none.
pub fn task_parts(s: &str) -> (Option<bool>, &str) {
    let b = s.as_bytes();
    if b.len() >= 3 && b[0] == b'[' && b[2] == b']' && (b.len() == 3 || b[3] == b' ') {
        let done = match b[1] {
            b' ' => Some(false),
            b'x' | b'X' => Some(true),
            _ => None,
        };
        if done.is_some() {
            return (done, s[3..].trim_start_matches(' '));
        }
    }
    (None, s)
}

/// Three or more of `-`, `*` or `_`, whatever the spaces between them.
pub fn is_rule(t: &str) -> bool {
    let marks = || t.chars().filter(|c| *c != ' ');
    marks().count() >= 3 && ['-', '*', '_'].iter().any(|mark| marks().all(|c| c == *mark))
}

/// The cells of a table's row, split at each `|` that is not `\|`; a `\|` is a `|` of its
/// cell, in code too.
pub fn table_cells(t: &str) -> Vec<String> {
    let t = t.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = match t.strip_suffix('|') {
        Some(rest) if !rest.ends_with('\\') => rest,
        _ => t,
    };
    let (mut cells, mut cell, mut chars) = (Vec::new(), String::new(), t.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => cell.push(chars.next().unwrap_or('|')),
            '|' => cells.push(std::mem::take(&mut cell).trim().to_string()),
            _ => cell.push(c),
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

/// The line under the head of a table: `|---|:-:|`.
pub fn is_table_rule(t: &str) -> bool {
    let t = t.trim();
    t.len() > 2 && t.contains('-') && t.chars().all(|c| matches!(c, '|' | ':' | '-' | ' '))
}

pub fn cell_align(cell: &str) -> Align {
    match (cell.starts_with(':'), cell.ends_with(':')) {
        (true, true) => Align::Center,
        (false, true) => Align::Right,
        (true, false) => Align::Left,
        (false, false) => Align::None,
    }
}

/// How many `>` a line of a quote starts with, and the text after them.
pub fn quote_parts(t: &str) -> (usize, &str) {
    let marks = t.bytes().take_while(|c| matches!(c, b'>' | b' ')).count();
    (t[..marks].matches('>').count(), &t[marks..])
}
