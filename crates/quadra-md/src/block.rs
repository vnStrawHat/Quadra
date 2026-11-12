//! The blocks of a document (specs/13, "Markdown"; behaviour/21): the part of CommonMark that
//! notes, READMEs and docs use. Headings, paragraphs, lists (nested by their indent) with
//! tasks, fenced code, quotes (nested), rules, pipe tables, display math and a front matter.
//! Comments show nothing. Not handled: headings underlined, code by indent, other HTML,
//! links by reference, footnotes.

use crate::lines::{cell_align, is_rule, is_table_rule, list_item, quote_parts, table_cells, task_parts};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

/// What a block is, with what it has. A `text` is the source of inlines
/// ([`crate::parse_inline`]), but that of code, math and the front matter, which is as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Paragraph {
        text: String,
    },
    /// `level` is 1 to 6.
    Heading {
        level: u8,
        text: String,
    },
    /// An item of a list: a numbered one has its number, a task whether it is done. `depth`
    /// is how far in it is nested, from 0: half its indent.
    Item {
        number: Option<u32>,
        depth: usize,
        task: Option<bool>,
        text: String,
    },
    /// `lang` is the first word after the fence, in lower case.
    Code {
        lang: String,
        text: String,
    },
    /// `depth` is how many `>` it has, from 1.
    Quote {
        depth: usize,
        text: String,
    },
    Rule,
    /// The cells of each row, the head's first, and how each column is aligned.
    Table {
        align: Vec<Align>,
        rows: Vec<Vec<String>>,
    },
    /// `$$ .. $$` or a fence of `math`: the source of a formula.
    Math {
        text: String,
    },
    /// The front matter: what is between the `---` a document starts with.
    Front {
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The source line it starts on, from 0.
    pub line: usize,
    pub kind: Kind,
}

impl Block {
    /// The source of its inlines, when it has any: a paragraph, a heading, an item, a quote.
    pub fn inline(&self) -> Option<&str> {
        match &self.kind {
            Kind::Paragraph { text } | Kind::Heading { text, .. } => Some(text),
            Kind::Item { text, .. } | Kind::Quote { text, .. } => Some(text),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Parser<'a> {
    out: Vec<Block>,
    /// The block lines are still added to: what it is, its lines, the line it starts on.
    open: Option<(Kind, Vec<&'a str>, usize)>,
}

impl<'a> Parser<'a> {
    fn flush(&mut self) {
        let Some((mut kind, lines, line)) = self.open.take() else { return };
        if let Kind::Paragraph { text }
        | Kind::Heading { text, .. }
        | Kind::Item { text, .. }
        | Kind::Quote { text, .. } = &mut kind
        {
            *text = join_lines(&lines);
        }
        self.out.push(Block { line, kind });
    }

    fn start(&mut self, kind: Kind, first: &'a str, line: usize) {
        self.flush();
        self.open = Some((kind, vec![first], line));
    }

    fn push(&mut self, kind: Kind, line: usize) {
        self.flush();
        self.out.push(Block { line, kind });
    }
}

/// The lines of a block as one text, a space between them; a line that ends with two spaces
/// or a backslash breaks there (a newline in the text).
fn join_lines(lines: &[&str]) -> String {
    let mut out = String::new();
    for (k, line) in lines.iter().enumerate() {
        let hard = line.ends_with("  ") || (line.ends_with('\\') && !line.ends_with("\\\\"));
        out.push_str(line.strip_suffix('\\').unwrap_or(line).trim());
        if k + 1 < lines.len() {
            out.push(if hard { '\n' } else { ' ' });
        }
    }
    out
}

pub fn parse(source: &str) -> Vec<Block> {
    let lines: Vec<&str> = source.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line)).collect();
    let (mut p, text) = (Parser::default(), String::new);
    let mut i = front_matter(&lines, &mut p.out);
    while i < lines.len() {
        let (at, line) = (i, lines[i]);
        let (t, rest) = (line.trim(), line.trim_start_matches([' ', '\t']));
        let lead = &line[..line.len() - rest.len()];
        let indent = lead.len() + 3 * lead.matches('\t').count(); // a tab counts as four
        i += 1;
        if let Some(fence) = ["```", "~~~"].into_iter().find(|fence| t.starts_with(fence)) {
            let end = (i..lines.len()).find(|&k| lines[k].trim().starts_with(fence)).unwrap_or(lines.len());
            let (lang, text) = (t[3..].split_whitespace().next().unwrap_or(""), lines[i..end].join("\n"));
            let lang = lang.to_lowercase();
            p.push(if lang == "math" { Kind::Math { text } } else { Kind::Code { lang, text } }, at);
            i = end + 1; // the closing fence
        } else if t.starts_with("<!--") && indent < 4 {
            // A comment runs to the line that has `-->` and shows nothing; what is after
            // `-->` on that line goes with it, as that of a block of HTML would.
            p.flush();
            i = (at..lines.len()).find(|&k| lines[k].contains("-->")).unwrap_or(lines.len()) + 1;
        } else if t.starts_with("$$") {
            p.flush();
            i = display_math(&lines, at, &mut p.out);
        } else if t.is_empty() {
            p.flush();
        } else if let Some(level) = heading(t) {
            p.start(Kind::Heading { level: level as u8, text: text() }, t[level..].trim_end_matches('#'), at);
            p.flush();
        } else if is_rule(t) {
            p.push(Kind::Rule, at);
        } else if t.starts_with('|') && lines.get(i).is_some_and(|next| is_table_rule(next)) {
            let align = table_cells(lines[i]).iter().map(|cell| cell_align(cell)).collect();
            let mut rows = vec![table_cells(t)];
            i += 1;
            while i < lines.len() && lines[i].trim().starts_with('|') {
                rows.push(table_cells(lines[i]));
                i += 1;
            }
            p.push(Kind::Table { align, rows }, at);
        } else if t.starts_with('>') {
            let (depth, said) = quote_parts(t);
            match &mut p.open {
                Some((Kind::Quote { depth: open, .. }, lines, _)) if *open == depth => lines.push(said),
                _ => p.start(Kind::Quote { depth, text: text() }, said, at),
            }
        } else if let Some((number, said)) = list_item(t) {
            let (task, said) = task_parts(said);
            p.start(Kind::Item { number, depth: indent / 2, task, text: text() }, said, at);
        } else {
            // A line more of the paragraph, or of an item or a quote when it is indented
            // (the spaces it ends with are kept: they may break the line).
            match &mut p.open {
                Some((kind, lines, _)) if indent > 0 || matches!(kind, Kind::Paragraph { .. }) => {
                    lines.push(rest);
                }
                _ => p.start(Kind::Paragraph { text: text() }, rest, at),
            }
        }
    }
    p.flush();
    p.out
}

/// The level of a heading: one to six `#`, then a space or the end of the line.
fn heading(t: &str) -> Option<usize> {
    let level = t.len() - t.trim_start_matches('#').len();
    ((1..=6).contains(&level) && (t.len() == level || t.as_bytes()[level] == b' ')).then_some(level)
}

/// Takes the `---` .. `---` a document starts with; gives the line after it.
fn front_matter(lines: &[&str], out: &mut Vec<Block>) -> usize {
    if lines.len() < 2 || lines[0].trim() != "---" {
        return 0;
    }
    let Some(end) = (1..lines.len()).find(|&k| matches!(lines[k].trim(), "---" | "...")) else { return 0 };
    out.push(Block { line: 0, kind: Kind::Front { text: lines[1..end].join("\n") } });
    end + 1
}

/// Reads `$$ .. $$` from the line `at`, on one line or on several; gives the line after it.
fn display_math(lines: &[&str], at: usize, out: &mut Vec<Block>) -> usize {
    let first = lines[at].trim()[2..].trim();
    let mut math =
        |text: &str| out.push(Block { line: at, kind: Kind::Math { text: text.trim().to_string() } });
    if let Some(all) = first.strip_suffix("$$") {
        math(all);
        return at + 1;
    }
    let (mut body, mut i) = (vec![first], at + 1);
    while i < lines.len() {
        let t = lines[i].trim();
        i += 1;
        if let Some(last) = t.strip_suffix("$$") {
            body.push(last);
            break;
        }
        body.push(lines[i - 1]);
    }
    math(&body.join("\n"));
    i
}
