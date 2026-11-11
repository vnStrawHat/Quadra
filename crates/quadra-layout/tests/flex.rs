//! Flex conformance: the cases of the V suite (specs/06, Tests), as style trees in and rects
//! out.

mod common;

use common::{Text, fill, node, px, rect};
use quadra_layout::{
    Align, AutoMargins, Direction, Insets, Justify, LayoutTree, Length, NoMeasure, Overflow, Position, Style,
};
use quadra_prim::Edges;

fn column() -> Style {
    Style { direction: Direction::Column, ..Style::default() }
}

#[test]
fn fixed_children_of_a_row_stretch_across() {
    let mut t = LayoutTree::new();
    let a = t.insert(Style { width: Length::Px(50.0), ..Style::default() });
    let b = t.insert(Style { width: Length::Px(70.0), ..Style::default() });
    let root = node(&mut t, px(300.0, 100.0), &[a, b]);
    t.layout(root, fill(300.0, 100.0), &mut NoMeasure);
    assert_eq!(t.rect(a), rect(0.0, 0.0, 50.0, 100.0));
    assert_eq!(t.rect(b), rect(50.0, 0.0, 70.0, 100.0));
}

#[test]
fn grow_shares_free_space_and_shrink_shares_overflow() {
    let mut t = LayoutTree::new();
    let a = t.insert(Style { grow: 1.0, ..Style::default() });
    let b = t.insert(Style { grow: 2.0, ..Style::default() });
    let root = node(&mut t, px(300.0, 50.0), &[a, b]);
    t.layout(root, fill(300.0, 50.0), &mut NoMeasure);
    assert_eq!((t.rect(a).width, t.rect(b).width, t.rect(b).x), (100.0, 200.0, 100.0));

    let mut t = LayoutTree::new();
    let ids: Vec<_> =
        (0..3).map(|_| t.insert(Style { basis: Length::Px(200.0), ..Style::default() })).collect();
    let root = node(&mut t, px(300.0, 50.0), &ids);
    t.layout(root, fill(300.0, 50.0), &mut NoMeasure);
    assert!(ids.iter().all(|&id| t.rect(id).width == 100.0));
}

#[test]
fn a_maximum_freezes_an_item_and_the_rest_is_shared_again() {
    let mut t = LayoutTree::new();
    let a = t.insert(Style { grow: 1.0, max_width: Length::Px(50.0), ..Style::default() });
    let b = t.insert(Style { grow: 1.0, ..Style::default() });
    let root = node(&mut t, px(300.0, 10.0), &[a, b]);
    t.layout(root, fill(300.0, 10.0), &mut NoMeasure);
    assert_eq!((t.rect(a).width, t.rect(b).width), (50.0, 250.0));
}

#[test]
fn justify_places_items_inside_the_padding() {
    let cases = [
        (Justify::Start, 10.0, 50.0),
        (Justify::End, 150.0, 190.0),
        (Justify::Center, 80.0, 120.0),
        (Justify::Between, 10.0, 190.0),
        (Justify::Around, 45.0, 155.0),
        (Justify::Evenly, 56.666668, 143.33334),
    ];
    for (justify, ax, bx) in cases {
        let mut t = LayoutTree::new();
        let a = t.insert(px(40.0, 10.0));
        let b = t.insert(px(40.0, 10.0));
        let style = Style { padding: Edges::all(10.0), justify, ..px(240.0, 30.0) };
        let root = node(&mut t, style, &[a, b]);
        t.layout(root, fill(240.0, 30.0), &mut NoMeasure);
        assert!((t.rect(a).x - ax).abs() < 1e-3, "{justify:?}: a.x {}", t.rect(a).x);
        assert!((t.rect(b).x - bx).abs() < 1e-3, "{justify:?}: b.x {}", t.rect(b).x);
        assert_eq!(t.rect(a).y, 10.0);
    }
}

#[test]
fn text_in_a_column_wraps_at_the_stretched_width() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let leaf = text.leaf(&mut t, 25); // 200 px unwrapped
    let other = t.insert(Style { height: Length::Px(10.0), ..Style::default() });
    let root = node(&mut t, Style { width: Length::Px(100.0), gap_row: 4.0, ..column() }, &[leaf, other]);
    t.layout(root, fill(100.0, 400.0), &mut text);
    assert_eq!(t.rect(leaf), rect(0.0, 0.0, 100.0, 48.0)); // 12 characters a line, 3 lines
    assert_eq!(t.rect(other).y, 52.0);
}

#[test]
fn wrapping_makes_lines() {
    let mut t = LayoutTree::new();
    let ids: Vec<_> = (0..4).map(|_| t.insert(px(100.0, 20.0))).collect();
    let style = Style {
        width: Length::Px(250.0),
        wrap: true,
        gap_row: 5.0,
        gap_col: 10.0,
        align_items: Align::Start,
        ..Style::default()
    };
    let wrapper = node(&mut t, style, &ids);
    let root = node(&mut t, Style { align_items: Align::Start, ..column() }, &[wrapper]);
    t.layout(root, fill(250.0, 500.0), &mut NoMeasure);
    assert_eq!(t.rect(ids[1]), rect(110.0, 0.0, 100.0, 20.0));
    assert_eq!(t.rect(ids[2]), rect(0.0, 25.0, 100.0, 20.0));
    assert_eq!(t.rect(wrapper).height, 45.0);
}

#[test]
fn absolute_children_and_auto_margins() {
    let mut t = LayoutTree::new();
    let inset = Insets { right: Length::Px(10.0), top: Length::Px(10.0), ..Insets::default() };
    let abs = t.insert(Style { position: Position::Absolute, inset, ..px(20.0, 20.0) });
    let margin_auto = AutoMargins { left: true, ..AutoMargins::default() };
    let pushed = t.insert(Style { width: Length::Px(30.0), margin_auto, ..Style::default() });
    let root = node(&mut t, px(200.0, 100.0), &[abs, pushed]);
    t.layout(root, fill(200.0, 100.0), &mut NoMeasure);
    assert_eq!(t.rect(abs), rect(170.0, 10.0, 20.0, 20.0));
    assert_eq!(t.rect(pushed).x, 170.0);
}

#[test]
fn negative_insets_are_still_insets() {
    let mut t = LayoutTree::new();
    let top_right = Insets { right: Length::Px(-1.0), top: Length::Px(-1.0), ..Insets::default() };
    let a = t.insert(Style { position: Position::Absolute, inset: top_right, ..px(20.0, 20.0) });
    let bottom_left = Insets { left: Length::Px(-1.0), bottom: Length::Px(-4.0), ..Insets::default() };
    let b = t.insert(Style { position: Position::Absolute, inset: bottom_left, ..px(10.0, 10.0) });
    let root = node(&mut t, px(200.0, 100.0), &[a, b]);
    t.layout(root, fill(200.0, 100.0), &mut NoMeasure);
    assert_eq!(t.rect(a), rect(181.0, -1.0, 20.0, 20.0));
    assert_eq!(t.rect(b), rect(-1.0, 94.0, 10.0, 10.0));
}

#[test]
fn an_auto_container_fits_its_content() {
    let mut t = LayoutTree::new();
    let a = t.insert(px(40.0, 10.0));
    let b = t.insert(px(60.0, 30.0));
    let inner = node(&mut t, Style { gap_col: 5.0, ..Style::default() }, &[a, b]);
    let root = node(&mut t, Style { align_items: Align::Start, ..column() }, &[inner]);
    t.set_style(root, Style { width: Length::Px(500.0), height: Length::Px(500.0), ..t.style(root).clone() });
    t.layout(root, fill(500.0, 500.0), &mut NoMeasure);
    assert_eq!(t.rect(inner), rect(0.0, 0.0, 105.0, 30.0));
    assert_eq!(t.rect(b).x, 45.0);
}

#[test]
fn the_cache_keeps_nested_auto_columns_from_measuring_exponentially() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let mut child = text.leaf(&mut t, 10);
    for _ in 0..31 {
        child = node(&mut t, column(), &[child]);
    }
    t.layout(child, fill(300.0, 300.0), &mut text);
    let asked = t.take_stats().asked;
    assert!(asked < 200, "{asked} evaluations");
}

#[test]
fn an_auto_column_in_a_scroll_container_keeps_its_content_height() {
    let mut t = LayoutTree::new();
    let a = t.insert(px(100.0, 300.0));
    let b = t.insert(px(100.0, 300.0));
    let page = node(&mut t, Style { gap_row: 10.0, ..column() }, &[a, b]);
    let root = node(&mut t, Style { overflow_y: Overflow::Scroll, ..column() }, &[page]);
    t.set_style(root, Style { width: Length::Px(200.0), height: Length::Px(400.0), ..t.style(root).clone() });
    t.layout(root, fill(200.0, 400.0), &mut NoMeasure);
    assert_eq!(t.rect(page).height, 610.0);
    assert_eq!(t.rect(b), rect(0.0, 310.0, 100.0, 300.0));
}

#[test]
fn a_growing_body_that_scrolls_takes_the_viewport_and_its_content_overflows() {
    let mut t = LayoutTree::new();
    let tall = t.insert(px(100.0, 2000.0));
    let body = node(&mut t, Style { grow: 1.0, overflow_y: Overflow::Scroll, ..Style::default() }, &[tall]);
    let header = t.insert(Style { height: Length::Px(52.0), ..Style::default() });
    let root = node(&mut t, Style { direction: Direction::Column, ..px(300.0, 800.0) }, &[header, body]);
    t.layout(root, fill(300.0, 800.0), &mut NoMeasure);
    assert_eq!(t.rect(header).height, 52.0);
    assert_eq!(t.rect(body), rect(0.0, 52.0, 300.0, 748.0));
    assert_eq!(t.rect(tall).height, 2000.0);
}

#[test]
fn a_row_that_scrolls_down_stretches_its_child_to_the_content() {
    let mut t = LayoutTree::new();
    let a = t.insert(px(100.0, 500.0));
    let b = t.insert(px(100.0, 500.0));
    let page = node(&mut t, column(), &[a, b]);
    let body = node(&mut t, Style { overflow_y: Overflow::Scroll, ..px(300.0, 400.0) }, &[page]);
    t.layout(body, fill(300.0, 400.0), &mut NoMeasure);
    assert_eq!((t.rect(page).height, t.rect(b).height), (1000.0, 500.0));
    assert_eq!(t.content_size(body).height, 1000.0);
}

#[test]
fn text_is_measured_at_the_width_a_maximum_leaves_it() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let leaf = text.leaf(&mut t, 100);
    let capped = Style { width: Length::Pct(100.0), max_width: Length::Px(200.0), ..column() };
    let col = node(&mut t, capped, &[leaf]);
    let style = Style {
        direction: Direction::Column,
        align_items: Align::Center,
        overflow_y: Overflow::Scroll,
        ..px(1000.0, 400.0)
    };
    let root = node(&mut t, style, &[col]);
    t.layout(root, fill(1000.0, 400.0), &mut text);
    assert_eq!((t.rect(col).width, t.rect(col).height), (200.0, 64.0)); // 25 characters a line
}

#[test]
fn the_only_growing_item_is_not_measured_for_its_base_size() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let leaf = text.leaf(&mut t, 2000);
    let body = node(&mut t, Style { grow: 1.0, ..column() }, &[leaf]);
    let header = t.insert(Style { height: Length::Px(52.0), ..Style::default() });
    let root = node(&mut t, Style { direction: Direction::Column, ..px(300.0, 800.0) }, &[header, body]);
    t.layout(root, fill(300.0, 800.0), &mut text);
    assert_eq!(t.rect(body), rect(0.0, 52.0, 300.0, 748.0));
    let asked = t.take_stats().asked;
    assert!(asked <= 6, "{asked} evaluations");
}

#[test]
fn hidden_overflow_clips_but_keeps_the_content_height() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let rows: Vec<_> = (0..10).map(|_| t.insert(px(100.0, 50.0))).collect();
    let clipped = Style { overflow_x: Overflow::Hidden, overflow_y: Overflow::Hidden, ..column() };
    let table = node(&mut t, clipped, &rows);
    let para = text.leaf(&mut t, 40); // 320 px
    let page = node(&mut t, column(), &[para, table]);
    let style = Style { direction: Direction::Column, overflow_y: Overflow::Scroll, ..px(200.0, 300.0) };
    let root = node(&mut t, style, &[page]);
    t.layout(root, fill(200.0, 300.0), &mut text);
    assert_eq!(t.rect(table).height, 500.0);
    assert_eq!(t.rect(para).height, 32.0); // two lines at 200
    assert_eq!(t.rect(rows[9]), rect(0.0, 450.0, 100.0, 50.0));
}
