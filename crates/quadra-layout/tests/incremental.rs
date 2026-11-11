//! What a change costs, and that the incremental result is always the layout from scratch
//! (specs/06, "What a change costs", Tests).

mod common;

use common::{Text, fill, node, px, rect};
use quadra_layout::{Direction, Display, LayoutId, LayoutTree, Length, RootSize, Style};
use quadra_prim::{Edges, Size};

fn column() -> Style {
    Style { direction: Direction::Column, ..Style::default() }
}

/// A fixed 400 x 600 column holding a card (fixed 200 x 80, one text) and a note (auto height,
/// one text), then a footer.
struct Page {
    t: LayoutTree,
    text: Text,
    root: LayoutId,
    card: LayoutId,
    card_text: LayoutId,
    note_text: LayoutId,
    footer: LayoutId,
}

fn page() -> Page {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let card_text = text.leaf(&mut t, 5);
    let card = node(&mut t, px(200.0, 80.0), &[card_text]);
    let note_text = text.leaf(&mut t, 30);
    let note = node(&mut t, column(), &[note_text]);
    let footer = t.insert(Style { height: Length::Px(20.0), ..Style::default() });
    let root =
        node(&mut t, Style { direction: Direction::Column, ..px(400.0, 600.0) }, &[card, note, footer]);
    t.layout(root, fill(400.0, 600.0), &mut text);
    t.take_stats();
    Page { t, text, root, card, card_text, note_text, footer }
}

impl Page {
    fn layout(&mut self) -> quadra_layout::Stats {
        self.t.layout(self.root, fill(400.0, 600.0), &mut self.text);
        assert_eq!(self.t.verify(&mut self.text), None);
        self.t.take_stats()
    }
}

#[test]
fn nothing_changed_costs_nothing() {
    let mut p = page();
    let stats = p.layout();
    assert_eq!((stats.asked, stats.measured, stats.climbed), (0, 0, 0));
}

#[test]
fn a_change_with_the_same_answers_stops_at_the_node() {
    let mut p = page();
    p.t.mark_dirty(p.note_text); // same text: it answers as before
    let stats = p.layout();
    assert_eq!(stats.climbed, 0);
    assert!(stats.asked <= 3, "{stats:?}");
}

#[test]
fn a_change_inside_a_fixed_box_stops_at_the_box() {
    let mut p = page();
    p.text.chars.insert(p.card_text, 9);
    p.t.mark_dirty(p.card_text);
    let stats = p.layout();
    assert_eq!(stats.climbed, 1, "{stats:?}"); // the card answers as before: the page never runs
    assert_eq!(p.t.rect(p.card_text).width, 72.0); // the card is a row: its text is as wide as its content
}

#[test]
fn a_change_of_size_moves_what_follows() {
    let mut p = page();
    assert_eq!(p.t.rect(p.footer).y, 96.0); // card 80, one line of note
    p.text.chars.insert(p.note_text, 120); // 960 px: three lines at 400
    p.t.mark_dirty(p.note_text);
    let stats = p.layout();
    assert_eq!(p.t.rect(p.footer).y, 80.0 + 48.0);
    assert_eq!(stats.climbed, 2, "{stats:?}"); // the note, then the page, which is the root
}

#[test]
fn padding_of_a_fixed_box_does_not_reach_its_parent_but_a_margin_does() {
    let mut p = page();
    let style = Style { padding: Edges::all(8.0), ..p.t.style(p.card).clone() };
    p.t.set_style(p.card, style.clone());
    assert_eq!(p.layout().climbed, 0);
    assert_eq!(p.t.rect(p.card_text), rect(8.0, 8.0, 40.0, 64.0));
    p.t.set_style(p.card, style.clone()); // equal: nothing at all
    assert_eq!(p.layout().asked, 0);
    p.t.set_style(p.card, Style { margin: Edges::all(4.0), ..style });
    assert_eq!(p.layout().climbed, 1);
    assert_eq!(p.t.rect(p.card).origin(), quadra_prim::Point::new(4.0, 4.0));
}

#[test]
fn children_can_be_added_removed_and_hidden() {
    let mut p = page();
    let extra = p.text.leaf(&mut p.t, 4);
    let mut kids = p.t.children(p.root).to_vec();
    kids.insert(1, extra);
    p.t.set_children(p.root, &kids);
    p.layout();
    assert_eq!((p.t.rect(extra).y, p.t.rect(p.footer).y), (80.0, 112.0));
    p.t.set_style(extra, Style { display: Display::None, ..Style::default() });
    p.layout();
    assert_eq!(p.t.rect(p.footer).y, 96.0);
    p.t.set_style(extra, Style::default());
    p.layout();
    assert_eq!(p.t.rect(p.footer).y, 112.0);
    p.t.remove(extra);
    p.text.chars.remove(&extra);
    p.layout();
    assert_eq!((p.t.rect(p.footer).y, p.t.len()), (96.0, 6));
}

#[test]
fn a_resize_asks_only_the_nodes_whose_request_changed() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let rows: Vec<_> = (0..50).map(|_| text.leaf(&mut t, 6)).collect();
    let side = node(
        &mut t,
        Style { direction: Direction::Column, width: Length::Px(200.0), ..Style::default() },
        &rows,
    );
    let body = t.insert(Style { grow: 1.0, ..Style::default() });
    let root = node(&mut t, Style::default(), &[side, body]);
    t.layout(root, fill(800.0, 600.0), &mut text);
    t.take_stats();
    t.layout(root, fill(900.0, 600.0), &mut text); // wider: the sidebar is asked what it was
    let stats = t.take_stats();
    assert_eq!(t.rect(body).width, 700.0);
    assert!(stats.asked <= 3 && stats.measured == 0, "{stats:?}");
    assert_eq!(t.verify(&mut text), None);
}

#[test]
fn many_changes_at_once_fall_back_to_one_layout() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let rows: Vec<_> = (0..60).map(|_| text.leaf(&mut t, 10)).collect();
    let wrapped: Vec<_> = rows.iter().map(|&r| node(&mut t, column(), &[r])).collect();
    let root = node(&mut t, Style { direction: Direction::Column, ..px(100.0, 600.0) }, &wrapped);
    t.layout(root, fill(100.0, 600.0), &mut text);
    for (i, &row) in rows.iter().enumerate() {
        text.chars.insert(row, 20 + i);
        t.mark_dirty(row);
    }
    t.layout(root, fill(100.0, 600.0), &mut text);
    assert_eq!(t.verify(&mut text), None);
    assert!(t.take_stats().climbed < 60 * 2);
}

#[test]
fn roots_can_fit_their_content_or_take_a_width() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let label = text.leaf(&mut t, 10);
    let menu = node(&mut t, Style { padding: Edges::all(4.0), ..column() }, &[label]);
    assert_eq!(t.layout(menu, RootSize::Fit(Size::new(800.0, 600.0)), &mut text), Size::new(88.0, 24.0));
    assert_eq!(t.layout(menu, RootSize::Width(48.0), &mut text), Size::new(48.0, 40.0)); // 5 a line
    assert_eq!(t.layout(menu, RootSize::Fixed(Size::new(30.0, 30.0)), &mut text), Size::new(30.0, 30.0));
    text.chars.insert(label, 3);
    t.mark_dirty(label);
    t.update(&mut text); // the root is laid out again as it last was
    assert_eq!(t.rect(label), rect(4.0, 4.0, 22.0, 22.0));
    assert_eq!(t.verify(&mut text), None);
}

/// A small deterministic generator: the sequences below are the same on every run.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

#[test]
fn random_changes_always_match_a_layout_from_scratch() {
    for seed in 1..=6 {
        let mut rng = Rng(seed);
        let (mut t, mut text) = (LayoutTree::new(), Text::default());
        let mut boxes = Vec::new();
        let mut leaves = Vec::new();
        for _ in 0..8 {
            let kids: Vec<_> = (0..3).map(|_| text.leaf(&mut t, 1 + rng.below(40))).collect();
            leaves.extend(&kids);
            boxes.push(node(&mut t, Style { direction: Direction::Column, ..Style::default() }, &kids));
        }
        let root = node(&mut t, Style { direction: Direction::Column, ..Style::default() }, &boxes);
        let mut size = (400.0, 600.0);
        for step in 0..250 {
            let op = rng.below(8);
            let b = boxes[rng.below(boxes.len())];
            match op {
                0 if !leaves.is_empty() => {
                    let leaf = leaves[rng.below(leaves.len())];
                    text.chars.insert(leaf, 1 + rng.below(80));
                    t.mark_dirty(leaf);
                }
                1 => {
                    let width = [Length::Auto, Length::Px(120.0), Length::Pct(50.0)][rng.below(3)];
                    t.set_style(b, Style { width, ..t.style(b).clone() });
                }
                2 => {
                    let padding = Edges::all(rng.below(3) as f32 * 4.0);
                    t.set_style(b, Style { padding, grow: rng.below(2) as f32, ..t.style(b).clone() });
                }
                3 => {
                    let direction = if rng.below(2) == 0 { Direction::Row } else { Direction::Column };
                    t.set_style(b, Style { direction, wrap: rng.below(2) == 0, ..t.style(b).clone() });
                }
                4 => {
                    let leaf = text.leaf(&mut t, 1 + rng.below(40));
                    let mut kids = t.children(b).to_vec();
                    kids.insert(rng.below(kids.len() + 1), leaf);
                    t.set_children(b, &kids);
                    leaves.push(leaf);
                }
                5 if leaves.len() > 4 => {
                    let leaf = leaves.swap_remove(rng.below(leaves.len()));
                    t.remove(leaf);
                    text.chars.remove(&leaf);
                }
                6 => {
                    let display = if rng.below(3) == 0 { Display::None } else { Display::Flex };
                    t.set_style(b, Style { display, ..t.style(b).clone() });
                }
                _ => size = (200.0 + rng.below(5) as f32 * 100.0, 600.0),
            }
            t.layout(root, fill(size.0, size.1), &mut text);
            assert_eq!(t.verify(&mut text), None, "seed {seed} step {step} op {op}");
        }
    }
}
