//! Feeding a session (specs/18, "Delivery"): the reader thread of a transport feeds the
//! engine under the lock, writes back the replies it is given, and what the UI thread must hear
//! goes through one inbox.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use oneterm_vt::{ClipboardKind, EventBatch, Progress, ShellMark, StringTerm, Terminal, VtEvent};
use quadra_async::Sender;

use crate::status::{ProgramState, Report, Statuses};

/// The engine and what feeding it needs, behind the session's lock.
pub(crate) struct Engine {
    pub term: Terminal,
    pub batch: EventBatch,
    pub statuses: Statuses,
    /// When the rows were written, for the gutter.
    pub stamps: crate::gutter::Stamps,
    /// The shell has marked where its input starts (OSC 133 `B`).
    pub marked: bool,
    /// The status of the command that ended last (OSC 133 `D`).
    pub status: Option<i32>,
    /// The images decoded and let go of since the UI thread last took them.
    pub images: crate::images::Arrivals,
    /// The last frame a program closed, copied when it did; `framed` while the UI thread has
    /// not taken it.
    pub frame: oneterm_vt::SnapshotState,
    pub framed: bool,
}

/// What the UI thread must hear from a feed.
pub(crate) enum Event {
    Output,
    Title(String),
    Cwd(String),
    Bell,
    Notice(Notice),
    Progress(Progress),
    /// The summary of the records, and the id of the one it is.
    Status(ProgramState, String),
    ClipStore(String),
    ClipLoad,
    /// OSC 22: the pointer shape the program names.
    Pointer(String),
}

/// An OSC 9 notification (or OSC 777's): what a program wants the user told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
}

/// What the reader thread of a transport calls with the program's output. It is `Send`.
#[derive(Clone)]
pub struct Feeder {
    pub(crate) engine: Arc<Mutex<Engine>>,
    pub(crate) events: Sender<Event>,
    /// An `Output` is on its way to the UI thread: a burst of feeds posts one.
    pub(crate) pending: Arc<AtomicBool>,
    /// The session's log, while one is open.
    pub(crate) log: crate::log::Slot,
}

pub(crate) fn lock(engine: &Mutex<Engine>) -> MutexGuard<'_, Engine> {
    engine.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Feeder {
    /// Feeds output to the engine; returns the replies the transport must write back, before
    /// anything else it was given, in order. A read is fed in parts that end where a
    /// synchronized update closes (`feed_frame`), and the frame it closed is copied then.
    pub fn feed(&self, bytes: &[u8], now: Instant) -> Vec<u8> {
        crate::log::Log::write(&self.log, bytes);
        let mut heard = Heard::default();
        {
            let mut engine = lock(&self.engine);
            let mut rest = bytes;
            while !rest.is_empty() {
                let (fed, closed) = apply(&mut engine, rest, now, &mut heard);
                if closed {
                    let Engine { term, frame, framed, .. } = &mut *engine;
                    term.snapshot_update(frame, now);
                    *framed = true;
                }
                rest = &rest[fed..];
            }
            let Engine { term, stamps, .. } = &mut *engine;
            stamps.update(term, heard.cleared, Some(now));
        }
        let Heard { replies, mut events, repaint, .. } = heard;
        if repaint && !self.pending.swap(true, Ordering::AcqRel) {
            events.push(Event::Output);
        }
        for event in events {
            self.events.push(event);
        }
        replies
    }

    /// The program ended with `code`: told after the output that came before.
    pub fn exited(&self, code: i32) {
        self.events.finish(code);
    }
}

/// What feeding heard: the replies, what the UI thread must hear, and whether to paint.
#[derive(Default)]
struct Heard {
    replies: Vec<u8>,
    events: Vec<Event>,
    repaint: bool,
    cleared: bool,
}

/// Feeds a read up to a closed synchronized update and takes what it made: how many bytes it
/// fed, and whether it closed one.
fn apply(engine: &mut Engine, bytes: &[u8], now: Instant, heard: &mut Heard) -> (usize, bool) {
    let stats = engine.term.feed_frame(bytes, &mut engine.batch, now);
    for image in engine.term.take_graphics() {
        engine.images.arrive(image);
        heard.repaint = true;
    }
    for event in engine.batch.iter() {
        match *event {
            VtEvent::Repaint => heard.repaint = true,
            VtEvent::GraphicReleased(id) => engine.images.release(id),
            VtEvent::ScreenCleared => heard.cleared = true,
            VtEvent::Reply(span) => heard.replies.extend_from_slice(engine.batch.bytes(span)),
            VtEvent::ColorQuery { key, terminator } => {
                heard.replies.extend(engine.term.color_reply(key, terminator).unwrap_or_default())
            }
            VtEvent::Title(text) => heard.events.push(Event::Title(engine.batch.str(text).to_string())),
            VtEvent::TitleReset => heard.events.push(Event::Title(String::new())),
            VtEvent::Pointer(name) => heard.events.push(Event::Pointer(engine.batch.str(name).to_string())),
            VtEvent::Bell => heard.events.push(Event::Bell),
            VtEvent::Cwd { path, .. } => heard.events.push(Event::Cwd(engine.batch.str(path).to_string())),
            VtEvent::Notification { title, body } => {
                let body = engine.batch.str(body);
                // OSC 9;9 is a Windows shell's working directory, not a notice.
                heard.events.push(match body.strip_prefix("9;") {
                    Some(path) => Event::Cwd(path.trim_matches('"').to_string()),
                    None => Event::Notice(Notice {
                        title: engine.batch.str(title).to_string(),
                        body: body.to_string(),
                    }),
                })
            }
            VtEvent::Progress(p) => heard.events.push(Event::Progress(p)),
            VtEvent::ShellMark(ShellMark::PromptEnd) => engine.marked = true,
            VtEvent::ShellMark(ShellMark::OutputEnd { exit_code }) => engine.status = exit_code,
            VtEvent::ShellMark(ShellMark::PromptStart) if engine.statuses.prompt() => {
                heard.events.push(Event::Status(engine.statuses.summary(), engine.statuses.named()))
            }
            VtEvent::Osc { code: 7501, params, terminator, truncated: false } => {
                let body = engine.batch.params(params).skip(1).collect::<Vec<_>>().join(&b';');
                match engine.statuses.report(&body) {
                    Report::Query => {
                        heard.replies.extend_from_slice(b"\x1b]7501;?");
                        heard.replies.extend_from_slice(match terminator {
                            StringTerm::Bel => b"\x07",
                            StringTerm::St => b"\x1b\\",
                        });
                    }
                    Report::Changed(state) => {
                        heard.events.push(Event::Status(state, engine.statuses.named()))
                    }
                    Report::Dropped => {}
                }
            }
            VtEvent::ClipboardStore { selection: ClipboardKind::Clipboard, text } => {
                heard.events.push(Event::ClipStore(engine.batch.str(text).to_string()))
            }
            VtEvent::ClipboardLoad { selection: ClipboardKind::Clipboard } => {
                heard.events.push(Event::ClipLoad)
            }
            _ => {}
        }
    }
    (stats.bytes.max(1).min(bytes.len()), stats.frames_closed > 0)
}
