//! The fire demo through the pseudo console with its output read and dropped, no engine and
//! no window (specs/14, "Terminal throughput"): the console host's ceiling, which a terminal
//! on Windows cannot pass.
//!
//!     doomfire-conpty [--seconds N] [--size COLSxROWS]

use std::sync::mpsc::channel;

use quadra_bench_doomfire::args;
use quadra_term::{Pty, PtyOptions};

fn main() {
    let args = args();
    let (cols, rows) = args.size.unwrap_or((200, 50));
    let here = std::env::current_exe().expect("this program's path");
    let demo = here.with_file_name("doomfire.exe");
    let out = std::env::current_dir().expect("a folder").join(".tmp/doomfire-conpty.txt");
    let seconds = args.seconds.unwrap_or(10.0).to_string();
    let demo_args = vec!["--seconds".into(), seconds, "--out".into(), out.display().to_string()];
    let options = PtyOptions {
        program: Some((demo.display().to_string(), demo_args)),
        cwd: None,
        env: Vec::new(),
        rows,
        cols,
    };
    let (ended, end) = channel();
    let _pty = Pty::spawn(
        options,
        |_| Vec::new(),
        move |code| {
            let _ = ended.send(code);
        },
    )
    .expect("the pseudo console");
    let _ = end.recv();
    let said = std::fs::read_to_string(&out).unwrap_or_default();
    println!("doomfire-conpty: {said}");
}
