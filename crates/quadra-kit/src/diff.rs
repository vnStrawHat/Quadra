//! The diff (specs/13, "Components"): what differs between two texts, line by line, as one
//! column or as two halves side by side.

mod lines;
mod rows;

use std::ops::Range;
use std::rc::Rc;

use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_reactive::{Signal, effect, memo, signal};
use quadra_tree::{NodeId, Role};
use quadra_ui::{
    Div, IntoProp, MountCx, Prop, View, column, div, dynamic, rich_text, row, span, text, uniform_list,
};

use crate::{fonts, token};
use lines::changed_part;
pub use lines::{Step, diff_lines};
pub use rows::DiffMode;
use rows::{Row, rows};

/// The height of a row, and the width of a letter of the mono face at 12 px.
const ROW: f32 = 18.0;
const LETTER: f32 = 7.2;

pub struct Diff {
    old: Prop<String>,
    new: Prop<String>,
    mode: Prop<DiffMode>,
    context: Option<usize>,
}

/// What differs between two texts, line by line. Give it a height, or let it grow.
pub fn diff<A, B>(old: impl IntoProp<String, A>, new: impl IntoProp<String, B>) -> Diff {
    let mode = Prop::Value(DiffMode::Inline);
    Diff { old: old.into_prop(), new: new.into_prop(), mode, context: None }
}

impl Diff {
    /// One column, or the old text beside the new.
    pub fn mode<M>(mut self, mode: impl IntoProp<DiffMode, M>) -> Self {
        self.mode = mode.into_prop();
        self
    }

    /// Folds the lines alike that are more than `lines` from a change into one row, which
    /// shows them when it is clicked.
    pub fn context(mut self, lines: usize) -> Self {
        self.context = Some(lines);
        self
    }
}

/// A text, and where each of its lines is in it.
#[derive(PartialEq)]
struct Lines {
    text: String,
    at: Vec<Range<usize>>,
}

impl Lines {
    fn new(text: String) -> Lines {
        let mut from = 0;
        let line = |line: &str| {
            let start = from;
            from += line.len();
            start..start + line.trim_end_matches(['\n', '\r']).len()
        };
        let at = text.split_inclusive('\n').map(line).collect();
        Lines { text, at }
    }

    fn line(&self, i: usize) -> &str {
        self.at.get(i).map_or("", |at| &self.text[at.clone()])
    }

    fn all(&self) -> Vec<&str> {
        self.at.iter().map(|at| &self.text[at.clone()]).collect()
    }
}

/// The two texts and the script from the one to the other.
#[derive(PartialEq)]
struct Sides {
    old: Lines,
    new: Lines,
    script: Vec<Step>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Same,
    Removed,
    Added,
}

/// A line on its row: its numbers, its mark and what it says, on the tint of what happened
/// to it. `other` is the line it is paired with: what is not common to both is tinted more.
fn line(numbers: &[Option<usize>], gutter: f32, mark: Mark, said: &str, other: Option<&str>) -> Div {
    let (sign, tint, name) = match mark {
        Mark::Same => (" ", None, said.to_string()),
        Mark::Removed => ("-", Some((token::DIFF_REMOVE, token::ERROR)), format!("Removed: {said}")),
        Mark::Added => ("+", Some((token::DIFF_ADD, token::OK)), format!("Added: {said}")),
    };
    // ponytail: a tab is four spaces; stops when the editor says where they are.
    let shown = |line: &str| line.replace('\t', "    ");
    let (said, other) = (shown(said), other.map(shown));
    let part = other.and_then(|other| match mark {
        Mark::Removed => changed_part(&said, &other).map(|(part, _)| part),
        Mark::Added => changed_part(&other, &said).map(|(_, part)| part),
        Mark::Same => None,
    });
    let part = part.unwrap_or(0..0);
    let before = span(&said[..part.start]);
    let changed = span(&said[part.clone()]).background(tint.map_or(token::CANVAS, |(_, ink)| ink).alpha(0.3));
    let spans =
        [(before, part.start), (changed, part.len()), (span(&said[part.end..]), said.len() - part.end)];
    let spans: Vec<_> = spans.into_iter().filter(|(_, len)| *len > 0).map(|(span, _)| span).collect();

    let number = |n: &Option<usize>| {
        let cell = row().width(gutter).shrink(0.0).justify(Justify::End).padding_right(8.0);
        let cell = cell.text_color(token::TEXT_3);
        match n {
            Some(n) => cell.child(text((n + 1).to_string())),
            None => cell,
        }
    };
    let line = row().height(ROW).align_items(Align::Center).role(Role::ListItem).label(name);
    let line = line.children(numbers.iter().map(number).collect::<Vec<_>>());
    let sign = div().width(16.0).shrink(0.0).child(text(sign));
    let line = match tint {
        Some((tint, ink)) => line.bg(tint).child(sign.text_color(ink)),
        None => line.child(sign),
    };
    // It does not shrink: a long line is cut at the edge, not wrapped.
    if spans.is_empty() { line } else { line.child(div().shrink(0.0).child(rich_text(spans))) }
}

/// The old text's half of a row beside the new one's, as wide as each other. The right
/// half is not seen through: what a long line at the left has past its half is under it.
fn halves(left: Div, right: Div) -> Div {
    let right = row().bg(token::CANVAS).border_left(1.0).border_color(token::LINE).child(right.grow(1.0));
    row().height(ROW).child(left.width(0.0).grow(1.0)).child(right.width(0.0).grow(1.0))
}

/// The row of lines that do not show: a button that shows them.
fn fold(old: usize, count: usize, open: Signal<Vec<usize>>) -> Div {
    let show = move || open.update(|open| open.push(old));
    let fold = row().height(ROW).align_items(Align::Center).justify(Justify::Center);
    let fold = fold.bg(token::PANEL).text_color(token::TEXT_3).hover(|s| s.bg(token::PANEL_HOVER));
    let fold = fold.cursor(Cursor::Pointer).focus_visible(|s| s.outline(token::BORDER_FOCUSED));
    quadra_base::press::press(fold.child(text(format!("{count} unchanged lines")))).on_click(show).build()
}

/// What a row of the listing shows.
fn build(sides: &Sides, at: Row, mode: DiffMode, gutter: f32, open: Signal<Vec<usize>>) -> Div {
    let (was, is) = (|i: usize| sides.old.line(i), |j: usize| sides.new.line(j));
    let one =
        |n: usize, mark: Mark, said: &str, other: Option<&str>| line(&[Some(n)], gutter, mark, said, other);
    let none = || row().height(ROW).bg(token::PANEL);
    match (at, mode) {
        (Row::Same { old, new }, DiffMode::Inline) => {
            line(&[Some(old), Some(new)], gutter, Mark::Same, was(old), None)
        }
        (Row::Same { old, new }, DiffMode::SideBySide) => {
            halves(one(old, Mark::Same, was(old), None), one(new, Mark::Same, is(new), None))
        }
        (Row::Removed { old, with }, _) => {
            line(&[Some(old), None], gutter, Mark::Removed, was(old), with.map(is))
        }
        (Row::Added { new, with }, _) => {
            line(&[None, Some(new)], gutter, Mark::Added, is(new), with.map(was))
        }
        (Row::Pair { old, new }, _) => halves(
            old.map_or_else(none, |i| one(i, Mark::Removed, was(i), new.map(is))),
            new.map_or_else(none, |j| one(j, Mark::Added, is(j), old.map(was))),
        ),
        (Row::Fold { old, count }, _) => fold(old, count, open),
    }
}

/// What reads a property, whichever way it was given.
fn reader<T: Clone + 'static>(prop: Prop<T>) -> Rc<dyn Fn() -> T> {
    match prop {
        Prop::Value(value) => Rc::new(move || value.clone()),
        Prop::Reactive(read) => Rc::from(read),
    }
}

impl View for Diff {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (old, new, mode, context) = (reader(self.old), reader(self.new), reader(self.mode), self.context);
        let sides = memo(move || {
            let (old, new) = (Lines::new(old()), Lines::new(new()));
            let script = diff_lines(&old.all(), &new.all());
            Rc::new(Sides { old, new, script })
        });
        // The folds that were opened, by their first line; other texts close them.
        let open = signal(Vec::new());
        effect(move || {
            sides.get();
            open.set(Vec::new());
        });
        let layout = mode.clone();
        let listed = memo(move || {
            let (sides, mode) = (sides.get(), layout());
            Rc::new(open.with(|open| rows(&sides.script, mode, context, open)))
        });
        let list = uniform_list(
            move || listed.with(|rows| rows.len()),
            ROW,
            signal(0.0),
            move |place| {
                let mode = mode.clone();
                let built = dynamic(move || {
                    let (sides, rows) = (sides.get(), listed.get());
                    // Room for the number of the last line, and of two digits at least.
                    let digits = sides.old.at.len().max(sides.new.at.len()).to_string().len().max(2);
                    let gutter = digits as f32 * LETTER + 16.0;
                    let row = rows.get(place.get()).map(|at| build(&sides, *at, mode(), gutter, open));
                    row.unwrap_or_else(div).grow(1.0)
                });
                built.container(div().height(ROW))
            },
        );
        let viewport = div().grow(1.0).role(Role::List).label("Diff".to_string());
        let frame = column().grow(1.0).bg(token::CANVAS).font_family(fonts::MONO);
        frame.text_size(12.0).line_height(1.5).child(list.container(viewport)).mount(cx)
    }
}
