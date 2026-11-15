//! A shell behind the pseudo console (specs/18, "Tests"): `cmd /c echo` gives its output and
//! its exit. Skipped where no console can be made.

#![cfg(windows)]

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use quadra_shell::{Pty, PtyOptions};

#[test]
fn a_command_gives_its_output_and_its_exit() {
    let out = Arc::new(Mutex::new(Vec::new()));
    let (seen, (ended, end)) = (out.clone(), channel());
    let options = PtyOptions {
        program: Some(("cmd.exe".into(), vec!["/c".into(), "echo".into(), "quadra-pty".into()])),
        rows: 24,
        cols: 80,
        ..PtyOptions::default()
    };
    let spawned = Pty::spawn(
        options,
        move |bytes| {
            seen.lock().unwrap().extend_from_slice(bytes);
            Vec::new()
        },
        move |code| _ = ended.send(code),
    );
    let Ok(_pty) = spawned else {
        eprintln!("no pseudo console here: skipped");
        return;
    };
    let code = end.recv_timeout(Duration::from_secs(30)).expect("the command ended");
    assert_eq!(code, 0);
    let text = String::from_utf8_lossy(&out.lock().unwrap()).into_owned();
    assert!(text.contains("quadra-pty"), "the output: {text:?}");
}
