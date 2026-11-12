//! The buffer (specs/13, "Code"): whatever is put in and taken out, it is the text a plain
//! string would be, and it knows where its lines are.

use quadra_code::Buffer;
use quadra_testing::fuzz::Rng;

/// The buffer says of every line what the model does.
fn check(buffer: &Buffer, model: &str) {
    assert_eq!(buffer.text(), model);
    assert_eq!((buffer.len(), buffer.is_empty()), (model.len(), model.is_empty()));
    assert_eq!(buffer.line_count(), model.split('\n').count());
    let mut start = 0;
    for (n, line) in model.split('\n').enumerate() {
        assert_eq!((buffer.line_start(n), buffer.line_end(n)), (start, start + line.len()), "line {n}");
        assert_eq!(buffer.line(n), line);
        assert_eq!((buffer.line_of(start), buffer.line_of(start + line.len())), (n, n));
        start += line.len() + 1;
    }
}

#[test]
fn changes_at_random_leave_the_text_a_string_would_have() {
    let mut rng = Rng::new(7);
    // Short things, things with line breaks, letters of several bytes, and one longer than a
    // piece of the buffer, so that pieces are cut, joined and dropped.
    let long = "a line of forty characters, give or take\n".repeat(300);
    let alphabet = ["a", "b", " ", "\n", "xy\nz", "ế", "\n\n", "tiếng việt", "😀", long.as_str()];
    let mut model = String::from("hello\nworld\n");
    let mut buffer = Buffer::new(&model);
    // An offset between characters, anywhere in the text.
    let offset = |rng: &mut Rng, model: &str, from: usize| {
        model.floor_char_boundary(from + rng.below(model.len() - from + 1))
    };
    for step in 0..4000 {
        if rng.below(3) == 0 {
            let start = offset(&mut rng, &model, 0);
            // Mostly a little, at times most of the text.
            let most = if rng.chance(5) { model.len() } else { model.floor_char_boundary(start + 200) };
            let end = offset(&mut rng, &model[..most], start);
            buffer.delete(start..end);
            model.replace_range(start..end, "");
        } else {
            // The long one now and then.
            let known = if rng.chance(3) { 10 } else { 9 };
            let (at, text) = (offset(&mut rng, &model, 0), *rng.pick(&alphabet[..known]));
            buffer.insert(at, text);
            model.insert_str(at, text);
        }
        if step % 50 == 0 {
            check(&buffer, &model);
            let start = offset(&mut rng, &model, 0);
            let end = offset(&mut rng, &model, start);
            assert_eq!(buffer.slice(start..end), &model[start..end]);
        }
    }
    check(&buffer, &model);
    // Everything can go, and what is left is a text of one empty line.
    buffer.delete(0..buffer.len());
    check(&buffer, "");
    buffer.insert(0, "again");
    check(&buffer, "again");
}

#[test]
fn a_file_keeps_its_line_ends_and_an_empty_text_has_a_line() {
    let mut buffer = Buffer::new("one\r\ntwo\r\n");
    assert!(buffer.crlf);
    assert_eq!(
        (buffer.line_count(), buffer.line(1).as_ref(), buffer.text().as_str()),
        (3, "two", "one\ntwo\n")
    );
    buffer.insert(buffer.line_end(0), "!");
    assert_eq!(buffer.file_text(), "one!\r\ntwo\r\n");
    let unix = Buffer::new("one\ntwo");
    assert!(!unix.crlf && unix.file_text() == "one\ntwo");
    let empty = Buffer::new("");
    assert_eq!(
        (empty.line_count(), empty.line(0).as_ref(), empty.line_start(0), empty.line_end(0)),
        (1, "", 0, 0)
    );
}

#[test]
fn a_large_file_knows_its_lines_and_a_line_without_end_is_held_too() {
    // Ten megabytes of lines of every length up to eighty.
    let mut rng = Rng::new(11);
    let mut model = String::new();
    let mut starts = vec![0];
    while model.len() < 10 << 20 {
        model.extend(std::iter::repeat_n('x', rng.below(80)));
        model.push('\n');
        starts.push(model.len());
    }
    let mut buffer = Buffer::new(&model);
    assert_eq!((buffer.len(), buffer.line_count()), (model.len(), starts.len()));
    for _ in 0..2000 {
        let n = rng.below(starts.len() - 1);
        assert_eq!((buffer.line_start(n), buffer.line_end(n)), (starts[n], starts[n + 1] - 1));
        assert_eq!(buffer.line_of(starts[n]), n);
    }
    // Typing in the middle moves what is after it, and nothing before.
    let middle = starts.len() / 2;
    for (k, letter) in ["l", "e", "t", "\n"].iter().enumerate() {
        buffer.insert(starts[middle] + k, letter);
    }
    assert_eq!(buffer.line(middle), "let");
    assert_eq!(
        (buffer.line_start(middle - 1), buffer.line_start(middle + 2)),
        (starts[middle - 1], starts[middle + 1] + 4)
    );
    assert_eq!(buffer.line_count(), starts.len() + 1);
    // A megabyte with no line break in it is one line, in many pieces.
    let mut one = Buffer::new(&"é".repeat(500_000));
    one.insert(500_000, "\n");
    assert_eq!((one.line_count(), one.line_end(0), one.line(1).len()), (2, 500_000, 500_000));
    one.delete(1000..999_001);
    assert_eq!((one.len(), one.line_count()), (2000, 1));
}
