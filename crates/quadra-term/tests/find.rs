//! Finding in a terminal (specs/18, "Find"): matches over the history and the screen, case
//! and whole words, the count, next and previous wrapping, the view scrolled to a match, output
//! found again after the wait, the matches painted. A box of 400 x 140 at size 10 holds 8 rows
//! of 64 columns (cells of 6 x 14).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra_design::token;
use quadra_prim::Color;
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{FindQuery, Session, terminal};
use quadra_testing::Harness;
use quadra_view::column;

fn term() -> (Harness, Session) {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(quadra_design::dark().theme()));
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).font(10.0, 14.0))
    });
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    (h, s)
}

fn feed(h: &mut Harness, s: &Session, bytes: &[u8]) {
    s.feeder().feed(bytes, Instant::now());
    h.settle();
}

/// Lines 0 to 29; `foo` on lines 2, 12 and 27, `Foo` on 20, `food` on 25.
fn lines(h: &mut Harness, s: &Session) {
    let mut out = String::new();
    for i in 0..30 {
        let word = match i {
            2 | 12 | 27 => " foo",
            20 => " Foo",
            25 => " food",
            _ => "",
        };
        out.push_str(&format!("line {i}{word}\r\n"));
    }
    feed(h, s, out.as_bytes());
}

fn query(text: &str, case: bool, word: bool) -> FindQuery {
    FindQuery { text: text.into(), case, word, pattern: false }
}

/// The first row that shows.
fn top(s: &Session) -> String {
    s.screen_text()[0].clone()
}

#[test]
fn matches_over_history_and_screen_with_case_and_whole_words() {
    let (mut h, s) = term();
    lines(&mut h, &s);
    s.find(&query("foo", false, false));
    assert_eq!(s.found().get_untracked().1, 5);
    s.find(&query("foo", true, false));
    assert_eq!(s.found().get_untracked().1, 4);
    s.find(&query("foo", false, true));
    assert_eq!(s.found().get_untracked().1, 4);
    s.find(&query("foo", true, true));
    assert_eq!(s.found().get_untracked().1, 3);
    s.find(&query("nothing", false, false));
    assert_eq!(s.found().get_untracked(), (0, 0));
    s.find(&FindQuery::default());
    assert_eq!(s.found().get_untracked(), (0, 0));
}

#[test]
fn the_current_match_starts_in_view_and_steps_wrapping_and_scrolls_into_view() {
    let (mut h, s) = term();
    lines(&mut h, &s);
    // The screen shows lines 23 to 29 and an empty row: the first match in view is line 25's.
    assert_eq!(top(&s), "line 23");
    s.find(&query("foo", false, false));
    h.settle();
    assert_eq!(s.found().get_untracked(), (4, 5));
    assert_eq!(top(&s), "line 23", "a match in view does not move it");
    s.find_next(false);
    s.find_next(false);
    h.settle();
    assert_eq!(s.found().get_untracked(), (1, 5), "next wraps to the first");
    // Line 2 is in the history: it stands in the middle, as near as the history allows.
    assert_eq!(top(&s), "line 0");
    s.find_next(true);
    h.settle();
    assert_eq!(s.found().get_untracked(), (5, 5), "the one before wraps to the last");
    assert_eq!(top(&s), "line 23");
    s.find_next(true);
    s.find_next(true);
    h.settle();
    assert_eq!(s.found().get_untracked(), (3, 5));
    assert_eq!(top(&s), "line 16", "line 20 in the middle of 8 rows");
}

#[test]
fn output_is_found_again_after_the_wait_and_the_current_match_is_kept() {
    let (mut h, s) = term();
    lines(&mut h, &s);
    s.find(&query("foo", false, false));
    s.find_next(true);
    assert_eq!(s.found().get_untracked(), (3, 5));
    feed(&mut h, &s, b"more foo\r\n");
    assert_eq!(s.found().get_untracked(), (3, 5), "not before the wait");
    h.advance(Duration::from_millis(200));
    h.settle();
    assert_eq!(s.found().get_untracked(), (3, 6));
}

/// The quads of a cell's height drawn in `color`: their x and width.
fn quads(h: &mut Harness, color: Color) -> Vec<(f32, f32)> {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    let near = |c: [f32; 4]| {
        (c[0] - color.r).abs() + (c[1] - color.g).abs() + (c[2] - color.b).abs() + (c[3] - color.a).abs()
            < 0.01
    };
    layers
        .flatten()
        .filter(|i| i.kind == Kind::Quad as u32 && near(i.color) && (i.bounds[3] - 14.0).abs() < 0.5)
        .map(|i| (i.bounds[0], i.bounds[2]))
        .collect()
}

#[test]
fn matches_are_painted_the_current_in_its_own_colour_and_one_that_wraps_on_each_row() {
    let (mut h, s) = term();
    let theme = quadra_design::dark().theme();
    let (found, current) = (theme.color(token::EDITOR_FIND), theme.color(token::EDITOR_FIND_CURRENT));
    // `abc` twice on the screen; then one across the wrap at column 64.
    feed(&mut h, &s, b"abc abc\r\n");
    s.find(&query("abc", false, false));
    h.settle();
    assert_eq!(quads(&mut h, current), [(8.0, 18.0)]);
    assert_eq!(quads(&mut h, found), [(32.0, 18.0)]);
    feed(&mut h, &s, format!("{}wxyz", " ".repeat(62)).as_bytes());
    s.find(&query("wxyz", false, false));
    h.settle();
    let mut parts = quads(&mut h, current);
    parts.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(parts, [(8.0, 12.0), (8.0 + 62.0 * 6.0, 12.0)]);
    s.find(&FindQuery::default());
    h.settle();
    assert!(quads(&mut h, current).is_empty() && quads(&mut h, found).is_empty());
}

#[test]
fn a_resize_that_reflows_finds_again() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(quadra_design::dark().theme()));
    let look =
        quadra_reactive::signal(quadra_term::TermLook { size: 10.0, line_height: 1.4, ..Default::default() });
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s: Session = slot.borrow_mut().take().unwrap();
    feed(&mut h, &s, format!("{}wxyz", " ".repeat(62)).as_bytes());
    s.find(&query("wxyz", false, false));
    h.settle();
    let current = quadra_design::dark().theme().color(token::EDITOR_FIND_CURRENT);
    assert_eq!(quads(&mut h, current).len(), 2, "across the wrap");
    // 40 columns: the line wraps at 40, the match is on its second row whole.
    look.update(|l| l.padding = quadra_prim::Edges::all(80.0));
    h.settle();
    assert_eq!(s.size().cols, 40);
    assert_eq!(s.found().get_untracked(), (1, 1));
    assert_eq!(quads(&mut h, current), [(80.0 + 22.0 * 6.0, 24.0)]);
}

#[test]
fn a_pattern_finds_by_a_regular_expression() {
    let (mut h, s) = term();
    lines(&mut h, &s);
    let pattern = |text: &str, case, word| FindQuery { pattern: true, ..query(text, case, word) };
    s.find(&pattern(r"line 2\d", false, false));
    assert_eq!(s.found().get_untracked().1, 10, "lines 20 to 29");
    s.find(&pattern("fo+d?", false, false));
    assert_eq!(s.found().get_untracked().1, 5);
    s.find(&pattern("fo+", true, true));
    assert_eq!(s.found().get_untracked().1, 3, "case and whole words as a literal's");
    s.find(&pattern("x*", false, false));
    assert_eq!(s.found().get_untracked(), (0, 0), "empty matches are none");
    s.find(&pattern("(unclosed", false, false));
    assert_eq!(s.found().get_untracked(), (0, 0), "not a pattern: nothing");
    s.find(&query("fo+", false, false));
    assert_eq!(s.found().get_untracked(), (0, 0), "a literal is no pattern");
}
