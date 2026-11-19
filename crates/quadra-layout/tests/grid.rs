//! Grid conformance: the cases of the V suite, and grid under incremental layout.

mod common;

use common::{Text, fill, node, rect};
use quadra_layout::{Display, GridStyle, LayoutTree, Length, NoMeasure, Style, Track};

fn grid(g: GridStyle) -> Option<Box<GridStyle>> {
    Some(Box::new(g))
}

fn grid_root(columns: Vec<Track>, width: f32) -> Style {
    Style {
        display: Display::Grid,
        width: Length::Px(width),
        grid: grid(GridStyle { columns, ..GridStyle::default() }),
        ..Style::default()
    }
}

fn tall(height: f32) -> Style {
    Style { height: Length::Px(height), ..Style::default() }
}

#[test]
fn fixed_and_fr_columns() {
    let mut t = LayoutTree::new();
    let ids: Vec<_> = (0..6).map(|_| t.insert(tall(20.0))).collect();
    let style =
        Style { gap_row: 10.0, ..grid_root(vec![Track::px(100.0), Track::fr(1.0), Track::fr(2.0)], 400.0) };
    let root = node(&mut t, style, &ids);
    t.layout(root, fill(400.0, 1000.0), &mut NoMeasure);
    assert_eq!(t.rect(ids[0]), rect(0.0, 0.0, 100.0, 20.0));
    assert_eq!(t.rect(ids[1]), rect(100.0, 0.0, 100.0, 20.0));
    assert_eq!(t.rect(ids[2]), rect(200.0, 0.0, 200.0, 20.0));
    assert_eq!(t.rect(ids[3]), rect(0.0, 30.0, 100.0, 20.0)); // the second row, after the gap
    assert_eq!(t.content_size(root).height, 50.0);
}

#[test]
fn an_fr_column_held_at_its_least_leaves_the_rest_to_the_others() {
    // Shares 1 and 9 of 400 are 40 and 360; the first is at least 100, so the second has 300.
    let mut t = LayoutTree::new();
    let ids: Vec<_> = (0..2).map(|_| t.insert(tall(20.0))).collect();
    let least = quadra_layout::TrackSize::Px(100.0);
    let columns = vec![Track::minmax(least, quadra_layout::TrackSize::Fr(1.0)), Track::fr(9.0)];
    let root = node(&mut t, grid_root(columns, 400.0), &ids);
    t.layout(root, fill(400.0, 1000.0), &mut NoMeasure);
    assert_eq!(t.rect(ids[0]), rect(0.0, 0.0, 100.0, 20.0));
    assert_eq!(t.rect(ids[1]), rect(100.0, 0.0, 300.0, 20.0));
}

#[test]
fn an_automatic_row_is_as_tall_as_its_tallest_item() {
    let mut t = LayoutTree::new();
    let a = t.insert(Style::default()); // auto height: stretched to the row
    let b = t.insert(tall(40.0));
    let root = node(&mut t, grid_root(Track::repeat(2, Track::fr(1.0)), 200.0), &[a, b]);
    t.layout(root, fill(200.0, 500.0), &mut NoMeasure);
    assert_eq!((t.rect(a).height, t.rect(b).height), (40.0, 40.0));
}

#[test]
fn named_areas_and_spans() {
    let mut t = LayoutTree::new();
    let mut area = |name: &str| {
        t.insert(Style {
            grid: grid(GridStyle { area: name.to_string(), ..GridStyle::default() }),
            ..Style::default()
        })
    };
    let (header, nav, main) = (area("header"), area("nav"), area("main"));
    let style = Style {
        display: Display::Grid,
        width: Length::Px(300.0),
        height: Length::Px(200.0),
        grid: grid(GridStyle {
            columns: vec![Track::px(100.0), Track::fr(1.0)],
            rows: vec![Track::px(50.0), Track::fr(1.0)],
            areas: vec!["header header".to_string(), "nav main".to_string()],
            ..GridStyle::default()
        }),
        ..Style::default()
    };
    let root = node(&mut t, style, &[main, nav, header]);
    t.layout(root, fill(300.0, 200.0), &mut NoMeasure);
    assert_eq!(t.rect(header), rect(0.0, 0.0, 300.0, 50.0));
    assert_eq!(t.rect(nav), rect(0.0, 50.0, 100.0, 150.0));
    assert_eq!(t.rect(main), rect(100.0, 50.0, 200.0, 150.0));
}

#[test]
fn columns_that_fill_the_width_are_responsive() {
    let mut t = LayoutTree::new();
    let ids: Vec<_> = (0..4).map(|_| t.insert(tall(10.0))).collect();
    let style = Style {
        display: Display::Grid,
        width: Length::Px(500.0),
        gap_col: 10.0,
        grid: grid(GridStyle { fill_min: 150.0, ..GridStyle::default() }),
        ..Style::default()
    };
    let root = node(&mut t, style.clone(), &ids);
    t.layout(root, fill(500.0, 500.0), &mut NoMeasure);
    assert_eq!(t.rect(ids[0]).width, 160.0); // 3 columns of (500 - 20) / 3
    assert_eq!((t.rect(ids[2]).x, t.rect(ids[3]).y), (340.0, 10.0)); // the fourth wraps
    t.set_style(root, Style { width: Length::Px(330.0), ..style });
    t.layout(root, fill(500.0, 500.0), &mut NoMeasure);
    assert_eq!((t.rect(ids[0]).width, t.rect(ids[2]).y), (160.0, 10.0)); // 2 columns now
    assert_eq!(t.verify(&mut NoMeasure), None);
}

#[test]
fn explicit_placement_and_automatic_flow() {
    let mut t = LayoutTree::new();
    let pinned = t.insert(Style {
        grid: grid(GridStyle { col_start: 2, row_start: 1, ..GridStyle::default() }),
        ..Style::default()
    });
    let a = t.insert(tall(10.0));
    let b = t.insert(tall(10.0));
    let root = node(&mut t, grid_root(Track::repeat(2, Track::px(10.0)), 20.0), &[pinned, a, b]);
    t.layout(root, fill(20.0, 100.0), &mut NoMeasure);
    assert_eq!(t.rect(pinned).origin(), quadra_prim::Point::new(10.0, 0.0));
    assert_eq!(t.rect(a).origin(), quadra_prim::Point::ZERO); // the free cell before it
    assert_eq!(t.rect(b), rect(0.0, 10.0, 10.0, 10.0)); // the next row: row one is full
}

#[test]
fn placement_always_ends() {
    // An item wider than the grid, and items pinned to a full row or column: each takes
    // tracks beyond the defined ones.
    let mut t = LayoutTree::new();
    let item = |t: &mut LayoutTree, g: GridStyle| t.insert(Style { grid: grid(g), ..tall(10.0) });
    let wide = item(&mut t, GridStyle { col_span: 5, ..GridStyle::default() });
    let pinned: Vec<_> =
        (0..3).map(|_| item(&mut t, GridStyle { row_start: 2, ..GridStyle::default() })).collect();
    let stacked: Vec<_> =
        (0..3).map(|_| item(&mut t, GridStyle { col_start: 1, ..GridStyle::default() })).collect();
    let mut kids = vec![wide];
    kids.extend(pinned.iter().chain(&stacked));
    let root = node(&mut t, grid_root(Track::repeat(2, Track::px(10.0)), 20.0), &kids);
    t.layout(root, fill(20.0, 100.0), &mut NoMeasure);
    assert_eq!(t.rect(pinned[2]).y, t.rect(pinned[0]).y);
    assert!(t.rect(stacked[2]).y > t.rect(stacked[1]).y);
}

#[test]
fn a_text_change_resizes_its_row_and_stops_at_a_fixed_grid() {
    let (mut t, mut text) = (LayoutTree::new(), Text::default());
    let cells: Vec<_> = (0..4).map(|_| text.leaf(&mut t, 5)).collect();
    let style = Style { height: Length::Px(300.0), ..grid_root(Track::repeat(2, Track::fr(1.0)), 200.0) };
    let table = node(&mut t, style, &cells);
    let footer = t.insert(tall(20.0));
    let root = node(
        &mut t,
        Style { direction: quadra_layout::Direction::Column, ..Style::default() },
        &[table, footer],
    );
    t.layout(root, fill(200.0, 400.0), &mut text);
    assert_eq!(t.rect(cells[2]).y, 16.0);
    t.take_stats();
    text.chars.insert(cells[0], 30); // 240 px in a 100 px column: three lines
    t.mark_dirty(cells[0]);
    t.layout(root, fill(200.0, 400.0), &mut text);
    assert_eq!((t.rect(cells[0]).height, t.rect(cells[1]).height, t.rect(cells[2]).y), (48.0, 48.0, 48.0));
    assert_eq!(t.take_stats().climbed, 1); // the grid has a fixed size: the page above never runs
    assert_eq!(t.verify(&mut text), None);
}
