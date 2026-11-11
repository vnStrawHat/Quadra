//! The edit buffer (behaviour/09, "Text input internals"): movement by graphemes and words,
//! selection by an anchor and a head, deletion, and an undo history where typing is one step
//! per word.

use quadra_text::{EditBuffer, Motion};

fn state(b: &EditBuffer) -> (String, usize, std::ops::Range<usize>) {
    (b.text().to_string(), b.head(), b.selection())
}

#[test]
fn the_caret_moves_by_graphemes_and_words() {
    let mut b = EditBuffer::new("héllo wörld_1, ok");
    assert_eq!(b.head(), 19); // at the end, in bytes
    b.move_to(Motion::Start, false);
    b.move_to(Motion::Right, false);
    b.move_to(Motion::Right, false);
    assert_eq!(b.head(), 3); // "h" and "é", which is two bytes
    b.move_to(Motion::WordRight, false);
    assert_eq!(b.head(), 6); // the end of "héllo"
    b.move_to(Motion::WordRight, false);
    assert_eq!(b.head(), 15); // over the space, to the end of "wörld_1"
    b.move_to(Motion::WordRight, false);
    assert_eq!(b.head(), 16); // the comma is a word of its own
    b.move_to(Motion::WordLeft, false);
    b.move_to(Motion::WordLeft, false);
    assert_eq!(b.head(), 7);
    b.move_to(Motion::Left, false);
    b.move_to(Motion::WordLeft, false);
    assert_eq!(b.head(), 0);
    b.move_to(Motion::Left, false); // nowhere further
    assert_eq!(b.head(), 0);

    // A base letter and its marks are one step, like an emoji of several code points.
    let mut b = EditBuffer::new("e\u{0323}\u{0302}x");
    b.move_to(Motion::Left, false);
    b.move_to(Motion::Left, false);
    assert_eq!(b.head(), 0);
}

#[test]
fn a_selection_has_an_anchor_that_stays_and_a_head_that_moves() {
    let mut b = EditBuffer::new("one two three");
    b.set_caret(4, false);
    b.move_to(Motion::WordRight, true);
    assert_eq!((b.selection(), b.selected_text()), (4..7, "two"));
    b.move_to(Motion::Left, true); // shrinks from the head
    assert_eq!(b.selection(), 4..6);
    b.move_to(Motion::Start, true); // the head crosses the anchor
    assert_eq!((b.selection(), b.head()), (0..4, 0));
    b.move_to(Motion::Right, false); // a step out of a selection lands on that end of it
    assert_eq!((b.selection(), b.head()), (4..4, 4));

    b.select_word(9);
    assert_eq!(b.selected_text(), "three");
    b.select_word(7); // on the space between two words: the word after it
    assert_eq!(b.selected_text(), "three");
    b.set_caret(2, false);
    b.set_caret(10, true); // a drag
    assert_eq!(b.selected_text(), "e two th");
    b.select_all();
    assert_eq!(b.selection(), 0..13);
    b.set_caret(99, false); // past the end: the end
    assert_eq!(b.head(), 13);
}

#[test]
fn typing_replaces_the_selection_and_deleting_removes_it_or_a_step() {
    let mut b = EditBuffer::new("one two");
    b.select_word(5);
    assert!(b.insert("2"));
    assert_eq!(state(&b), ("one 2".to_string(), 5, 5..5));
    assert!(b.delete(Motion::Left)); // Backspace
    assert!(b.delete(Motion::WordLeft)); // Ctrl+Backspace: the space, then the word
    assert_eq!(state(&b), ("".to_string(), 0, 0..0));
    assert!(!b.delete(Motion::Left) && !b.delete(Motion::Right) && !b.insert("")); // nothing to do

    let mut b = EditBuffer::new("héllo");
    b.set_caret(1, false);
    assert!(b.delete(Motion::Right)); // Delete takes the whole "é"
    assert_eq!(b.text(), "hllo");
    b.select_all();
    assert!(b.delete(Motion::Right));
    assert_eq!(b.text(), "");
}

#[test]
fn undo_takes_back_a_word_of_typing_and_restores_the_selection() {
    let mut b = EditBuffer::new("");
    for c in "hello world".chars() {
        b.insert(&c.to_string());
    }
    assert!(b.undo());
    assert_eq!(b.text(), "hello "); // "world" was one step
    assert!(b.undo());
    assert_eq!(b.text(), "hello"); // the space was its own
    assert!(b.redo() && b.redo() && !b.redo());
    assert_eq!(state(&b), ("hello world".to_string(), 11, 11..11));

    b.set_caret(0, false);
    b.move_to(Motion::WordRight, true);
    b.insert("bye"); // over a selection: one step, and undo selects again what was there
    assert_eq!(b.text(), "bye world");
    assert!(b.undo());
    assert_eq!(state(&b), ("hello world".to_string(), 5, 0..5));

    // Moving the caret ends a run of typing; a new edit forgets what was undone.
    let mut b = EditBuffer::new("");
    b.insert("a");
    b.insert("b");
    b.move_to(Motion::Left, false);
    b.insert("c");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    b.insert("x");
    assert!(!b.redo());
    assert!(b.undo() && b.undo() && !b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn a_text_set_from_outside_keeps_the_caret_where_it_fits() {
    let mut b = EditBuffer::new("hello");
    b.insert("!");
    b.set_text("hi");
    assert_eq!(state(&b), ("hi".to_string(), 2, 2..2));
    assert!(!b.undo()); // the history was of another text
    b.set_caret(1, false);
    b.set_text("hi"); // the same text: nothing changes
    assert_eq!(b.head(), 1);
}
