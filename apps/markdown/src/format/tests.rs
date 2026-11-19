use super::{Change, Format, format};

/// The text after the change, and what is selected in it.
fn apply(text: &str, sel: std::ops::Range<usize>, what: Format) -> (String, String) {
    let Change { edits, selection } = format(text, sel, what);
    let mut out = text.to_string();
    for (range, new) in edits.iter().rev() {
        out.replace_range(range.clone(), new);
    }
    let picked = out[selection].to_string();
    (out, picked)
}

#[test]
fn text_styles_wrap_the_selection_the_word_or_an_empty_pair() {
    assert_eq!(apply("say hello now", 4..9, Format::Bold), ("say **hello** now".into(), "hello".into()));
    // The word at the caret.
    assert_eq!(apply("say hello now", 6..6, Format::Italic), ("say *hello* now".into(), "hello".into()));
    // No word: an empty pair, the caret between.
    let (text, picked) = apply("say ", 4..4, Format::Code);
    assert_eq!((text.as_str(), picked.as_str()), ("say ``", ""));
    let Change { selection, .. } = format("say ", 4..4, Format::Code);
    assert_eq!(selection, 5..5);
    assert_eq!(apply("a b", 2..3, Format::Strike).0, "a ~~b~~");
}

#[test]
fn a_style_already_there_is_taken_off_inside_or_outside() {
    assert_eq!(apply("say **hello** now", 4..13, Format::Bold), ("say hello now".into(), "hello".into()));
    assert_eq!(apply("say **hello** now", 6..11, Format::Bold), ("say hello now".into(), "hello".into()));
    assert_eq!(apply("x `code` y", 3..7, Format::Code), ("x code y".into(), "code".into()));
}

#[test]
fn headings_set_the_level_of_each_line_or_make_it_a_paragraph() {
    assert_eq!(apply("Title\nbody", 2..2, Format::Heading(1)).0, "# Title\nbody");
    assert_eq!(apply("### Title", 4..4, Format::Heading(2)).0, "## Title");
    assert_eq!(apply("## Title", 4..4, Format::Heading(2)).0, "Title", "the same level again");
    assert_eq!(apply("one\ntwo", 0..7, Format::Heading(3)).0, "### one\n### two");
}

#[test]
fn lists_and_quotes_toggle_their_prefix_on_every_line_touched() {
    assert_eq!(
        apply("one\ntwo\nthree", 1..9, Format::Bullet),
        ("- one\n- two\n- three".into(), "ne\n- two\n- t".into())
    );
    assert_eq!(apply("- one\n- two", 0..11, Format::Bullet).0, "one\ntwo", "every line has it: off");
    assert_eq!(
        apply("- one\n- two", 0..11, Format::Numbered).0,
        "1. one\n2. two",
        "another list's: replaced"
    );
    assert_eq!(apply("one", 0..0, Format::Task).0, "- [ ] one");
    assert_eq!(apply("  one", 3..3, Format::Quote).0, "  > one", "after the indent");
    assert_eq!(apply("- [x] done", 7..7, Format::Task).0, "done");
}

#[test]
fn a_link_or_an_image_selects_what_comes_next() {
    assert_eq!(apply("see docs", 4..8, Format::Link), ("see [docs](url)".into(), "url".into()));
    assert_eq!(apply("see ", 4..4, Format::Link), ("see [text](url)".into(), "text".into()));
    assert_eq!(apply("", 0..0, Format::Image), ("![alt](path)".into(), "alt".into()));
}

#[test]
fn blocks_go_on_lines_of_their_own_with_blank_lines_around() {
    let (text, picked) = apply("para\nnext", 2..2, Format::Table);
    assert!(text.starts_with("para\n\n| Column 1 |") && text.ends_with("|  |  |  |\n\nnext"), "{text:?}");
    assert_eq!(picked, "Column 1");
    assert_eq!(apply("para", 4..4, Format::Rule).0, "para\n\n---");
    assert_eq!(apply("para\n\n", 6..6, Format::Rule).0, "para\n\n---");
    // An empty block of code, the caret after its opening fence; lines selected, fenced.
    let Change { selection, .. } = format("", 0..0, Format::CodeBlock);
    assert_eq!((apply("", 0..0, Format::CodeBlock).0, selection), ("```\n\n```".into(), 3..3));
    assert_eq!(apply("a\nb\nc", 2..3, Format::CodeBlock).0, "a\n```\nb\n```\nc");
}
