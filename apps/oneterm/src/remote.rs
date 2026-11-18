//! A space's remote shell: a session over `oneterm-ssh` (specs/19, "M7 in detail"). What the
//! engine asks while it connects is kept with the session as its [`Conn`], which the Connect
//! dialogs show (`asking.rs`); the session is linked to the engine from the start.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use oneterm_ssh::{Ask, Event, Options, Ssh};
use quadra::{Signal, signal};
use quadra_term::vt::{Config, ResizePolicy};
use quadra_term::{Link, Session};

use crate::shells::Launch;

/// Where a connection is, as its dialogs show it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Phase {
    Connecting,
    /// A question of the engine's, failing included.
    Ask(Ask),
    /// The shell is open, or the connection was given up: no dialog.
    Done,
}

/// A remote space's connection while it is made, kept with its session.
pub(crate) struct Conn {
    pub phase: Signal<Phase>,
    /// `user@host`, as the engine last said it was connecting to.
    pub label: Signal<String>,
    ssh: Arc<Ssh>,
}

impl Conn {
    /// Answers the question asked; `None` gives it up, and a failure given up is done.
    pub fn answer(&self, answer: Option<String>) {
        let failed = self.phase.with_untracked(|it| matches!(it, Phase::Ask(Ask::Failed { .. })));
        self.phase.set(if failed && answer.is_none() { Phase::Done } else { Phase::Connecting });
        self.ssh.answer(answer);
    }

    /// Gives up the connecting under way.
    pub fn cancel(&self) {
        self.ssh.cancel();
    }
}

/// A host's saved secret and the question it answers: the target's password, or its key
/// file's passphrase.
struct Saved {
    secret: Option<crate::vault::Secret>,
    target: String,
    key: Option<std::path::PathBuf>,
}

impl Saved {
    fn of(remote: &crate::shells::Remote, secret: Option<crate::vault::Secret>) -> Saved {
        let key = match &remote.auth {
            oneterm_ssh::Auth::Key(path) => Some(path.clone()),
            _ => None,
        };
        Saved { secret, target: remote.target.label(), key }
    }

    /// The secret, each time its question is asked first (a refusal asks `again`).
    fn answer(&self, ask: &Ask) -> Option<String> {
        let ours = match ask {
            Ask::Password { target, again: false } => *target == self.target,
            Ask::Passphrase { file, again: false } => self.key.as_ref() == Some(file),
            _ => false,
        };
        // A copy goes to the engine, which gives it to `russh` to send.
        if ours { self.secret.as_ref().map(|it| it.as_str().to_string()) } else { None }
    }
}

/// A session running a host's shell; called with a launch that names one. The user's own
/// SSH files and agent are used.
pub fn remote_shell(launch: Launch) -> Session {
    remote_shell_with(launch, Options::new)
}

/// As [`remote_shell`], with the engine's options made from the target by `options` (a test
/// gives its own files and no agent).
pub fn remote_shell_with(launch: Launch, options: impl FnOnce(oneterm_ssh::Target) -> Options) -> Session {
    let mut config = Config::default();
    (config.default_cursor_style, config.scrollback_limit) = (launch.cursor, launch.scrollback);
    config.notifications = crate::notify::abilities();
    // A server's terminal reflows its own lines; the viewport's top stays.
    let session = Session::configured(ResizePolicy::KeepViewportTop, config);
    let Some(remote) = launch.ssh else { return session };
    session.title().set(remote.target.label());
    let saved = Saved::of(&remote, launch.secret);
    let startup = launch.startup.map(|it| format!("{it}\r"));
    let size = session.size();
    let (phase, label) = (signal(Phase::Connecting), signal(remote.target.label()));
    let mut options = options(remote.target);
    (options.auth, options.jumps, options.forwards) = (remote.auth, remote.jumps, remote.forwards);
    (options.rows, options.cols) = (size.rows, size.cols);
    if launch.keepalive.1 > 0 {
        (options.keepalive, options.keepalive_max) = launch.keepalive;
    }
    // The engine tells from its threads; the dialogs follow on the UI's.
    let events = quadra::inbox::<Event>();
    let told = events.sender();
    let (feeder, ended) = (Arc::new(Mutex::new(session.feeder())), session.feeder());
    let ssh = Ssh::connect(
        options,
        move |bytes| {
            if let Ok(feeder) = feeder.lock() {
                feeder.feed(bytes, Instant::now());
            }
        },
        move |event| _ = told.push(event),
        move |code| ended.exited(code),
    );
    // Kept with the session, where the SFTP dock and the dialogs find it.
    let ssh = Arc::new(ssh);
    let (answers, starts) = (ssh.clone(), ssh.clone());
    events.on_data(move |burst| {
        for event in burst.drain(..) {
            let _ = match event {
                Event::Connecting(to) => label.try_set(to) && phase.try_set(Phase::Connecting),
                // The saved secret answers its question once; refused, the dialog asks.
                Event::Ask(ask) => match saved.answer(&ask) {
                    Some(secret) => {
                        answers.answer(Some(secret));
                        true
                    }
                    None => phase.try_set(Phase::Ask(ask)),
                },
                Event::Open => {
                    // Each time the shell opens, a Retry's too.
                    if let Some(command) = &startup {
                        starts.write(command.as_bytes());
                    }
                    phase.try_set(Phase::Done)
                }
            };
        }
    });
    session.keep(std::rc::Rc::new(ssh.clone()));
    session.keep(std::rc::Rc::new(Conn { phase, label, ssh: ssh.clone() }));
    // The latency the engine measured, read every 2 s.
    let (pings, note) = (ssh.clone(), session.link_note());
    let _ = quadra::interval(std::time::Duration::from_secs(2), move || {
        if let Some(took) = pings.latency() {
            note.set(format!("{} ms", took.as_millis().max(1)));
        }
    });
    let (resizes, closes) = (ssh.clone(), ssh.clone());
    let link = Link::new(move |bytes| ssh.write(bytes))
        .on_resize(move |size| resizes.resize(size.rows, size.cols))
        .on_close(move || closes.close());
    session.connect(link);
    session
}
