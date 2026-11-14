//! Paragraphs on the fake backend: 0.6 em per grapheme, so at size 10 every character is
//! 6 px wide and a line is 14 px high (specs/08, "Paragraphs", "Shaping cache", Tests).

use quadra_prim::Size;
use quadra_text::{FakeBackend, Paragraph, Shaping, TextStyle, TextSystem};

fn system() -> TextSystem {
    TextSystem::new(Box::new(FakeBackend::new()))
}

fn style() -> TextStyle {
    TextStyle { size: 10.0, ..TextStyle::default() }
}

/// The text of each line.
fn lines(p: &Paragraph) -> Vec<&str> {
    p.lines().iter().map(|l| &p.text()[l.start..l.end]).collect()
}

#[test]
fn words_wrap_at_spaces_and_trailing_spaces_hang() {
    let mut ts = system();
    let mut p = Paragraph::new(&mut ts, "one two three four", style());
    assert_eq!(p.wrap(&mut ts, f32::INFINITY), Size::new(108.0, 14.0));
    // 48 px: "one two " is 8 characters, of which the trailing space hangs past the edge.
    assert_eq!(p.wrap(&mut ts, 48.0), Size::new(42.0, 42.0));
    assert_eq!(lines(&p), ["one two ", "three ", "four"]);
    assert_eq!(p.lines()[1].width, 30.0); // without its trailing space
    assert_eq!((p.lines()[0].y, p.lines()[2].y), (0.0, 28.0));
}

#[test]
fn newlines_break_and_a_word_too_wide_breaks_between_graphemes() {
    let mut ts = system();
    let mut p = Paragraph::new(&mut ts, "ab\n\ncdefghij k", style());
    p.wrap(&mut ts, 24.0); // four characters a line
    assert_eq!(lines(&p), ["ab", "", "cdef", "ghij ", "k"]);
    assert_eq!(p.size(), Size::new(24.0, 70.0));

    let mut flag = Paragraph::new(&mut ts, "a\u{1F1FB}\u{1F1F3}e\u{301}b", style());
    flag.wrap(&mut ts, 12.0); // two graphemes a line: a cluster is never cut
    assert_eq!(lines(&flag), ["a\u{1F1FB}\u{1F1F3}", "e\u{301}b"]);
    let mut empty = Paragraph::new(&mut ts, "", style());
    assert_eq!(empty.wrap(&mut ts, 100.0), Size::new(0.0, 14.0)); // one empty line
}

#[test]
fn a_word_measured_to_fit_is_not_broken_by_float_error() {
    let mut ts = system();
    let odd = TextStyle { size: 7.3, ..TextStyle::default() };
    let mut p = Paragraph::new(&mut ts, "abcdefghijk mnopqrstu", odd);
    let natural = p.wrap(&mut ts, f32::INFINITY).width;
    assert_eq!(p.wrap(&mut ts, natural).height, p.style().line_px()); // still one line
}

#[test]
fn a_new_width_breaks_lines_again_without_shaping() {
    let mut ts = system();
    let mut p = Paragraph::new(&mut ts, "the quick brown fox jumps over the lazy dog", style());
    let stats = ts.take_stats();
    assert_eq!((stats.words_shaped, stats.word_hits), (9, 8)); // "the" twice, one space shape
    p.wrap(&mut ts, 120.0);
    p.wrap(&mut ts, 60.0);
    p.wrap(&mut ts, 60.0); // the same width: nothing
    let stats = ts.take_stats();
    assert_eq!((stats.words_shaped, stats.paragraphs_broken), (0, 2));
}

#[test]
fn lines_that_fit_stay_as_they_are_at_any_wider_width() {
    let mut ts = system();
    let mut p = Paragraph::new(&mut ts, "short\ntext", style());
    assert_eq!(p.wrap(&mut ts, 500.0), Size::new(30.0, 28.0));
    ts.take_stats();
    p.wrap(&mut ts, 400.0);
    p.wrap(&mut ts, 30.0); // exactly its own width
    assert_eq!(ts.take_stats().paragraphs_broken, 0);
    p.wrap(&mut ts, 24.0); // narrower than its widest line
    assert_eq!(ts.take_stats().paragraphs_broken, 1);
    assert_eq!(lines(&p), ["shor", "t", "text"]);
}

#[test]
fn a_changed_text_shapes_only_the_words_it_had_not_seen() {
    let mut ts = system();
    let mut p = Paragraph::new(&mut ts, "alpha beta gamma", style());
    ts.take_stats();
    assert!(!p.set_text(&mut ts, "alpha beta gamma")); // equal: nothing at all
    assert_eq!(ts.take_stats().word_hits, 0);
    assert!(p.set_text(&mut ts, "alpha beta gamma delta"));
    assert_eq!(ts.take_stats().words_shaped, 1);
    // Another size is other words; the same size again is a hit.
    assert!(p.set_style(&mut ts, &TextStyle { size: 20.0, ..style() }));
    assert_eq!(ts.take_stats().words_shaped, 5);
    assert!(!p.set_style(&mut ts, &TextStyle { size: 20.0, ..style() }));
    assert_eq!(p.wrap(&mut ts, f32::INFINITY).width, 22.0 * 12.0);
}

#[test]
fn the_cache_is_bounded_and_keeps_what_is_in_use() {
    let mut ts = system();
    ts.set_cache_limit(8);
    let face = ts.face_for(&style());
    for round in 0..3 {
        for n in 0..20 {
            ts.word(&format!("w{n}"), face, 10.0);
            ts.word("often", face, 10.0); // used all the time: never shaped again
        }
        let stats = ts.take_stats();
        assert_eq!(stats.words_shaped, if round == 0 { 21 } else { 20 }, "round {round}");
    }
}

#[test]
fn glyphs_of_a_line_start_at_zero_and_stops_follow_graphemes() {
    let mut ts = system();
    let mut p = Paragraph::new(&mut ts, "ab cd e\u{301}f", style());
    p.wrap(&mut ts, 18.0);
    assert_eq!(lines(&p), ["ab ", "cd ", "e\u{301}f"]);
    let (glyphs, origin) = p.line_glyphs(&p.lines()[1]);
    let xs: Vec<f32> = glyphs.iter().map(|g| g.x - origin).collect();
    assert_eq!(xs, [0.0, 6.0, 12.0]); // c, d, and the space
    assert_eq!(glyphs[0].id, 'c' as u16);
    // "e" and its accent are one stop: an offset inside the cluster snaps to its start.
    assert_eq!((p.x_at(6), p.x_at(7), p.x_at(9), p.x_at(10)), (36.0, 36.0, 42.0, 48.0));
    assert_eq!(p.metrics(), (8.0, 2.0));
}

#[test]
fn an_unknown_family_falls_back_and_loaded_fonts_are_known() {
    let mut ts = system();
    let fancy = TextStyle { family: "No Such Font".into(), ..style() };
    assert_eq!(ts.face_for(&fancy), ts.face_for(&style()));
    let bold = TextStyle { weight: 700, ..style() };
    assert_ne!(ts.face_for(&bold), ts.face_for(&style()));
    let names = ts.backend().load_fonts(&[b"font bytes"]).unwrap();
    assert!(ts.backend().families().contains(&names[0]));
    let loaded = TextStyle { family: names[0].as_str().into(), ..style() };
    assert_ne!(ts.face_for(&loaded), ts.face_for(&style()));
}

#[test]
fn a_number_is_composed_from_glyphs_shaped_once() {
    let mut ts = system();
    let face = ts.face_for(&style());
    // The first numbers of a face are shaped like any word: few windows show many.
    for n in 0..64 {
        ts.word(&format!("{n}"), face, 10.0);
    }
    assert_eq!(ts.take_stats().words_shaped, 64);
    let first = ts.word("12.50", face, 10.0);
    assert_eq!(first.width, 30.0);
    assert_eq!(
        first.glyphs.iter().map(|g| (g.x, g.cluster)).collect::<Vec<_>>(),
        [(0.0, 0), (6.0, 1), (12.0, 2), (18.0, 3), (24.0, 4)]
    );
    ts.take_stats();
    // A thousand numbers never seen before: nothing is shaped and the cache does not grow.
    for n in 0..1000 {
        ts.word(&format!("{n}.{:02}", n % 100), face, 10.0);
    }
    let stats = ts.take_stats();
    assert_eq!((stats.words_composed, stats.words_shaped, stats.word_hits), (1000, 0, 0));
    // Text that is not a number is shaped.
    ts.word("7", face, 20.0);
    ts.word("7th", face, 20.0);
    assert_eq!(ts.take_stats().words_shaped, 1);
}

/// On the fake backend a character is 0.6 of the size wide.
#[test]
fn runs_are_set_on_one_pen_and_a_word_across_them_never_breaks() {
    let mut ts = TextSystem::new(Box::new(FakeBackend::new()));
    let style = TextStyle { size: 10.0, ..TextStyle::default() };
    // "cd" is twice as large, and glued to "ef": "cdef" is one word of two runs.
    let large = Shaping { size: Some(20.0), ..Shaping::default() };
    let runs = vec![(3, Shaping::default()), (5, large)];
    let mut p = Paragraph::rich(&mut ts, "ab cdef gh", style.clone(), runs);
    assert_eq!([p.x_at(3), p.x_at(5), p.x_at(7), p.x_at(10)], [18.0, 42.0, 54.0, 72.0]);
    assert_eq!(
        [p.run_at(0), p.run_at(3), p.run_at(4), p.run_at(5)],
        [(0, 10.0), (1, 20.0), (1, 20.0), (2, 10.0)]
    );
    // 50 px: "ab " and "cdef" do not fit on a line, nor "cdef " and "gh"; where the runs
    // meet is no place to break.
    p.wrap(&mut ts, 50.0);
    let lines: Vec<_> = p.lines().iter().map(|line| (line.start, line.end, line.width)).collect();
    assert_eq!(lines, [(0, 3, 12.0), (3, 8, 36.0), (8, 10, 12.0)]);
    // What a run does not say it has from the paragraph's style, when that changes too.
    p.set_style(&mut ts, &TextStyle { size: 20.0, ..style });
    assert_eq!([p.x_at(3), p.x_at(5), p.x_at(7)], [36.0, 60.0, 84.0]);
}

#[test]
fn a_run_that_is_a_box_is_as_wide_as_it_says_in_the_word_around_it() {
    let mut ts = system();
    // "ab " then a box of 25 px glued to a comma, then " cd": the box has no glyph.
    let boxed = Shaping { advance: Some(25.0), ..Shaping::default() };
    let runs = vec![(3, Shaping::default()), (6, boxed)];
    let mut p = Paragraph::rich(&mut ts, "ab \u{FFFC}, cd", style(), runs);
    assert_eq!([p.x_at(3), p.x_at(6), p.x_at(7), p.x_at(10)], [18.0, 43.0, 49.0, 67.0]);
    assert_eq!(p.wrap(&mut ts, f32::INFINITY), Size::new(67.0, 14.0));
    let glyphs = p.line_glyphs(&p.lines()[0]).0;
    assert_eq!(glyphs.len(), 7, "seven characters have glyphs, the box none");
    // The box and its comma are one word: 45 px takes "ab " and not them.
    p.wrap(&mut ts, 45.0);
    assert_eq!(lines(&p), ["ab ", "\u{FFFC}, ", "cd"]);
    assert_eq!(p.spans(3..6, 0.0).next().map(|part| (part.x, part.y, part.width)), Some((0.0, 14.0, 25.0)));
}

#[test]
fn features_set_for_a_family_shape_its_words_again() {
    let mut ts = system();
    let face = ts.face_for(&style());
    ts.word("word", face, 10.0);
    ts.word("word", face, 10.0);
    ts.set_features("Fake Sans", &[quadra_text::Feature::off(b"liga")]);
    ts.word("word", face, 10.0);
    let stats = ts.take_stats();
    assert_eq!((stats.words_shaped, stats.word_hits), (2, 1));
}
