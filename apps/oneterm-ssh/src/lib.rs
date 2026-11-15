//! 1Term's SSH (specs/19, "SSH (P2)"): `russh` on a `tokio` runtime of its own, behind a
//! blocking API of the shape of `quadra_shell::Pty`. What the engine asks (a host key, a
//! password) is asked in the terminal, as the OpenSSH client does.

mod auth;
mod prompt;
mod session;
mod target;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

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

/// A connection running a shell. Dropping it closes it.
pub struct Ssh {
    to: UnboundedSender<Cmd>,
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
        runtime().spawn(session::run(options, from, Arc::new(output), Box::new(exited)));
        Ssh { to }
    }

    /// Bytes typed: an answer while the engine asks, else the shell's input.
    pub fn write(&self, bytes: &[u8]) {
        let _ = self.to.send(Cmd::Input(bytes.to_vec()));
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
fn runtime() -> &'static Runtime {
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
