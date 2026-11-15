//! Semantic highlighting (specs/18, "The view"): what text in the default foreground means,
//! a class a cell, worked out a logical line at a time and kept by row id, so a paint scans
//! only the lines that changed.

mod output;
mod probe;
mod scan;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::ops::Range;

use oneterm_vt::{Attrs, Color as VtColor, NamedColor, RowId, Semantic, SeqNo, SnapshotContent, SnapshotRow};

pub(crate) use scan::Role;

/// What a run of text means; each has its colour (`TermColors::highlights`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Highlight {
    /// The prompt's sign (`$`, `#`, `>`, `❯`).
    PromptSign = 1,
    /// The first word of a command line.
    Command,
    /// `--flag`, `-x`.
    Flag,
    Error,
    Success,
    Warn,
    Info,
    Debug,
    Path,
    /// An IPv4 or IPv6 address.
    Ip,
    Mac,
    DateTime,
    Number,
    /// A quoted string.
    Quoted,
    Operator,
    Bracket,
    Url,
    /// The permission bits of `ls -l`: the type, then each bit.
    PermType,
    PermRead,
    PermWrite,
    PermExec,
    PermSpecial,
    PermNone,
}

impl Highlight {
    pub const COUNT: usize = 23;
    pub const ALL: [Highlight; Highlight::COUNT] = {
        use Highlight::*;
        [
            PromptSign,
            Command,
            Flag,
            Error,
            Success,
            Warn,
            Info,
            Debug,
            Path,
            Ip,
            Mac,
            DateTime,
            Number,
            Quoted,
            Operator,
            Bracket,
            Url,
            PermType,
            PermRead,
            PermWrite,
            PermExec,
            PermSpecial,
            PermNone,
        ]
    };

    /// The ANSI colour (0 to 15) it takes when the look sets none.
    pub fn ansi(self) -> usize {
        use Highlight::*;
        match self {
            PromptSign | Error | Operator | PermExec => 1,
            Success | DateTime | PermRead => 2,
            Warn | Quoted | PermWrite => 3,
            Path => 4,
            Flag | Bracket | PermSpecial => 5,
            Info | PermType => 6,
            Debug | PermNone => 8,
            Mac => 9,
            Command => 11,
            Ip | Url => 12,
            Number => 13,
        }
    }

    /// Its place in `TermColors::highlights`.
    pub fn index(self) -> usize {
        self as usize - 1
    }

    pub(crate) fn of(class: u8) -> Option<Highlight> {
        Highlight::ALL.get(usize::from(class).checked_sub(1)?).copied()
    }

    /// Drawn underlined too.
    pub(crate) fn underlined(self) -> bool {
        matches!(self, Highlight::Url | Highlight::Ip)
    }
}

/// What the shell's marks say of a logical line: the zone of its first marked cell, and
/// whether it has input cells (the shell closed its prompt).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LineMark {
    head: Semantic,
    closed: bool,
}

/// A row's classes and what they were worked out from.
struct Entry {
    seq: SeqNo,
    /// The first row of its logical line, how many rows that has, and the mark of the line
    /// above (`None`: the viewport's first line).
    head: RowId,
    rows: usize,
    prev: Option<LineMark>,
    mark: LineMark,
    role: Option<Role>,
    /// A class a column; empty when none has one.
    classes: Vec<u8>,
    seen: u32,
}

/// The classes of the rows that show, kept by row id.
#[derive(Default)]
pub(crate) struct Marks {
    rows: HashMap<RowId, Entry>,
    epoch: u32,
    /// Lines scanned so far.
    pub(crate) scanned: usize,
    chars: Vec<char>,
    text: String,
    /// For each character: its row in the line and its column.
    at: Vec<(usize, usize)>,
    out: Vec<u8>,
}

/// Whether a cell's glyph takes its class's colour: the program left it the default.
pub(crate) fn plain(style: &oneterm_vt::Style) -> bool {
    style.fg == VtColor::Named(NamedColor::Foreground) && !style.attrs.contains(Attrs::INVERSE)
}

impl Marks {
    pub(crate) fn clear(&mut self) {
        self.rows.clear();
    }

    /// The classes of `rows`, the viewport: lines whose rows, stamps or mark above changed
    /// are scanned again, the others kept.
    pub(crate) fn update(&mut self, rows: &[SnapshotRow]) {
        self.epoch = self.epoch.wrapping_add(1);
        let (mut prev, mut start) = (None, 0);
        while start < rows.len() {
            let mut end = start;
            while end + 1 < rows.len() && rows[end].wrapped {
                end += 1;
            }
            let line = &rows[start..=end];
            let kept = line.iter().all(|row| {
                self.rows.get(&row.id).is_some_and(|e| {
                    e.seq == row.seq && e.head == line[0].id && e.rows == line.len() && e.prev == prev
                })
            });
            let mark = if kept {
                for row in line {
                    if let Some(e) = self.rows.get_mut(&row.id) {
                        e.seen = self.epoch;
                    }
                }
                self.rows[&line[0].id].mark
            } else {
                self.scanned += 1;
                self.scan_line(line, prev)
            };
            prev = Some(mark);
            start = end + 1;
        }
        let epoch = self.epoch;
        self.rows.retain(|_, e| e.seen == epoch);
    }

    /// A row's classes, a column each; empty when it has none.
    pub(crate) fn classes(&self, id: RowId) -> &[u8] {
        self.rows.get(&id).map_or(&[], |e| &e.classes)
    }

    fn scan_line(&mut self, line: &[SnapshotRow], prev: Option<LineMark>) -> LineMark {
        let (chars, text, at) = (&mut self.chars, &mut self.text, &mut self.at);
        chars.clear();
        text.clear();
        at.clear();
        let (mut head, mut input_at, mut any) = (Semantic::None, None, false);
        for (r, row) in line.iter().enumerate() {
            for (col, cell) in row.cells.iter().enumerate() {
                if cell.width.is_spacer() {
                    continue;
                }
                if head == Semantic::None {
                    head = cell.semantic;
                }
                if cell.semantic == Semantic::Input && input_at.is_none() {
                    input_at = Some(chars.len());
                }
                let mut push = |c: char| {
                    chars.push(c);
                    text.push(c);
                    at.push((r, col));
                };
                match cell.content {
                    SnapshotContent::Scalar(c) => {
                        any |= c != ' ' && plain(row.style_of(cell));
                        push(c);
                    }
                    SnapshotContent::Cluster { start, len } => {
                        row.cluster(start, len).iter().for_each(|&c| push(c))
                    }
                }
            }
        }
        // The blanks at the end mean nothing, and every matcher would walk them.
        let end = chars.iter().rposition(|c| !c.is_whitespace()).map_or(0, |p| p + 1);
        chars.truncate(end);
        at.truncate(end);
        text.truncate(text.trim_end().len());
        let mark = LineMark { head, closed: head == Semantic::Prompt && input_at.is_some() };
        let role = match head {
            Semantic::Prompt if prev.is_some_and(|p: LineMark| p.head != Semantic::Prompt || p.closed) => {
                Some(Role::Prompt)
            }
            Semantic::Output => Some(Role::Output),
            _ => None,
        };
        let mut classes: Vec<Vec<u8>> = vec![Vec::new(); line.len()];
        if any {
            scan::scan(chars, text, role, input_at.filter(|_| role == Some(Role::Prompt)), &mut self.out);
            for (&class, &(r, col)) in self.out.iter().zip(at.iter()) {
                if class != 0 {
                    let row = &mut classes[r];
                    row.resize(line[r].cells.len(), 0);
                    row[col] = class;
                }
            }
        }
        for (row, classes) in line.iter().zip(classes) {
            let entry = Entry {
                seq: row.seq,
                head: line[0].id,
                rows: line.len(),
                prev,
                mark,
                role,
                classes,
                seen: self.epoch,
            };
            self.rows.insert(row.id, entry);
        }
        mark
    }

    /// The rows of the prompt whose command ended last, and the class its sign takes: the
    /// prompt line above the newest one, success for a status of 0, error otherwise.
    pub(crate) fn tinted(&self, rows: &[SnapshotRow], status: i32) -> Option<(Range<usize>, u8)> {
        let prompt = |r: usize| self.rows.get(&rows[r].id).is_some_and(|e| e.role == Some(Role::Prompt));
        let (mut before, mut last, mut r) = (None, None, 0);
        while r < rows.len() {
            if !prompt(r) {
                r += 1;
                continue;
            }
            let start = r;
            while rows[r].wrapped && r + 1 < rows.len() && prompt(r + 1) {
                r += 1;
            }
            r += 1;
            before = last.replace(start..r);
        }
        let class = if status == 0 { Highlight::Success } else { Highlight::Error };
        before.map(|rows| (rows, class as u8))
    }
}
