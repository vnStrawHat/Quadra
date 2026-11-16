//! A text deep in auto-sized rows, like a status bar's clock: when it changes to a text of the
//! same size, its layout costs the same whatever the size of the window (specs/06, "Measuring
//! leaves"); when its size changes, the layout is still the one from scratch.

mod common;

use common::{Exact, Text, fill, node};
use quadra_layout::{
    Align, Direction, Holds, Justify, Known, LayoutId, LayoutTree, Length, Measure, Stats, Style,
};
use quadra_prim::{Edges, Size};

fn row(gap: f32) -> Style {
    Style { align_items: Align::Center, gap_col: gap, ..Style::default() }
}

/// A 3200 x 700 window, wide enough that no item of the bar shrinks: a body of `rows` rows of texts, over a bar 30 px high with a border on
/// top and padding, holding `rows` groups of three items; an item is a padded row around a
/// row around a text. The clock is the last group's last text, 19 characters.
struct Window {
    t: LayoutTree,
    text: Text,
    root: LayoutId,
    clock: LayoutId,
    /// A text in an item 100 px wide: it wraps.
    note: LayoutId,
}

fn window(rows: usize) -> Window {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let mut lines = Vec::new();
    for _ in 0..rows {
        let texts: Vec<_> = (0..3).map(|i| text.leaf(&mut t, 10 + i)).collect();
        lines.push(node(&mut t, row(8.0), &texts));
    }
    let note = text.leaf(&mut t, 30);
    lines.push(node(&mut t, Style { width: Length::Px(100.0), ..Style::default() }, &[note]));
    let column = Style { direction: Direction::Column, grow: 1.0, ..Style::default() };
    let body = node(&mut t, column, &lines);
    let (mut groups, mut clock) = (Vec::new(), None);
    for _ in 0..rows {
        let mut items = Vec::new();
        for chars in [4, 6, 19] {
            let leaf = text.leaf(&mut t, chars);
            clock = Some(leaf);
            let wrapper = node(&mut t, Style::default(), &[leaf]);
            items.push(node(&mut t, Style { padding: Edges::xy(4.0, 2.0), ..row(0.0) }, &[wrapper]));
        }
        groups.push(node(&mut t, row(12.0), &items));
    }
    let border = Edges { top: 1.0, ..Edges::default() };
    let bar = Style { height: Length::Px(30.0), border, padding: Edges::xy(8.0, 0.0), ..row(0.0) };
    let bar = node(&mut t, Style { justify: Justify::Between, ..bar }, &groups);
    let root = node(&mut t, Style { direction: Direction::Column, ..Style::default() }, &[body, bar]);
    let clock = clock.expect("a group");
    let mut w = Window { t, text, root, clock, note };
    w.layout();
    w
}

impl Window {
    /// Lays out what changed, checks it against a layout from scratch that measures every
    /// request as asked, and returns what it cost.
    fn layout(&mut self) -> Stats {
        self.t.layout(self.root, fill(3200.0, 700.0), &mut self.text);
        assert_eq!(self.t.verify(&mut Exact(&mut self.text)), None);
        self.t.take_stats()
    }

    fn set(&mut self, leaf: LayoutId, chars: usize) -> Stats {
        self.text.chars.insert(leaf, chars);
        self.t.mark_dirty(leaf);
        self.layout()
    }
}

#[test]
fn a_clock_whose_size_does_not_change_costs_the_same_in_any_window() {
    let costs: Vec<Stats> = [1, 10]
        .into_iter()
        .map(|rows| {
            let mut w = window(rows);
            let clock = w.clock;
            let stats = w.set(clock, 19); // the next second: another text, as wide
            assert_eq!(stats.climbed, 0, "{rows} rows: {stats:?}");
            assert!(stats.asked <= 4 && stats.measured <= 4, "{rows} rows: {stats:?}");
            stats
        })
        .collect();
    assert_eq!(costs[0], costs[1]);
}

#[test]
fn a_clock_that_grows_or_a_text_that_wraps_is_laid_out_as_from_scratch() {
    let mut w = window(3);
    let (clock, note) = (w.clock, w.note);
    let before = w.t.rect(clock);
    w.set(clock, 25); // checked against a layout from scratch
    assert_eq!(w.t.rect(clock).width, before.width + 6.0 * 8.0);
    w.set(clock, 19);
    assert_eq!(w.t.rect(clock), before);

    assert_eq!(w.t.rect(note).height, 3.0 * 16.0); // 30 characters, 12 a line
    let stats = w.set(note, 30);
    assert!(stats.climbed == 0 && stats.asked <= 4, "{stats:?}");
    w.set(note, 40);
    assert_eq!(w.t.rect(note).height, 4.0 * 16.0);
    w.set(note, 5);
    assert_eq!(w.t.rect(note).height, 16.0);
}

/// A text that is two lines of 3 characters, then one of 7 ("ab\ncd", then "ab cd"): as
/// high and as wide as before where 7 do not fit, wider where they do.
#[derive(Default)]
struct Swap {
    joined: bool,
    holds: (f32, f32),
}

impl Measure for Swap {
    fn measure(&mut self, _: LayoutId, known: Known, avail: Size) -> Size {
        let limit = known.width.unwrap_or(avail.width);
        let (size, holds) = match self.joined && limit >= 56.0 {
            true => (Size::new(56.0, 16.0), (56.0, f32::INFINITY)),
            false if self.joined => (Size::new(24.0, 32.0), (24.0, limit)),
            false => (Size::new(24.0, 32.0), (24.0, f32::INFINITY)),
        };
        self.holds = holds;
        size
    }

    fn holds(&mut self, _: LayoutId) -> Option<Holds> {
        Some(Holds { width: self.holds, any_height: true })
    }
}

#[test]
fn a_size_asked_at_one_width_is_the_same_only_if_it_holds_for_as_much() {
    let mut t = LayoutTree::new();
    let leaf = t.insert_leaf(Style::default());
    let root = node(&mut t, Style::default(), &[leaf]);
    let mut text = Swap::default();
    t.layout(root, fill(32.0, 100.0), &mut text);
    t.layout(root, fill(800.0, 100.0), &mut text); // its size at 32 holds at 800: not measured
    text.joined = true;
    t.mark_dirty(leaf);
    t.layout(root, fill(800.0, 100.0), &mut text);
    assert_eq!(t.rect(leaf).width, 56.0); // as wide as at 32, but no longer at 800
    struct Exact<'a>(&'a mut Swap);
    impl Measure for Exact<'_> {
        fn measure(&mut self, node: LayoutId, known: Known, avail: Size) -> Size {
            self.0.measure(node, known, avail)
        }
    }
    assert_eq!(t.verify(&mut Exact(&mut text)), None);
}
