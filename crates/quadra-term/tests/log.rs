//! A session's log (specs/18, "Logging"): the text without its sequences, a stamped line at a
//! time, after what the file holds or over it. Files go under the project's `.tmp`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use quadra_term::vt::ResizePolicy;
use quadra_term::{Session, terminal};
use quadra_testing::Harness;

fn file(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/quadra-term-log").join(name);
    _ = std::fs::remove_file(&path);
    path
}

#[test]
fn the_log_holds_the_text_a_line_at_a_time() {
    let (mut h, slot) = (Harness::new(), Rc::new(RefCell::new(None)));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        terminal(&s)
    });
    let s: Session = slot.borrow_mut().take().unwrap();
    let path = file("one.log");
    s.start_log(&path, true, || "T".into()).unwrap();
    assert!(s.logging());
    s.feeder().feed(b"\x1b[31mhello\x1b[0m\r\nwor\x08ld\r\n\x1b]0;title\x07ta", Instant::now());
    s.feeder().feed(b"il", Instant::now());
    s.stop_log();
    assert!(!s.logging());
    s.feeder().feed(b"\r\nnot logged\r\n", Instant::now());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[T] hello\n[T] wold\n[T] tail\n");

    s.start_log(&path, true, || "U".into()).unwrap();
    s.feeder().feed(b"more\n", Instant::now());
    s.stop_log();
    assert!(std::fs::read_to_string(&path).unwrap().ends_with("[T] tail\n[U] more\n"), "appended");
    s.start_log(&path, false, || "V".into()).unwrap();
    s.feeder().feed(b"over\n", Instant::now());
    s.stop_log();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[V] over\n", "overwritten");
}
