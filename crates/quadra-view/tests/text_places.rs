//! Where the characters of an input are, for a reader (specs/12): a line each, every
//! character of the value in one of them, from the input's corner. On the fake backend a
//! character is 6 px wide at size 10, on lines of 14.

use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{column, text_area};

#[test]
fn an_area_tells_its_lines_and_where_their_characters_are() {
    let value = signal("ab cd\nef".to_string());
    let mut h = Harness::new();
    h.mount(move || column().width(200.0).text_size(10.0).line_height(1.4).child(text_area(value)));
    h.frame();
    let says = h.tree.semantic_tree();
    let area = says.nodes.iter().find(|n| n.semantics.role == Role::TextInput).unwrap();
    let lines = &area.semantics.lines;
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].start, lines[1].start), (0, 6));
    // "ab cd" and its line break, then "ef", one line under the other.
    assert_eq!(lines[0].positions, [0.0, 6.0, 12.0, 18.0, 24.0, 30.0]);
    assert_eq!(lines[1].positions, [0.0, 6.0]);
    assert_eq!(lines[1].widths, [6.0, 6.0]);
    assert_eq!((lines[0].bounds.y, lines[1].bounds.y), (0.0, 14.0));
}
