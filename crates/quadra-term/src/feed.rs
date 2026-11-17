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
    /// anything else it was given, in order.
    pub fn feed(&self, bytes: &[u8], now: Instant) -> Vec<u8> {
        crate::log::Log::write(&self.log, bytes);
        let (mut replies, mut events, mut repaint, mut cleared) = (Vec::new(), Vec::new(), false, false);
        {
            let mut engine = lock(&self.engine);
            let Engine { term, batch, statuses, stamps, marked, status, images } = &mut *engine;
            term.feed(bytes, batch, now);
            for image in term.take_graphics() {
                images.arrive(image);
                repaint = true;
            }
            for event in batch.iter() {
                match *event {
                    VtEvent::Repaint => repaint = true,
                    VtEvent::GraphicReleased(id) => images.release(id),
                    VtEvent::ScreenCleared => cleared = true,
                    VtEvent::Reply(span) => replies.extend_from_slice(batch.bytes(span)),
                    VtEvent::ColorQuery { key, terminator } => {
                        replies.extend(term.color_reply(key, terminator).unwrap_or_default())
                    }
                    VtEvent::Title(text) => events.push(Event::Title(batch.str(text).to_string())),
                    VtEvent::TitleReset => events.push(Event::Title(String::new())),
                    VtEvent::Bell => events.push(Event::Bell),
                    VtEvent::Cwd { path, .. } => events.push(Event::Cwd(batch.str(path).to_string())),
                    VtEvent::Notification { title, body } => {
                        let body = batch.str(body);
                        // OSC 9;9 is a Windows shell's working directory, not a notice.
                        events.push(match body.strip_prefix("9;") {
                            Some(path) => Event::Cwd(path.trim_matches('"').to_string()),
                            None => Event::Notice(Notice {
                                title: batch.str(title).to_string(),
                                body: body.to_string(),
                            }),
                        })
                    }
                    VtEvent::Progress(p) => events.push(Event::Progress(p)),
                    VtEvent::ShellMark(ShellMark::PromptEnd) => *marked = true,
                    VtEvent::ShellMark(ShellMark::OutputEnd { exit_code }) => *status = exit_code,
                    VtEvent::ShellMark(ShellMark::PromptStart) if statuses.prompt() => {
                        events.push(Event::Status(statuses.summary(), statuses.named()))
                    }
                    VtEvent::Osc { code: 7501, params, terminator, truncated: false } => {
                        let body = batch.params(params).skip(1).collect::<Vec<_>>().join(&b';');
                        match statuses.report(&body) {
                            Report::Query => {
                                replies.extend_from_slice(b"\x1b]7501;?");
                                replies.extend_from_slice(match terminator {
                                    StringTerm::Bel => b"\x07",
                                    StringTerm::St => b"\x1b\\",
                                });
                            }
                            Report::Changed(state) => events.push(Event::Status(state, statuses.named())),
                            Report::Dropped => {}
                        }
                    }
                    VtEvent::ClipboardStore { selection: ClipboardKind::Clipboard, text } => {
                        events.push(Event::ClipStore(batch.str(text).to_string()))
                    }
                    VtEvent::ClipboardLoad { selection: ClipboardKind::Clipboard } => {
                        events.push(Event::ClipLoad)
                    }
                    _ => {}
                }
            }
            stamps.update(term, cleared, Some(now));
        }
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
