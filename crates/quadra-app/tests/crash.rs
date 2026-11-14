//! The log and crash reports (specs/15, "Log and crashes"): the log in a file a day, and a
//! report of a panic, told once. A binary of its own: the log and the hook of panics are the
//! process's.

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use quadra_app::{App, AppDirs, log};
use quadra_platform::HeadlessPlatform;
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;

#[test]
fn the_log_goes_to_a_file_a_day_and_a_panic_leaves_a_report_told_once() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/tests/crashes");
    let _ = fs::remove_dir_all(&dir);
    let dirs = AppDirs::under(&dir);
    let platform = HeadlessPlatform::new();
    let app = App::with(Rc::new(platform), NullRenderer::new(), Box::new(FakeBackend::new()));
    let app = app.dirs(dirs.clone());
    log::info("test", "a line of its own");
    let today = fs::read_dir(dirs.data.join("logs")).unwrap().flatten().next().unwrap().path();
    assert!(fs::read_to_string(&today).unwrap().contains("INFO test: a line of its own"));
    assert!(log::recent(1)[0].ends_with("INFO test: a line of its own"));

    assert_eq!(app.last_crash(), None);
    let _ = std::thread::spawn(|| panic!("on purpose")).join();
    let report = app.last_crash().unwrap();
    let text = fs::read_to_string(report).unwrap();
    assert!(text.contains("on purpose") && text.contains("a line of its own"), "{text}");
    assert_eq!(app.last_crash(), None); // told once
}
