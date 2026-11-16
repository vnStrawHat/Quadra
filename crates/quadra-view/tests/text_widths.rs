//! A text's size answers the widths it holds for and no others (specs/06, "Measuring
//! leaves"). On the fake backend a character is 6 px wide at size 10.

use quadra_prim::Size;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_view::{row, text};

#[test]
fn a_text_that_joins_its_lines_after_the_window_widened_takes_the_width() {
    let words = signal("ab\ncd".to_string());
    let mut h = Harness::new();
    h.size = Size::new(20.0, 100.0);
    h.mount(move || row().text_size(10.0).child(text(move || words.get())));
    h.frame();
    // Two lines of 12 px: that size holds at any width, so the wider window asks nothing.
    h.size = Size::new(800.0, 100.0);
    h.frame();
    // As two lines at 20 px it is as large as before, but at 800 it is one line.
    words.set("ab cd".to_string());
    h.frame();
    let node = h.get_by_text("ab cd").node();
    assert_eq!(h.tree.window_rect(node).width, 30.0);
}
