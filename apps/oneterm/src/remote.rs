//! A space's remote shell: a session over `oneterm-ssh` (specs/19, "M7 in detail"). What the
//! engine asks is asked in the space, so the session is linked to the engine from the start.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use oneterm_ssh::{Options, Ssh};
use quadra_term::vt::{Config, ResizePolicy};
use quadra_term::{Link, Session};

use crate::shells::Launch;

/// A session running a host's shell; called with a launch that names one.
pub fn remote_shell(launch: Launch) -> Session {
    let mut config = Config::default();
    (config.default_cursor_style, config.scrollback_limit) = (launch.cursor, launch.scrollback);
    // A server's terminal reflows its own lines; the viewport's top stays.
    let session = Session::configured(ResizePolicy::KeepViewportTop, config);
    let Some(remote) = launch.ssh else { return session };
    session.title().set(remote.target.label());
    let size = session.size();
    let mut options = Options::new(remote.target);
    options.auth = remote.auth;
    (options.rows, options.cols) = (size.rows, size.cols);
    // The engine calls from its threads.
    let (feeder, ended) = (Arc::new(Mutex::new(session.feeder())), session.feeder());
    let ssh = Ssh::connect(
        options,
        move |bytes| {
            if let Ok(feeder) = feeder.lock() {
                feeder.feed(bytes, Instant::now());
            }
        },
        move |code| ended.exited(code),
    );
    let ssh = std::rc::Rc::new(ssh);
    let (resizes, closes) = (ssh.clone(), ssh.clone());
    let link = Link::new(move |bytes| ssh.write(bytes))
        .on_resize(move |size| resizes.resize(size.rows, size.cols))
        .on_close(move || closes.close());
    session.connect(link);
    session
}
