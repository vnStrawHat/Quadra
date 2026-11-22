//! A local shell behind the OS's pseudo console (specs/18, "Pieces"; the `pty` feature): the
//! console of `oneterm_vt::pty`, on its poller in safe code (`Registered`), and the thread that
//! owns it, polls it, hands its output to the app and
//! writes what the app gives it. The app joins it to a terminal session: the output callback
//! is the session's feeder, and the session's link calls [`Pty::write`], `resize`, `close`.

use std::io::{self, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use oneterm_vt::pty::{
    ChildEvent, Options, PTY_CHILD_EVENT_TOKEN, PTY_READ_WRITE_TOKEN, PseudoConsole, Registered, Shell,
    WindowSize,
};
use polling::{Event, Events, PollMode, Poller};

/// What a shell is started with.
#[derive(Clone, Debug, Default)]
pub struct PtyOptions {
    /// The program and its arguments; `None` is the OS's shell (`cmd.exe` on Windows).
    pub program: Option<(String, Vec<String>)>,
    pub cwd: Option<PathBuf>,
    /// Added to the app's environment; `TERM` and `COLORTERM` are set unless given here.
    pub env: Vec<(String, String)>,
    pub rows: u16,
    pub cols: u16,
}

enum Command {
    Write(Vec<u8>),
    Resize(u16, u16),
    Close,
}

/// A running shell. Dropping it closes the console.
pub struct Pty {
    commands: Sender<Command>,
    poller: Arc<Poller>,
    /// The shell's process, when the OS said it.
    pid: Option<u32>,
}

impl Pty {
    /// Starts the shell and its thread. `output` is called on that thread with each read and
    /// returns the bytes to write back first (a terminal's replies); `exited` with the exit
    /// code once the output is all read (-1 when it cannot be known).
    pub fn spawn(
        options: PtyOptions,
        output: impl FnMut(&[u8]) -> Vec<u8> + Send + 'static,
        exited: impl FnOnce(i32) + Send + 'static,
    ) -> io::Result<Pty> {
        let mut env: std::collections::HashMap<String, String> = options.env.into_iter().collect();
        env.entry("TERM".into()).or_insert_with(|| "xterm-256color".into());
        env.entry("COLORTERM".into()).or_insert_with(|| "truecolor".into());
        let spawn = Options {
            shell: options.program.map(|(program, args)| Shell::new(program, args)),
            working_directory: options.cwd,
            env,
            ..Options::default()
        };
        let size = WindowSize {
            rows: options.rows.max(1),
            cols: options.cols.max(1),
            cell_width: 0,
            cell_height: 0,
        };
        let console = PseudoConsole::spawn(&spawn, size)?;
        let pid = console.child_pid();
        let poller = Arc::new(Poller::new()?);
        let readable = Event::readable(PTY_READ_WRITE_TOKEN);
        let mut console = Registered::new(console, poller.clone(), readable, PollMode::Level)?;
        let (commands, inbox) = channel();
        thread::Builder::new().name("pty".into()).spawn(move || {
            let code = run(&mut console, &inbox, output).unwrap_or(-1);
            drop(console); // off the poller, then closed
            if code != CLOSED {
                exited(code);
            }
        })?;
        Ok(Pty { commands, poller, pid })
    }

    /// The shell's process id, for what reads what it runs.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn write(&self, bytes: &[u8]) {
        self.send(Command::Write(bytes.to_vec()));
    }

    pub fn resize(&self, rows: u16, cols: u16) {
        self.send(Command::Resize(rows, cols));
    }

    /// Closes the console: the shell is ended, and `exited` is not called.
    pub fn close(&self) {
        self.send(Command::Close);
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_ok() {
            let _ = self.poller.notify();
        }
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.close();
    }
}

/// What `run` gives back when the app closed the console.
const CLOSED: i32 = i32::MIN;

/// The console's loop: commands, output, writes kept until there is room, the exit.
fn run(
    console: &mut Registered,
    commands: &Receiver<Command>,
    mut output: impl FnMut(&[u8]) -> Vec<u8>,
) -> io::Result<i32> {
    let (mut events, mut pending, mut ended, mut writable) = (Events::new(), Vec::<u8>::new(), false, false);
    loop {
        events.clear();
        console.poller().wait(&mut events, None)?;
        for command in commands.try_iter() {
            match command {
                Command::Write(bytes) => pending.extend_from_slice(&bytes),
                Command::Resize(rows, cols) => {
                    let _ = console.on_resize(WindowSize { rows, cols, cell_width: 0, cell_height: 0 });
                }
                Command::Close => return Ok(CLOSED),
            }
        }
        let mut exit = None;
        for event in events.iter() {
            match event.key {
                PTY_READ_WRITE_TOKEN if event.readable && !ended => {
                    ended = drain(console, &mut output, &mut pending)?
                }
                PTY_CHILD_EVENT_TOKEN => {
                    if let Some(ChildEvent::Exited(status)) = console.next_child_event() {
                        exit = Some(status.and_then(|s| s.code()).unwrap_or(-1));
                    }
                }
                _ => {}
            }
        }
        if let Some(code) = exit {
            if !ended {
                drain(console, &mut output, &mut pending)?; // what is still buffered
            }
            return Ok(code);
        }
        // What the console takes now; the rest when it says there is room.
        while !pending.is_empty() {
            match console.write(&pending) {
                Ok(0) => break,
                Ok(n) => _ = pending.drain(..n),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        if writable == pending.is_empty() {
            writable = !pending.is_empty();
            let interest = if writable {
                Event::all(PTY_READ_WRITE_TOKEN)
            } else {
                Event::readable(PTY_READ_WRITE_TOKEN)
            };
            console.reregister(interest, PollMode::Level)?;
        }
    }
}

/// Reads until the console has no more for now; true when its output ended.
fn drain(
    console: &mut Registered,
    output: &mut impl FnMut(&[u8]) -> Vec<u8>,
    pending: &mut Vec<u8>,
) -> io::Result<bool> {
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = match console.read(&mut buf) {
            Ok(0) => return Ok(true),
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(false),
            Err(e) if e.raw_os_error() == Some(5) => return Ok(true), // EIO: Linux's end
            Err(e) => return Err(e),
        };
        let replies = output(&buf[..n]);
        // After what is already queued: a key half written must not be cut by a reply.
        pending.extend_from_slice(&replies);
    }
}
