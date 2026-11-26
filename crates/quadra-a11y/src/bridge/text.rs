//! The text of an input for a reader: runs of it, a line each when the input says where
//! its characters are, which its caret, its selection and the bounds of a range are places in.

use std::collections::HashMap;

use crate::{Role, SemanticNode};
use accesskit::{Node, NodeId, TextPosition, TextSelection};

use super::rect;

/// How many lines an input is told in, a run each: past them, one run with no places.
const MOST: usize = 2048;

/// What a run is, by its id: the input it is of, and its first character in the value.
pub type Runs = HashMap<u64, (u64, usize)>;

/// The run `k` of an input: the input's id with the top bit set and `k` above the low bits
/// of its generation, which no node of a tree has (two nodes alive at once have other
/// indices).
fn run_id(id: u64, k: usize) -> NodeId {
    NodeId(1 << 63 | (k as u64) << 52 | (id & ((1 << 52) - 1)))
}

/// Where the runs of an input start, in characters of its value: one at 0 when it says no
/// lines.
fn starts(from: &SemanticNode) -> Vec<usize> {
    let lines = &from.semantics.lines;
    if lines.is_empty() || lines.len() > MOST {
        vec![0]
    } else {
        lines.iter().map(|line| line.start).collect()
    }
}

/// The value of an input, if it is one.
fn value(from: &SemanticNode) -> Option<&str> {
    let says = &from.semantics;
    says.value.as_deref().filter(|_| says.role == Role::TextInput)
}

/// The runs of an input's text, with what they are noted in `runs`; none for another node.
pub(super) fn runs(from: &SemanticNode, runs: &mut Runs) -> Vec<(NodeId, Node)> {
    let Some(value) = value(from) else { return Vec::new() };
    let says = &from.semantics;
    let bytes: Vec<usize> = value.char_indices().map(|(at, _)| at).chain([value.len()]).collect();
    let starts = starts(from);
    let placed = starts.len() == says.lines.len();
    let mut out = Vec::new();
    for (k, &start) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(bytes.len() - 1);
        let text = &value[bytes[start.min(end)]..bytes[end]];
        let mut run = Node::new(accesskit::Role::TextRun);
        run.set_value(text);
        run.set_character_lengths(text.chars().map(|c| c.len_utf8() as u8).collect::<Vec<_>>());
        match says.lines.get(k).filter(|line| placed && line.positions.len() == end - start) {
            Some(line) => {
                run.set_character_positions(line.positions.clone());
                run.set_character_widths(line.widths.clone());
                run.set_bounds(rect(line.bounds.translate(from.bounds.x, from.bounds.y)));
            }
            None => run.set_bounds(rect(from.bounds)),
        }
        let id = run_id(from.id, k);
        runs.insert(id.0, (from.id, start));
        out.push((id, run));
    }
    out
}

/// The ids of an input's runs, and where its selection is in them.
pub(super) fn children_and_selection(from: &SemanticNode) -> Option<(Vec<NodeId>, TextSelection)> {
    let count = value(from)?.chars().count();
    let starts = starts(from);
    let ids = (0..starts.len()).map(|k| run_id(from.id, k)).collect();
    let at = |index: usize| {
        let index = index.min(count);
        let k = starts.partition_point(|&start| start <= index).saturating_sub(1);
        TextPosition { node: run_id(from.id, k), character_index: index - starts[k] }
    };
    let (anchor, focus) = from.semantics.selection.unwrap_or((count, count));
    Some((ids, TextSelection { anchor: at(anchor), focus: at(focus) }))
}

/// The node a request is of, and the character a place in a run is of the input's value.
pub(super) fn of(runs: &Runs, node: NodeId, index: usize) -> (u64, usize) {
    match runs.get(&node.0) {
        Some(&(input, start)) => (input, start + index),
        None => (node.0 & !(1 << 63), index),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Semantics, TextLine};
    use quadra_prim::Rect;

    use super::*;

    #[test]
    fn an_input_of_two_lines_is_two_runs_with_the_places_of_their_characters() {
        let line = |start: usize, y: f32, n: usize| TextLine {
            start,
            bounds: Rect::new(0.0, y, 6.0 * n as f32, 14.0),
            positions: (0..n).map(|i| i as f32 * 6.0).collect(),
            widths: vec![6.0; n],
        };
        let semantics = Semantics {
            role: Role::TextInput,
            value: Some("ab\ncd".into()),
            selection: Some((1, 4)),
            lines: vec![line(0, 0.0, 3), line(3, 14.0, 2)],
            ..Semantics::default()
        };
        let input = SemanticNode {
            id: 7,
            semantics,
            bounds: Rect::new(10.0, 20.0, 100.0, 28.0),
            focused: true,
            focusable: true,
            children: Vec::new(),
        };
        let mut noted = Runs::new();
        let made = runs(&input, &mut noted);
        assert_eq!(made.len(), 2);
        assert_eq!((made[0].1.value(), made[1].1.value()), (Some("ab\n"), Some("cd")));
        assert_eq!(made[1].1.character_positions(), Some(&[0.0, 6.0][..]));
        let bounds = made[1].1.bounds().unwrap();
        assert_eq!((bounds.x0, bounds.y0, bounds.x1), (10.0, 34.0, 22.0));
        // The selection from the second character to the fifth: in the first run, then in
        // the second; and a place in the second run is of the input, three characters on.
        let (ids, chosen) = children_and_selection(&input).unwrap();
        assert_eq!(ids, [made[0].0, made[1].0]);
        assert_eq!((chosen.anchor.node, chosen.anchor.character_index), (made[0].0, 1));
        assert_eq!((chosen.focus.node, chosen.focus.character_index), (made[1].0, 1));
        assert_eq!(of(&noted, made[1].0, 1), (7, 4));
        assert_eq!(of(&noted, NodeId(7), 2), (7, 2));
    }
}
