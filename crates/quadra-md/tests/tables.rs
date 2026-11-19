//! Tables with long text (specs/13, "Markdown"): prose in a cell wraps inside it and the table
//! stays as wide as the page; a word longer than a column makes the table scroll sideways; a
//! table of one row or one column is laid out like any other.

use std::rc::Rc;

use quadra_design::dark;
use quadra_md::markdown;
use quadra_prim::Rect;
use quadra_testing::Harness;
use quadra_tree::Role;

const PROSE: &str = "A cell of prose long enough to wrap on several lines in its column, as notes in a \
                     table of settings often are, with words of every length";

fn page(source: &str, width: f32) -> Harness {
    let mut h = Harness::new();
    h.size = quadra_prim::Size::new(width, 800.0);
    h.tree.set_theme(Rc::new(dark().theme()));
    let source = source.to_string();
    h.mount(move || quadra_design::root().child(markdown(source.clone())));
    h.frame();
    h
}

fn rects(h: &Harness, role: Role) -> Vec<Rect> {
    h.query(|n| n.semantics.role == role).into_iter().map(|id| h.tree.window_rect(id)).collect()
}

fn inside(inner: Rect, outer: Rect) -> bool {
    let near = 0.5;
    inner.x >= outer.x - near
        && inner.y >= outer.y - near
        && inner.x + inner.width <= outer.x + outer.width + near
        && inner.y + inner.height <= outer.y + outer.height + near
}

#[test]
fn prose_in_a_cell_wraps_inside_it_and_the_table_is_as_wide_as_the_page() {
    for width in [800.0, 360.0] {
        let source = format!("| Name | Notes |\n|---|---|\n| retries | {PROSE} |\n| timeout | 30 s |\n");
        let mut h = page(&source, width);
        let table = rects(&h, Role::Table)[0];
        assert!(table.x >= 0.0 && table.x + table.width <= width + 0.5, "{width}: {table:?}");
        let cells = rects(&h, Role::Cell);
        assert_eq!(cells.len(), 6);
        // The cell of prose is several lines high, its row's other cell as high.
        assert!(cells[3].height > 1.5 * cells[5].height, "{width}: {cells:?}");
        assert_eq!(cells[2].height, cells[3].height);
        let node = h.get_by_text(PROSE).node();
        let words = h.tree.window_rect(node);
        assert!(inside(words, cells[3]), "{width}: {words:?} in {:?}", cells[3]);
        assert!(cells.iter().all(|cell| inside(*cell, table)));
    }
}

#[test]
fn a_word_longer_than_its_column_scrolls_the_table_and_stays_in_its_cell() {
    let url = "https://example.com/a/path/that/is/one/word/far/too/long/for/the/column/it/is/in/index.html";
    let source = format!("| Link | What |\n|---|---|\n| {url} | {PROSE} |\n");
    let mut h = page(&source, 360.0);
    let cells = rects(&h, Role::Cell);
    let node = h.get_by_role(Role::Link, url).node();
    let words = h.tree.window_rect(node);
    assert!(inside(words, cells[2]), "{words:?} in {:?}", cells[2]);
    // The prose beside it still wraps: the table is no wider than the word asks.
    assert!(cells[3].height > cells[0].height * 2.0, "{cells:?}");
}

#[test]
fn a_table_of_one_row_of_one_column_and_of_too_many_columns() {
    let h = page("| Only a head | And another |\n|---|---|\n", 800.0);
    assert_eq!(rects(&h, Role::Cell).len(), 2);
    let h = page("| One |\n|---|\n| a |\n| b |\n", 800.0);
    let cells = rects(&h, Role::Cell);
    assert_eq!(cells.len(), 3);
    assert!(cells.iter().all(|cell| cell.width == cells[0].width && cell.x == cells[0].x));
    // Past 64 columns the cells are dropped: a document that is not ours does not say how
    // wide the page is.
    let head: Vec<String> = (0..70).map(|c| format!("c{c}")).collect();
    let source = format!("| {} |\n|{}\n", head.join(" | "), "---|".repeat(70));
    let h = page(&source, 800.0);
    assert_eq!(rects(&h, Role::Cell).len(), 64);
}
