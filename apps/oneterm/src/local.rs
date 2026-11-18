//! A space's local shell: a session over `quadra-shell`'s pseudo console.

use std::rc::Rc;
use std::time::Instant;

use quadra_shell::{Pty, PtyOptions};
use quadra_term::vt::{Config, ResizePolicy};
use quadra_term::{Link, Session};

use crate::shells::Launch;

/// A session running a shell (the OS's when the launch names none) in a directory (the
/// app's own when it names none). The working directory shows from the start; Command
/// Prompt is asked to say it with each prompt (OSC 9;9), as it says nothing of it otherwise.
pub fn local_shell(launch: Launch) -> Session {
    // Windows' console resizes keeping the viewport's top.
    let mut config = Config::default();
    (config.default_cursor_style, config.scrollback_limit) = (launch.cursor, launch.scrollback);
    let session = Session::configured(ResizePolicy::KeepViewportTop, config);
    let start = launch.cwd.filter(|dir| dir.is_dir()).or_else(|| std::env::current_dir().ok());
    if let Some(start) = &start {
        session.cwd().set(start.display().to_string());
    }
    // The console starts at the grid's size, once the view has sized it: a program that reads
    // its size only at start reads the real one. ponytail: the closure holds the session until
    // then, so a space never shown keeps its session alive with it.
    let program = launch.shell.map(|shell| (shell.program.display().to_string(), shell.args));
    let mut env: Vec<(String, String)> = vec![
        ("PROMPT".into(), "$e]9;9;$P$e\\$P$G".into()),
        ("TERM_PROGRAM".into(), "1Term".into()),
        ("TERM_PROGRAM_VERSION".into(), env!("CARGO_PKG_VERSION").into()),
    ];
    if launch.force_links {
        // Programs that know terminals by name show OSC 8 links here too.
        env.push(("FORCE_HYPERLINK".into(), "1".into()));
    }
    let starts = session.clone();
    session.when_sized(move |size| {
        let options = PtyOptions { program, cwd: start, env, rows: size.rows, cols: size.cols };
        let (feeder, ended) = (starts.feeder(), starts.feeder());
        let spawned = Pty::spawn(
            options,
            move |bytes| feeder.feed(bytes, Instant::now()),
            move |code| ended.exited(code),
        );
        match spawned {
            Ok(pty) => {
                let pty = Rc::new(pty);
                let (resizes, closes) = (pty.clone(), pty.clone());
                let link = Link::new(move |bytes| pty.write(bytes))
                    .on_resize(move |size| resizes.resize(size.rows, size.cols))
                    .on_close(move || closes.close());
                starts.connect(link);
            }
            Err(e) => {
                let said = format!("1Term: the shell did not start: {e}\r\n");
                starts.feeder().feed(said.as_bytes(), Instant::now());
            }
        }
    });
    session
}
