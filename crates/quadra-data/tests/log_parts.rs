//! A log view for a dashboard's log tab (specs/13, "Data"): its matches marked, long lines
//! wrapped, merged sources, a line's menu and note, a header, following from outside, the
//! empty row, and the time switched in place.

use std::cell::Cell;
use std::rc::Rc;

use quadra_data::{Log, log_view};
use quadra_design::{MenuItem, dark, token};
use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_render::Step;
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role};

const AT: i64 = 1_793_966_400_000; // 12:00 UTC

fn requests(from: usize, n: usize) -> Vec<(i64, String)> {
    (from..from + n).map(|i| (AT + i as i64 * 250, format!("INFO request {i} ok"))).collect()
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

fn page(log: &Log, view: impl Fn(&Log) -> quadra_data::LogView + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    let shown = log.clone();
    h.mount(move || quadra_design::root().child(view(&shown)));
    settle(&mut h);
    h
}

/// The text whose name starts with `words`.
fn text(h: &Harness, words: &str) -> NodeId {
    h.query(|n| n.semantics.role == Role::Text && n.semantics.name.starts_with(words))[0]
}

/// The quads painted in the colour of a token at an alpha, and their radii.
fn quads(h: &Harness, fill: [f32; 4]) -> Vec<[f32; 4]> {
    let near = |a: [f32; 4]| a.iter().zip(fill).all(|(x, y)| (x - y).abs() < 0.01);
    // A layer may be drawn by more than one step: each once.
    let mut ids = Vec::new();
    for step in h.renderer.steps() {
        if let Step::Layer { id, .. } = step
            && !ids.contains(id)
        {
            ids.push(*id);
        }
    }
    let layers = ids.into_iter().map(|id| h.tree.scene().layer(id));
    layers.flat_map(|layer| layer.instances().iter()).filter(|i| near(i.color)).map(|i| i.radii).collect()
}

#[test]
fn a_match_is_marked_and_a_long_line_wraps_when_switched() {
    let log = Log::new();
    log.append(requests(0, 20));
    log.push(AT + 9000, &format!("INFO {}", "wide ".repeat(60)));
    let wraps = signal(false);
    let mut h = page(&log, move |log| log_view(log).bar(false).wrap(move || wraps.get()));
    // The filter's match has a ground of the accent at 32 %, its corners round by 2.
    log.set_filter("request 1", false, false);
    settle(&mut h);
    let accent = dark().theme().resolve(token::ACCENT.into()).alpha(0.32).to_array();
    let marks = quads(&h, accent);
    assert!(!marks.is_empty() && marks.iter().all(|r| *r == [2.0; 4]), "{marks:?}");
    // Wrapped, a long line is as tall as its rows; the others a row.
    log.set_filter("", false, false);
    let row = |h: &Harness, words| h.tree.window_rect(text(h, words)).height;
    settle(&mut h);
    assert_eq!(row(&h, "INFO wide"), 18.0);
    wraps.set(true);
    settle(&mut h);
    assert!(row(&h, "INFO wide") >= 36.0 && row(&h, "INFO request 19 ok") == 18.0);
}

#[test]
fn merged_sources_read_in_time_order_each_with_its_dot() {
    let log = Log::new();
    log.append_from(0, [(AT, "api up"), (AT + 2000, "api ready")]);
    let mut h = page(&log, |log| log_view(log).bar(false));
    let time = |h: &Harness| h.tree.window_rect(text(h, "12:00:00.000")).x;
    let before = time(&h);
    // One source: no dot. A second: a round dot of 8 px before each time, in the source's
    // hue, the time 16 px further.
    let hue = dark().theme().resolve(token::HUES[1].into()).to_array();
    assert!(quads(&h, hue).is_empty());
    log.append_from(1, [(AT + 1000, "db up")]);
    settle(&mut h);
    let said: Vec<String> = (0..3).filter_map(|p| log.line(p).map(|l| l.2)).collect();
    assert_eq!(said, ["api up", "db up", "api ready"]);
    assert_eq!((quads(&h, hue), time(&h) - before), (vec![[4.0; 4]], 16.0));
}

#[test]
fn a_lines_menu_tints_it_while_open_and_a_note_shows_at_its_end() {
    let log = Log::new();
    log.append(requests(0, 10));
    let asked: Rc<Cell<Option<usize>>> = Rc::default();
    let notes: Signal<Option<(usize, String)>> = signal(None);
    let told = asked.clone();
    let mut h = page(&log, move |log| {
        let told = told.clone();
        let menu = move |place| {
            told.set(Some(place));
            vec![MenuItem::new("Copy line").on_select(move || notes.set(Some((place, "Copied".into()))))]
        };
        log_view(log).bar(false).line_menu(menu).notes(notes)
    });
    let line = h.tree.window_rect(text(&h, "INFO request 4 ok"));
    let at = Point::new(line.x + 20.0, line.y + line.height / 2.0);
    for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
        h.tree.pointer(input, MouseButton::Right, at, Mods::NONE, h.now());
    }
    settle(&mut h);
    assert_eq!((asked.get(), log.line(4).map(|l| l.2)), (Some(4), Some("INFO request 4 ok".into())));
    let tint = dark().theme().resolve(token::ACCENT.into()).alpha(0.12).to_array();
    assert_eq!(quads(&h, tint).len(), 1, "the line is tinted while the menu is open");
    h.get_by_role(Role::MenuItem, "Copy line").click();
    settle(&mut h);
    assert!(quads(&h, tint).is_empty() && h.query(|n| n.semantics.role == Role::MenuItem).is_empty());
    assert!(!h.query(|n| n.semantics.name == "INFO request 4 ok  Copied").is_empty(), "{}", h.semantics());
    // Closed by Escape: not tinted either.
    for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
        h.tree.pointer(input, MouseButton::Right, at, Mods::NONE, h.now());
    }
    settle(&mut h);
    h.key(Key::Escape, Mods::NONE);
    settle(&mut h);
    assert!(quads(&h, tint).is_empty());
}

#[test]
fn a_header_is_no_line_and_the_empty_row_says_why() {
    let log = Log::new();
    let mut h = page(&log, |log| {
        log_view(log).bar(false).header(|| "container api · previous run".into()).empty("No logs yet")
    });
    assert!(h.has_text("container api · previous run") && h.has_text("No logs yet"));
    log.append(requests(0, 5));
    settle(&mut h);
    assert!(!h.has_text("No logs yet") && h.has_text("INFO request 4 ok"));
    // The header is neither selected nor copied.
    h.clipboard.write_text("before");
    h.get_by_text("container api · previous run").click();
    h.key(Key::C, Mods::CTRL);
    assert_eq!(h.clipboard.read_text().as_deref(), Some("before"));
    log.set_filter("nothing like it", false, false);
    settle(&mut h);
    assert!(h.has_text("No line matches what is set in the bar above"));
}

#[test]
fn following_is_the_apps_switch_and_the_time_switches_in_place() {
    let log = Log::new();
    log.append(requests(0, 100));
    let (follow, stamps) = (signal(true), signal(true));
    let mut h =
        page(&log, move |log| log_view(log).bar(false).follow(follow).timestamps(move || stamps.get()));
    assert!(h.has_text("INFO request 99 ok"));
    follow.set(false);
    log.append(requests(100, 20));
    settle(&mut h);
    assert!(h.has_text("INFO request 99 ok") && !h.has_text("INFO request 119 ok"));
    follow.set(true);
    settle(&mut h);
    assert!(h.has_text("INFO request 119 ok"));
    // Scrolled away, the switch reads off; the time switched off keeps where it is.
    h.wheel(400.0, 300.0, 3.0);
    settle(&mut h);
    assert!(!follow.get() && !h.has_text("INFO request 119 ok"));
    let shown = |h: &Harness| -> Vec<String> {
        h.semantics()
            .lines()
            .filter(|l| l.contains("\"INFO request"))
            .map(str::trim)
            .map(String::from)
            .collect()
    };
    let before = shown(&h);
    stamps.set(false);
    settle(&mut h);
    assert!(h.query(|n| n.semantics.name.starts_with("12:00:")).is_empty() && !follow.get());
    assert_eq!(shown(&h), before);
}
