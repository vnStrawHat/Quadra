//! The document (specs/13, "Code"): where cursors go, what the commands do to the text and
//! to every cursor, and what undo takes back at once. A text is written with `|` where a
//! cursor is and `[..]` about what one selects.

use quadra_code::{Cursor, Doc, Move};

fn doc(marked: &str) -> Doc {
    let (mut text, mut cursors, mut anchor) = (String::new(), Vec::new(), 0);
    for c in marked.chars() {
        match c {
            '|' => cursors.push(Cursor::at(text.len())),
            '[' => anchor = text.len(),
            ']' => cursors.push(Cursor { anchor, head: text.len(), goal: None }),
            _ => text.push(c),
        }
    }
    let mut doc = Doc::new(&text);
    if !cursors.is_empty() {
        doc.cursors = cursors;
    }
    doc.normalize();
    doc
}

fn show(doc: &Doc) -> String {
    let mut text = doc.text();
    // From the end, so that what is put in moves nothing still to be marked.
    for c in doc.cursors.iter().rev() {
        if c.is_empty() {
            text.insert(c.head, '|');
        } else {
            text.insert(c.end(), ']');
            text.insert(c.start(), '[');
        }
    }
    text
}

/// Types a text a letter at a time, ten milliseconds a letter.
fn type_text(doc: &mut Doc, text: &str) {
    for letter in text.chars() {
        doc.now += 10;
        doc.insert_text(letter.encode_utf8(&mut [0; 4]), true);
    }
}

#[test]
fn cursors_move_by_grapheme_word_line_and_column() {
    let mut d = doc("ab|c\nxy");
    d.move_cursors(Move::Right, false, 0);
    assert_eq!(show(&d), "abc|\nxy");
    d.move_cursors(Move::Right, false, 0);
    assert_eq!(show(&d), "abc\n|xy");
    d.move_cursors(Move::Up, false, 0);
    assert_eq!(show(&d), "|abc\nxy");
    let mut d = doc("foo bar|.baz");
    d.move_cursors(Move::WordLeft, false, 0);
    assert_eq!(show(&d), "foo |bar.baz");
    d.move_cursors(Move::WordRight, true, 0);
    assert_eq!(show(&d), "foo [bar].baz");
    // Left leaves a selection at its start.
    d.move_cursors(Move::Left, false, 0);
    assert_eq!(show(&d), "foo |bar.baz");
    let mut d = doc("    if x|");
    d.move_cursors(Move::Home, false, 0);
    assert_eq!(show(&d), "    |if x");
    d.move_cursors(Move::Home, false, 0);
    assert_eq!(show(&d), "|    if x");
    // The column a cursor came from is where it goes back to after a short line.
    let mut d = doc("abcdef|\nx\nabcdefgh");
    d.move_cursors(Move::Down, false, 0);
    assert_eq!(show(&d), "abcdef\nx|\nabcdefgh");
    d.move_cursors(Move::Down, false, 0);
    assert_eq!(show(&d), "abcdef\nx\nabcdef|gh");
    d.move_cursors(Move::PageUp, true, 10);
    assert_eq!(show(&d), "abcdef[\nx\nabcdef]gh");
    // A letter with a mark after it is one step, a wide one two columns, a tab to its stop.
    let mut d = doc("|e\u{301}x\n漢字\n\tab");
    d.move_cursors(Move::Right, false, 0);
    assert_eq!(show(&d), "e\u{301}|x\n漢字\n\tab");
    assert_eq!((d.col_of(d.buffer.line_end(1)), d.col_of(d.buffer.line_end(2))), (4, 6));
    assert_eq!((d.offset_at_col(1, 2), d.offset_at_col(2, 1), d.offset_at_col(2, 3)), (8, 12, 13));
    // Words: across a line break a step, and letters that are not ASCII are of words.
    let mut d = doc("tiếng| việt\nx");
    d.move_cursors(Move::WordRight, false, 0);
    d.move_cursors(Move::WordRight, false, 0);
    assert_eq!(show(&d), "tiếng việt\n|x");
    d.move_cursors(Move::WordLeft, false, 0);
    d.move_cursors(Move::WordLeft, false, 0);
    assert_eq!((show(&d), d.word_at(2)), ("tiếng |việt\nx".to_string(), 0..7));
}

#[test]
fn typing_is_undone_a_run_at_a_time() {
    let mut d = doc("|");
    type_text(&mut d, "hello");
    d.newline();
    type_text(&mut d, "x");
    assert_eq!(show(&d), "hello\nx|");
    assert!(d.undo() && show(&d) == "hello\n|");
    assert!(d.undo() && show(&d) == "hello|");
    // The five letters are one group.
    assert!(d.undo() && show(&d) == "|");
    assert!(!d.undo());
    assert!(d.redo() && show(&d) == "hello|");
    assert!(d.redo() && d.redo() && !d.redo() && show(&d) == "hello\nx|");
    // A cursor that moves ends the group, and so does a pause.
    let mut d = doc("|");
    type_text(&mut d, "a");
    d.move_cursors(Move::Left, false, 0);
    d.move_cursors(Move::Right, false, 0);
    type_text(&mut d, "b");
    d.now += 2000;
    type_text(&mut d, "c");
    assert!(d.undo() && show(&d) == "ab|");
    assert!(d.undo() && show(&d) == "a|");
    // Something new after an undo leaves nothing to do again; every change is counted, and
    // the first line changed is known to who colours the text.
    type_text(&mut d, "z");
    assert!(!d.redo() && d.text() == "az");
    let mut d = doc("one\ntwo\nthree|");
    assert_eq!((d.version, d.dirty), (0, usize::MAX));
    type_text(&mut d, "!");
    d.set_cursor(5, 5, false);
    type_text(&mut d, "?");
    assert_eq!((d.version, d.dirty), (2, 1));
}

#[test]
fn a_new_line_keeps_its_indent_and_deleting_goes_by_grapheme_or_word() {
    let mut d = doc("fn main() {|}");
    d.newline();
    assert_eq!(show(&d), "fn main() {\n    |}");
    let mut d = doc("\tx := 1|");
    d.newline();
    assert_eq!(show(&d), "\tx := 1\n\t|");
    // A cursor in the indent takes the part of it that is before it.
    let mut d = doc("  |  x");
    d.newline();
    assert_eq!(show(&d), "  \n  |  x");
    let mut d = doc("ab|c");
    d.delete_back(false);
    assert_eq!(show(&d), "a|c");
    let mut d = doc("foo bar|");
    d.delete_back(true);
    assert_eq!(show(&d), "foo |");
    let mut d = doc("a|b\nc");
    d.delete_forward(false);
    d.delete_forward(false);
    assert_eq!(show(&d), "a|c");
    let mut d = doc("a[bc]d|e\u{301}");
    d.delete_forward(false);
    assert_eq!(show(&d), "a|d|");
}

#[test]
fn every_cursor_does_what_one_does() {
    let mut d = doc("a|\nb|\nc|");
    type_text(&mut d, "!");
    assert_eq!(show(&d), "a!|\nb!|\nc!|");
    d.delete_back(false);
    d.delete_back(false);
    assert_eq!(show(&d), "|\n|\n|");
    // One of them has nothing before it: the others delete, and it stays.
    d.delete_back(false);
    assert_eq!(show(&d), "|");
    let mut d = doc("x = foo|; y = foo; foo");
    d.select_next();
    assert_eq!(show(&d), "x = [foo]; y = foo; foo");
    d.select_next();
    d.select_next();
    assert_eq!(show(&d), "x = [foo]; y = [foo]; [foo]");
    // A cursor in the word that is selected is that selection.
    let mut e = doc("f|o|o");
    e.select_next();
    assert_eq!(show(&e), "[foo]");
    // Nothing more to find: nothing more is selected.
    d.select_next();
    assert_eq!(d.cursors.len(), 3);
    type_text(&mut d, "bar");
    assert_eq!(d.text(), "x = bar; y = bar; bar");
    // From the last one the search goes on from the start.
    let mut d = doc("foo foo [foo]");
    d.select_next();
    assert_eq!(show(&d), "[foo] foo [foo]");
    let mut d = doc("ab|c\nabc\nabc");
    d.add_cursor_vertical(true);
    d.add_cursor_vertical(true);
    type_text(&mut d, "-");
    assert_eq!(show(&d), "ab-|c\nab-|c\nab-|c");
    // A selection in a line is one in the line above too: a column.
    let mut d = doc("abcd\na[bc]d");
    d.add_cursor_vertical(false);
    assert_eq!(show(&d), "a[bc]d\na[bc]d");
    // Two cursors in a word delete it once.
    let mut d = doc("one tw|o th|ree four|");
    d.delete_back(true);
    assert_eq!(show(&d), "one |o |ree |");
    let mut d = doc("|on|e two");
    d.delete_forward(true);
    assert_eq!(show(&d), "| two");
    // Cursors that meet are one.
    let mut d = doc("a|b|c");
    d.delete_back(false);
    d.delete_back(false);
    assert_eq!(show(&d), "|c");
    let mut d = doc("[ab]c|");
    d.select_line();
    assert_eq!(show(&d), "[abc]");
    d.set_cursor(1, 99, false);
    assert_eq!(show(&d), "a[bc]");
}

#[test]
fn lines_are_moved_copied_deleted_indented_and_made_comments() {
    let mut d = doc("one\nt|wo\nthree");
    d.move_lines(true);
    assert_eq!(show(&d), "one\nthree\nt|wo");
    d.move_lines(true);
    d.move_lines(false);
    d.move_lines(false);
    assert_eq!(show(&d), "t|wo\none\nthree");
    assert!(d.undo() && show(&d) == "one\nt|wo\nthree");
    d.move_lines(false);
    d.duplicate_lines();
    assert_eq!(d.text(), "two\ntwo\none\nthree");
    // A selection that ends at the start of a line goes with the lines above that line.
    let mut d = doc("a\n[b\nc\n]d");
    d.move_lines(false);
    assert_eq!(show(&d), "[b\nc\n]a\nd");
    let mut d = doc("a\nb|\nc");
    d.delete_lines();
    assert_eq!(show(&d), "a\n|c");
    let mut d = doc("a\nb|");
    d.delete_lines();
    assert_eq!(d.text(), "a");
    let mut d = doc("[x\n\ny]");
    d.indent_lines(true);
    assert_eq!(d.text(), "    x\n\n    y");
    d.indent_lines(false);
    assert_eq!(show(&d), "[x\n\ny]");
    let mut d = doc("  [a\n\n    b]");
    d.toggle_comment();
    assert_eq!(d.text(), "  // a\n\n  //   b");
    d.toggle_comment();
    assert_eq!(d.text(), "  a\n\n    b");
    let mut d = doc("a|");
    d.tab();
    assert_eq!(show(&d), "a   |");
    let mut d = doc("[a\nb]");
    d.use_tabs = true;
    d.tab();
    assert_eq!(d.text(), "\ta\n\tb");
}

#[test]
fn copy_takes_the_selection_or_the_line_and_paste_gives_a_line_to_each_cursor() {
    let mut d = doc("one|\ntwo");
    assert_eq!(d.copy_text(), "one\n");
    assert!(d.cut() == "one\n" && d.text() == "two");
    let mut d = doc("[ab] [cd]");
    assert_eq!(d.copy_text(), "ab\ncd");
    assert!(d.cut() == "ab\ncd" && show(&d) == "| |");
    let mut d = doc("x|\ny|");
    d.paste("1\n2");
    assert_eq!(show(&d), "x1|\ny2|");
    d.paste("same");
    assert_eq!(show(&d), "x1same|\ny2same|");
    let mut d = doc("|");
    d.paste("a\r\nb");
    assert_eq!(show(&d), "a\nb|");
    assert!(d.undo() && show(&d) == "|");
}

#[test]
fn a_bracket_finds_its_match() {
    // Brackets are what the notation of these tests is made of: the cursor is put by hand.
    let pair = |text: &str, at: usize| {
        let mut d = Doc::new(text);
        d.set_cursor(at, at, false);
        d.bracket_pair()
    };
    let text = "f(a[1], {b: (c)})";
    // At a bracket, and after one; the nearest kind inside is not its match.
    assert_eq!(
        (pair(text, 1), pair(text, 17), pair(text, 16)),
        (Some((1, 16)), Some((16, 1)), Some((16, 1)))
    );
    assert_eq!((pair(text, 3), pair(text, 6), pair(text, 8)), (Some((3, 5)), Some((5, 3)), Some((8, 15))));
    assert_eq!((pair("abc", 2), pair("(", 1), pair("", 0), pair(")(", 1)), (None, None, None, None));
}
