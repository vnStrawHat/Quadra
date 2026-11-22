//! A log view as tall as its rows up to a most, and JSON lines as their message and fields
//! (specs/13, "Data").

use std::rc::Rc;

use quadra_data::{Level, Log, log_view};
use quadra_design::dark;
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;

const AT: i64 = 1_793_966_400_000; // 12:00 UTC

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

fn height(h: &Harness) -> f32 {
    h.tree.window_rect(h.query(|n| n.semantics.role == Role::Table)[0]).height
}

#[test]
fn a_view_is_as_tall_as_its_rows_up_to_the_most() {
    let log = Log::new();
    log.append((0..3).map(|i| (AT + i, format!("INFO line {i}"))));
    let wraps = signal(false);
    let mut h = page(&log, move |log| log_view(log).bar(false).fit(5).wrap(move || wraps.get()));
    assert_eq!(height(&h), 3.0 * 18.0);
    log.append((3..20).map(|i| (AT + i, format!("INFO line {i}"))));
    settle(&mut h);
    assert!(height(&h) == 5.0 * 18.0 && h.has_text("INFO line 19"));
    // Wrapped, a line counts as the rows it takes.
    log.clear();
    log.append([(AT, "INFO short".to_string()), (AT + 1, format!("INFO {}", "wide ".repeat(300)))]);
    wraps.set(true);
    settle(&mut h);
    assert!(height(&h) >= 3.0 * 18.0 && height(&h) <= 5.0 * 18.0, "{}", height(&h));
    // No line: no row.
    log.clear();
    settle(&mut h);
    assert_eq!(height(&h), 0.0);
}

#[test]
fn a_json_line_shows_as_its_message_and_fields_and_as_it_came_when_off() {
    let log = Log::new();
    let raw = r#"{"time":"12:00","level":"info","msg":"Error budget fine","user":7,"http":{"status":502}}"#;
    log.append_leveled(0, [(AT, Level::Info, raw)]);
    let fields = signal(true);
    let mut h = page(&log, move |log| log_view(log).bar(false).levels().fields(move || fields.get()));
    let said = "Error budget fine user=7 http.status=502";
    assert!(h.has_text(said) && h.has_text("INFO"), "{}", h.semantics());
    // The copy is what shows.
    h.get_by_text(said).click();
    h.key(Key::C, Mods::CTRL);
    assert_eq!(h.clipboard.read_text().as_deref(), Some(format!("12:00:00.000 {said}").as_str()));
    fields.set(false);
    settle(&mut h);
    assert!(h.has_text(raw) && !h.has_text(said));
}
