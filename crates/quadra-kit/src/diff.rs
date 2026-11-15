//! The diff (specs/13, "Components"): what differs between two texts, line by line, as one
//! column or as two halves side by side.

mod compare;
mod lines;
mod rows;
mod select;
mod view;
mod words;

use std::ops::Range;
use std::rc::Rc;

use quadra_platform::Clipboard;
use quadra_reactive::{Signal, effect, memo, signal, untrack, use_context};
use quadra_tree::{NodeId, Token};
use quadra_ui::{IntoProp, MountCx, Prop, View, by_text, column, div, dynamic, uniform_list, virtual_list};

use crate::{fonts, token};
pub use lines::{Step, diff_ids, diff_lines};
pub use rows::DiffMode;
use rows::rows;
use view::build;

/// The height of a row, and the width of a letter of the mono face at 12 px.
const ROW: f32 = 18.0;
const LETTER: f32 = 7.2;

/// The colours of the lines of a text: for each line, parts of it and the role of each.
pub type LineColours = Vec<Vec<(Range<usize>, Token)>>;

type Colourer = Rc<dyn Fn(&str) -> LineColours>;

pub struct Diff {
    old: Prop<String>,
    new: Prop<String>,
    mode: Prop<DiffMode>,
    context: Option<usize>,
    colours: Option<Colourer>,
    wrap: bool,
}

/// What differs between two texts, line by line. Give it a height, or let it grow.
pub fn diff<A, B>(old: impl IntoProp<String, A>, new: impl IntoProp<String, B>) -> Diff {
    let mode = Prop::Value(DiffMode::Inline);
    Diff { old: old.into_prop(), new: new.into_prop(), mode, context: None, colours: None, wrap: false }
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

    /// Colours the lines of both texts: `f` is given a whole text and gives the colours of
    /// each of its lines (`quadra_code::line_colours(path)` colours code).
    pub fn colours(mut self, f: impl Fn(&str) -> LineColours + 'static) -> Self {
        self.colours = Some(Rc::new(f));
        self
    }

    /// Long lines wrap at the width of their column, and a row is as high as its lines:
    /// nothing scrolls sideways.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
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

impl Sides {
    /// The most characters a line of either text has, a tab four.
    fn widest(&self) -> usize {
        let chars = |line: &str| line.chars().count() + 3 * line.matches('\t').count();
        let of = |lines: &Lines| (0..lines.at.len()).map(|i| chars(lines.line(i))).max().unwrap_or(0);
        of(&self.old).max(of(&self.new))
    }
}

/// The two texts and the script from the one to the other.
#[derive(PartialEq)]
struct Sides {
    old: Lines,
    new: Lines,
    script: Vec<Step>,
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
        let sides = compare::compare(old, new);
        let colourer = self.colours;
        let painted = memo(move || {
            Rc::new(colourer.as_ref().map(|f| sides.with(|s| (f(&s.old.text), f(&s.new.text)))))
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
            // The same line edited: enough of it is left as it was (`changed_parts`).
            let alike =
                |i: usize, j: usize| words::changed_parts(sides.old.line(i), sides.new.line(j)).is_some();
            Rc::new(open.with(|open| rows(&sides.script, mode, context, open, &alike)))
        });
        // Room for the number of the last line, and of two digits at least.
        let letter = by_text(LETTER);
        let wide = letter.clone();
        let gutter = memo(move || {
            let digits = sides.with(|s| s.old.at.len().max(s.new.at.len())).to_string().len().max(2);
            digits as f32 * wide() + 16.0
        });
        let widest = memo(move || sides.with(|s| s.widest()) as f32 * letter());
        let (sx, sheet) = (signal(0.0), select::sheet_of(sides, listed));
        let (rows_of, shown, wrap) = (sheet.clone(), mode.clone(), self.wrap);
        let count = move || listed.with(|rows| rows.len());
        let each_row = move |place: Signal<usize>| {
            let mode = mode.clone();
            let built = dynamic(move || {
                let (sides, rows, painted) = (sides.get(), listed.get(), painted.get());
                let how = (mode(), gutter.get(), painted.as_ref().as_ref());
                let row = rows.get(place.get()).map(|at| build(&sides, *at, how, (open, sx, wrap)));
                row.unwrap_or_else(div).grow(1.0)
            });
            if wrap {
                // As high as its lines, which wrap: nothing to scroll sideways.
                let face = crate::rows::selectable(&rows_of, div().min_height(by_text(ROW)), place);
                return built.container(face);
            }
            let shown = shown.clone();
            // How far the lines go past what shows of them, in a row so wide.
            let end = move |width: f32| {
                let halves = if shown() == DiffMode::SideBySide { 2.0 } else { 1.0 };
                let numbers = if halves == 2.0 { 1.0 } else { 2.0 };
                let (widest, gutter) = untrack(|| (widest.get(), gutter.get()));
                widest + 8.0 - (width / halves - numbers * gutter - 16.0)
            };
            let face = crate::rows::selectable(&rows_of, div().height(by_text(ROW)), place);
            built.container(crate::rows::sideways(face, sx, end))
        };
        let list = match wrap {
            true => virtual_list(count, ROW, signal(0.0), each_row),
            false => uniform_list(count, ROW, signal(0.0), each_row),
        };
        let list = list.reveal(move || sheet.cursor.get());
        let frame = column().grow(1.0).bg(token::CANVAS).font_family(fonts::MONO).text_size(12.0);
        let frame = select::frame(&sheet, frame.line_height(1.5), listed, use_context::<Rc<dyn Clipboard>>());
        frame.child(list.container(div().grow(1.0))).mount(cx)
    }
}
