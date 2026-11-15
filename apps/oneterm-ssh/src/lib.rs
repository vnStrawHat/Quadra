//! 1Term's SSH (specs/19, "SSH (P2)"): `russh` on a `tokio` runtime of its own, behind a
//! blocking API of the shape of `quadra_shell::Pty`. What the engine asks (a host key, a
//! password) is asked in the terminal, as the OpenSSH client does.

mod auth;
mod files;
mod prompt;
mod session;
mod target;
#[cfg(feature = "testing")]
pub mod testing;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

pub use files::{Entry, Files, join, safe_name};
use prompt::Cmd;
pub use target::Target;

/// How to prove who the user is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Auth {
    /// The agent, the usual key files, then the password.
    #[default]
    Auto,
    Agent,
    Key(PathBuf),
    Password,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub target: Target,
    pub auth: Auth,
    /// The host keys known, OpenSSH's file.
    pub known_hosts: PathBuf,
    /// Where the usual key files are (`~/.ssh`).
    pub keys: PathBuf,
    /// The agent's pipe.
    pub agent: String,
    pub rows: u16,
    pub cols: u16,
}

impl Options {
    /// The user's own files (`%USERPROFILE%\.ssh`) and agent, 24 by 80.
    pub fn new(target: Target) -> Options {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        let ssh = home.join(".ssh");
        Options {
            target,
            auth: Auth::Auto,
            known_hosts: ssh.join("known_hosts"),
            keys: ssh,
            agent: r"\\.\pipe\openssh-ssh-agent".into(),
            rows: 24,
            cols: 80,
        }
    }
}

/// What [`Ssh::files`] says before the shell is open.
pub const NOT_YET: &str = "Not connected yet";

/// A connection running a shell. Dropping it closes it.
pub struct Ssh {
    to: UnboundedSender<Cmd>,
    shared: Arc<Shared>,
}

/// What the connection's task and the handle share.
#[derive(Default)]
pub(crate) struct Shared {
    /// The last ping's time in microseconds; 0 before the first.
    latency: AtomicU32,
    /// The connection, once the shell is open.
    handle: Mutex<Option<Arc<russh::client::Handle<session::Checker>>>>,
    files: Mutex<Option<Files>>,
}

impl Ssh {
    /// Starts connecting at once and returns: what the server (or the engine) says comes to
    /// `output`, from the engine's threads; `exited` is told the shell's exit status, 255
    /// when the connection failed or dropped.
    pub fn connect(
        options: Options,
        output: impl Fn(&[u8]) + Send + Sync + 'static,
        exited: impl FnOnce(i32) + Send + 'static,
    ) -> Ssh {
        let (to, from) = unbounded_channel();
        let shared = Arc::new(Shared::default());
        runtime().spawn(session::run(options, from, Arc::new(output), Box::new(exited), shared.clone()));
        Ssh { to, shared }
    }

    /// How long the last ping took, once the shell is open (every 5 s).
    pub fn latency(&self) -> Option<Duration> {
        let micros = self.shared.latency.load(Ordering::Relaxed);
        (micros > 0).then(|| Duration::from_micros(micros.into()))
    }

    /// Bytes typed: an answer while the engine asks, else the shell's input.
    pub fn write(&self, bytes: &[u8]) {
        let _ = self.to.send(Cmd::Input(bytes.to_vec()));
    }

    /// The connection's files over SFTP, opened the first time; an error before the shell
    /// is open. It blocks: an app calls it on its pool.
    pub fn files(&self) -> Result<Files, String> {
        let mut files = self.shared.files.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(files) = &*files {
            return Ok(files.clone());
        }
        let handle = self.shared.handle.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let opened = Files::open(handle.ok_or(NOT_YET)?)?;
        *files = Some(opened.clone());
        Ok(opened)
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        let _ = self.to.send(Cmd::Resize(rows, cols));
    }

    pub fn close(&self) {
        let _ = self.to.send(Cmd::Close);
    }
}

impl Drop for Ssh {
    fn drop(&mut self) {
        self.close();
    }
}

/// The engine's runtime: two threads, made with the first connection.
pub(crate) fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ssh")
            .enable_all()
            .build()
            .expect("the SSH runtime starts")
    })
}
