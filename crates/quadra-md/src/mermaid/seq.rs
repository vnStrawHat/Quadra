//! Sequence diagrams as they are written: participants and actors, messages, notes, frames
//! (loop, alt with else, opt, par with and, critical with option, break) and autonumber.
//! What activates, boxes and titles are read and not drawn.

use std::collections::HashMap;

use super::draw::{Head, decode};
use super::scan::shown;

pub(super) mod layout;
#[cfg(test)]
mod tests;

/// What a diagram may hold: one past these is a fault, shown as its source.
pub const MOST_ACTORS: usize = 200;
pub const MOST_ITEMS: usize = 5000;

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub label: String,
    /// Drawn as a figure, not as a box.
    pub figure: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Over,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Message {
        from: usize,
        to: usize,
        text: String,
        dashed: bool,
        head: Head,
    },
    /// A note beside a participant, or over those from `from` to `to`, which is not before it.
    Note {
        from: usize,
        to: usize,
        text: String,
        side: Side,
    },
    /// A frame starts: its word, and what it says.
    Open {
        word: &'static str,
        text: String,
    },
    /// The next part of a frame: else, and, option.
    Branch {
        text: String,
    },
    Close,
}

#[derive(Default)]
pub struct Seq {
    pub actors: Vec<Actor>,
    ids: HashMap<String, usize>,
    pub items: Vec<Item>,
    /// Messages are numbered.
    pub autonumber: bool,
}

/// The arrows of messages and the heads they end with.
const ARROWS: [(&str, Head); 8] = [
    ("-->>", Head::Arrow),
    ("->>", Head::Arrow),
    ("--x", Head::Cross),
    ("-x", Head::Cross),
    ("--)", Head::Open),
    ("-)", Head::Open),
    ("-->", Head::None),
    ("->", Head::None),
];
const FRAMES: [&str; 6] = ["loop", "alt", "opt", "par", "critical", "break"];
const BRANCHES: [&str; 3] = ["else", "and", "option"];
const SILENT: [&str; 3] = ["activate", "deactivate", "title"];
const SIDES: [(&str, Side); 3] = [("left of", Side::Left), ("right of", Side::Right), ("over", Side::Over)];

/// The sequence diagram of the lines after its header; `first` is the number of the header's
/// line, for what a fault says.
pub fn parse(lines: &[&str], first: usize) -> Result<Seq, String> {
    let mut seq = Seq::default();
    // For each block that is open, whether it is a frame that is drawn.
    let mut open: Vec<bool> = Vec::new();
    for (n, raw) in lines.iter().enumerate() {
        let t = raw.trim();
        let (word, rest) = t.split_once(' ').unwrap_or((t, ""));
        let (word, rest) = (word.to_ascii_lowercase(), rest.trim());
        let fault = |fault: String| format!("line {}: {fault}", first + n + 1);
        if t.is_empty() || t.starts_with("%%") || SILENT.contains(&word.as_str()) {
            continue;
        }
        if let Some(frame) = FRAMES.iter().find(|frame| **frame == word) {
            seq.items.push(Item::Open { word: frame, text: decode(rest) });
            open.push(true);
            continue;
        }
        match word.as_str() {
            "autonumber" => seq.autonumber = true,
            "participant" | "actor" => {
                let (id, label) = rest.split_once(" as ").unwrap_or((rest, rest));
                seq.actor(id.trim(), label, word == "actor").map_err(fault)?;
            }
            "note" => seq.note(rest).map_err(fault)?,
            "box" | "rect" => open.push(false),
            "end" if open.pop() == Some(true) => seq.items.push(Item::Close),
            "end" => {}
            _ if BRANCHES.contains(&word.as_str()) => seq.items.push(Item::Branch { text: decode(rest) }),
            _ => seq.message(t).map_err(fault)?,
        }
        if seq.items.len() > MOST_ITEMS {
            return Err(format!("more than {MOST_ITEMS} messages, notes and frames"));
        }
    }
    // A frame that is never closed ends with the diagram.
    seq.items.extend(open.iter().filter(|drawn| **drawn).map(|_| Item::Close));
    Ok(seq)
}

impl Seq {
    /// The participant of a name: a new one when it was not said before, with its label.
    fn actor(&mut self, id: &str, label: &str, figure: bool) -> Result<usize, String> {
        if let Some(at) = self.ids.get(id) {
            return Ok(*at);
        }
        if self.actors.len() >= MOST_ACTORS {
            return Err(format!("more than {MOST_ACTORS} participants"));
        }
        self.ids.insert(id.to_string(), self.actors.len());
        self.actors.push(Actor { label: decode(label), figure });
        Ok(self.actors.len() - 1)
    }

    /// `A->>B: text`. The marks of activation before the receiver, `+` and `-`, are dropped.
    fn message(&mut self, t: &str) -> Result<(), String> {
        // The first arrow of the line, and of those that start there the longest.
        let found = ARROWS.iter().filter_map(|(arrow, head)| t.find(arrow).map(|at| (at, *arrow, *head)));
        let first = found.min_by_key(|(at, arrow, _)| (*at, usize::MAX - arrow.len()));
        let Some((at, arrow, head)) = first.filter(|(at, ..)| *at > 0) else {
            return Err(format!("cannot read `{}`", shown(t)));
        };
        let rest = t[at + arrow.len()..].trim_start_matches(['+', '-', ' ']);
        let (to, text) = rest.split_once(':').unwrap_or((rest, ""));
        let (from, to) = (t[..at].trim(), to.trim());
        if to.is_empty() {
            return Err(format!("a message needs a receiver: `{}`", shown(t)));
        }
        let (from, to) = (self.actor(from, from, false)?, self.actor(to, to, false)?);
        self.items.push(Item::Message {
            from,
            to,
            text: decode(text),
            dashed: arrow.starts_with("--"),
            head,
        });
        Ok(())
    }

    /// `left of A: text`, `right of A: text`, `over A: text`, `over A,B: text`.
    fn note(&mut self, rest: &str) -> Result<(), String> {
        let (place, text) = rest.split_once(':').unwrap_or((rest, ""));
        let place = place.trim();
        let starts =
            |word: &str| place.get(..word.len()).is_some_and(|start| start.eq_ignore_ascii_case(word));
        let Some((word, side)) = SIDES.iter().find(|(word, _)| starts(word)) else {
            return Err("a note is left of, right of or over participants".to_string());
        };
        let mut who = place[word.len()..].split(',').map(str::trim);
        let a = who.next().unwrap_or("");
        let a = self.actor(a, a, false)?;
        let b = who.next().map_or(Ok(a), |b| self.actor(b, b, false))?;
        self.items.push(Item::Note { from: a.min(b), to: a.max(b), text: decode(text), side: *side });
        Ok(())
    }
}
