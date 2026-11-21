//! A log as a panel of an app's own (specs/13, "Data"): no bar, its size and row, a ground,
//! the time in the app's time and colour, lines in the colour of their level, and a filter
//! by level.

use std::rc::Rc;

use quadra_data::{Level, Log, log_view};
use quadra_design::{dark, token};
use quadra_render::{FLAG_GLYPH, Step};
use quadra_testing::Harness;
use quadra_tree::{Role, Token};

const AT: i64 = 1_793_966_400_000; // 12:00 UTC

fn lines() -> Vec<(i64, &'static str)> {
    vec![
        (AT, "INFO started on :8080"),
        (AT + 1000, r#"{"msg":"slow disk","level":"warn"}"#),
        (AT + 2000, "time=12:00:02 level=error msg=\"no route\""),
        (AT + 3000, "FATAL out of memory"),
        (AT + 4000, "\u{1b}[34mINFO\u{1b}[0m in blue, as it says"),
        (AT + 5000, "an error in passing, with no level"),
        (AT + 6000, "error: bucket not found"),
        (AT + 7000, "Warning: disk at 91%"),
        (AT + 8000, "E1121 10:02:03.123456    1 reflector.go:138] failed to list"),
        (AT + 9000, "I1121 10:02:04.000001    1 leader.go:42] became leader"),
    ]
}

#[test]
fn a_level_is_read_from_the_words_and_filters_with_the_text() {
    let log = Log::new();
    log.append(lines());
    let levels: Vec<Level> = (0..10).map(|i| log.level(i)).collect();
    let said = [Level::Info, Level::Warn, Level::Error, Level::Error, Level::Info, Level::None];
    let said = [&said[..], &[Level::Error, Level::Warn, Level::Error, Level::Info]].concat();
    assert_eq!(levels, said);
    log.set_level(Level::Warn);
    assert_eq!(log.listed(), 6);
    log.set_filter("memory", false, false);
    assert_eq!((log.listed(), log.line(0).map(|l| l.0)), (1, Some(3)));
    // Lines that come are tested as they come; Dim keeps them, faint.
    log.push(AT + 6000, "WARN memory low");
    log.push(AT + 7000, "INFO memory fine");
    assert_eq!(log.listed(), 2);
    log.set_filter("", false, true);
    log.set_level(Level::Error);
    assert_eq!((log.listed(), log.line(0).unwrap().3, log.line(2).unwrap().3), (12, true, false));
    log.set_level(Level::None);
    log.set_filter("", false, false);
    assert_eq!(log.listed(), 12);
}

/// The colour of the glyphs of the text that says `words`, in the window.
fn ink(h: &Harness, words: &str) -> [f32; 4] {
    let node = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == words)[0];
    let r = h.tree.window_rect(node);
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, world } => Some((h.tree.scene().layer(*id), world.transform)),
        Step::Run { .. } => None,
    });
    let mut glyphs = layers.flat_map(|(layer, at)| layer.instances().iter().map(move |i| (i, at)));
    let (glyph, _) = glyphs
        .find(|(i, at)| {
            let p = at.apply(quadra_prim::Point::new(i.bounds[0], i.bounds[1] + i.bounds[3] / 2.0));
            i.flags & FLAG_GLYPH != 0 && r.contains(p)
        })
        .expect("a glyph of the text");
    glyph.color
}

#[test]
fn a_panel_has_no_bar_its_own_size_a_ground_local_times_and_lines_in_their_level() {
    let log = Log::new();
    log.append(lines());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    let shown = log.clone();
    h.mount(move || {
        let panel = log_view(&shown).bar(false).text(15.0, 24.0).ground().levels();
        quadra_design::root().child(panel.time_color(token::TEXT_2).offset(7 * 3600))
    });
    h.frame();
    while h.frame_if_needed().is_some() {}
    // No bar: no filter, no Follow.
    assert!(h.query(|n| n.semantics.role == Role::TextInput).is_empty());
    assert!(h.query(|n| n.semantics.role == Role::Button).is_empty());
    // Rows of 24, the times at UTC+7.
    let first = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "INFO started on :8080")[0];
    assert_eq!(h.tree.window_rect(first).height, 24.0);
    assert!(h.has_text("19:00:00.000") && !h.has_text("12:00:00.000"));
    // The ground: the canvas, 6 px round, the first row 6 px down and 12 in.
    let table = h.query(|n| n.semantics.role == Role::Table)[0];
    let ground = h.tree.paint_style(table);
    let theme = dark().theme();
    let color = |t: Token| theme.resolve(t.into()).to_array();
    assert_eq!(ground.background, theme.resolve(token::CANVAS.into()));
    let (frame, row) = (h.tree.window_rect(table), h.tree.window_rect(first));
    assert_eq!(row.y - frame.y, 6.0);
    // The time at the ground's edge, a line's text two letters (9 px each) after it.
    let time = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "19:00:00.000")[0];
    let (time, text) = (h.tree.window_rect(time), h.tree.window_rect(first));
    assert_eq!((time.x - frame.x, text.x - time.x), (12.0, 14.0 * 9.0));
    // Each line in its level's colour; the escape sequence's blue wins; no level, the text.
    let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01);
    assert!(near(ink(&h, "19:00:00.000"), color(token::TEXT_2)));
    assert!(near(ink(&h, "INFO started on :8080"), color(token::OK)));
    assert!(near(ink(&h, r#"{"msg":"slow disk","level":"warn"}"#), color(token::WARN)));
    assert!(near(ink(&h, "FATAL out of memory"), color(token::ERROR)));
    assert!(near(ink(&h, "INFO in blue, as it says"), color(token::ANSI[4])));
    assert!(near(ink(&h, "an error in passing, with no level"), color(token::TEXT)));
}

#[test]
fn the_bar_lists_only_warnings_and_errors_by_its_warn() {
    let log = Log::new();
    log.append(lines());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    let shown = log.clone();
    h.mount(move || quadra_design::root().child(log_view(&shown)));
    h.frame();
    h.get_by_role(Role::Button, "Warn").click();
    h.frame();
    assert_eq!(log.listed(), 6);
    assert!(h.has_text("FATAL out of memory") && !h.has_text("INFO started on :8080"));
    h.get_by_role(Role::Button, "Warn").click();
    h.frame();
    assert_eq!(log.listed(), 10);
}

#[test]
fn a_time_may_be_to_the_second() {
    let log = Log::new();
    log.push(AT + 41_208, "step 3/7");
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    let shown = log.clone();
    h.mount(move || quadra_design::root().child(log_view(&shown).bar(false).ground().millis(false)));
    h.frame();
    while h.frame_if_needed().is_some() {}
    assert!(h.has_text("12:00:41") && !h.has_text("12:00:41.208"));
    // Copied as it shows.
    h.click(200.0, 12.0);
    h.key(quadra_platform::Key::C, quadra_platform::Mods::CTRL);
    assert_eq!(h.clipboard.read_text().as_deref(), Some("12:00:41 step 3/7"));
}
