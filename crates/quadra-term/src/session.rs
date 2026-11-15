//! A session (specs/18, "Pieces", "Delivery"): one terminal engine behind a lock, what a
//! transport's reader thread feeds it with (`Feeder`), what the session writes back through
//! (`Link`), and the signals an app reads, kept on the UI thread from one inbox.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use oneterm_vt::{Config, EventBatch, OscRoute, Progress, ResizePolicy, Size, Terminal};
use quadra_async::inbox;
use quadra_reactive::{Signal, signal};
use quadra_tree::{Handle, NodeId};

use crate::feed::{Engine, Event, Feeder, Notice, lock};
use crate::status::{ProgramState, Statuses};

type WriteFn = dyn FnMut(&[u8]);
type NoticeFn = dyn Fn(&Notice);

/// What a transport gives a session: where the keys go, and what a resize and the end do.
pub struct Link {
    write: Box<WriteFn>,
    resize: Box<dyn FnMut(Size)>,
    close: Box<dyn FnMut()>,
}

impl Link {
    pub fn new(write: impl FnMut(&[u8]) + 'static) -> Link {
        Link { write: Box::new(write), resize: Box::new(|_| {}), close: Box::new(|| {}) }
    }

    /// Told the grid's size after the engine took it.
    pub fn on_resize(mut self, f: impl FnMut(Size) + 'static) -> Link {
        self.resize = Box::new(f);
        self
    }

    pub fn on_close(mut self, f: impl FnMut() + 'static) -> Link {
        self.close = Box::new(f);
        self
    }
}

/// The UI thread's handle on one terminal. Made in a view's scope: its signals and the task
/// that takes its output live as long as that scope.
#[derive(Clone)]
pub struct Session(pub(crate) Rc<Shared>);

pub(crate) struct Shared {
    pub engine: Arc<Mutex<Engine>>,
    feeder: Feeder,
    link: RefCell<Link>,
    /// How many links were connected: one connected while the last was called replaces it.
    linked: Cell<u64>,
    policy: ResizePolicy,
    /// The view that paints it, told when output came.
    pub view: Cell<Option<NodeId>>,
    pub handle: RefCell<Option<Handle>>,
    title: Signal<String>,
    cwd: Signal<String>,
    bells: Signal<u32>,
    progress: Signal<Option<Progress>>,
    status: Signal<ProgramState>,
    exited: Signal<Option<i32>>,
    notices: RefCell<Vec<Rc<NoticeFn>>>,
}

impl Session {
    /// A session of 80 by 24 cells until its view sizes it; `policy` is how the engine
    /// resizes (a pseudo console of Windows wants `KeepViewportTop`).
    pub fn new(policy: ResizePolicy) -> Session {
        Session::configured(policy, Config::default())
    }

    /// As [`Session::new`], with the engine's configuration: the cursor until a program sets
    /// one, the history kept. OSC 7501 is routed to the session whatever it says.
    pub fn configured(policy: ResizePolicy, mut config: Config) -> Session {
        config.osc_routes.route(7501, OscRoute::Forward);
        let term = Terminal::new(Size { rows: 24, cols: 80 }, config);
        let engine = Engine { term, batch: EventBatch::new(), statuses: Statuses::default() };
        let engine = Arc::new(Mutex::new(engine));
        let events = inbox::<Event>();
        let feeder = Feeder {
            engine: engine.clone(),
            events: events.sender(),
            pending: Arc::new(AtomicBool::new(false)),
        };
        let shared = Rc::new(Shared {
            engine,
            feeder,
            link: RefCell::new(Link::new(|_| {})),
            linked: Cell::new(0),
            policy,
            view: Cell::new(None),
            handle: RefCell::new(None),
            title: signal(String::new()),
            cwd: signal(String::new()),
            bells: signal(0),
            progress: signal(None),
            status: signal(ProgramState::Idle),
            exited: signal(None),
            notices: RefCell::new(Vec::new()),
        });
        let (took, ended) = (Rc::downgrade(&shared), Rc::downgrade(&shared));
        events
            .on_finish(move |code| {
                if let Some(s) = ended.upgrade() {
                    s.exited.set(Some(code));
                }
            })
            .on_data(move |burst| {
                if let Some(s) = took.upgrade() {
                    s.take(burst);
                }
            });
        Session(shared)
    }

    /// What a transport feeds the output with, on any thread.
    pub fn feeder(&self) -> Feeder {
        self.0.feeder.clone()
    }

    /// Where the session writes keys and replies, and tells a resize and the end.
    /// The link is told the grid's size at once: a transport started before layout sized
    /// the grid hears it.
    pub fn connect(&self, link: Link) {
        let size = self.size();
        self.0.linked.set(self.0.linked.get() + 1);
        *self.0.link.borrow_mut() = link;
        self.with_link(|link| (link.resize)(size));
    }

    /// Calls the link without holding it: what it calls may write, close or connect again.
    /// The link is put back unless another was connected meanwhile.
    fn with_link(&self, f: impl FnOnce(&mut Link)) {
        let linked = self.0.linked.get();
        let mut link = std::mem::replace(&mut *self.0.link.borrow_mut(), Link::new(|_| {}));
        f(&mut link);
        if self.0.linked.get() == linked {
            *self.0.link.borrow_mut() = link;
        }
    }

    /// Writes bytes to the program.
    pub fn write(&self, bytes: &[u8]) {
        if !bytes.is_empty() {
            self.with_link(|link| (link.write)(bytes));
        }
    }

    pub fn close(&self) {
        self.with_link(|link| (link.close)());
    }

    /// Sets the grid's size: the engine's, then the link's. Nothing when it is that already.
    pub fn resize(&self, rows: u16, cols: u16) {
        let size = Size { rows: rows.max(1), cols: cols.max(1) };
        {
            let mut engine = lock(&self.0.engine);
            if engine.term.size() == size {
                return;
            }
            engine.term.resize(size, self.0.policy);
        }
        self.with_link(|link| (link.resize)(size));
    }

    /// How many rows of history the engine keeps from now on.
    pub fn set_scrollback(&self, rows: u32) {
        self.engine().term.set_scrollback_limit(rows);
    }

    /// The grid's size.
    pub fn size(&self) -> Size {
        lock(&self.0.engine).term.size()
    }

    /// The text of the rows that show, each with its trailing blanks cut.
    pub fn screen_text(&self) -> Vec<String> {
        let engine = lock(&self.0.engine);
        let top = engine.term.viewport().top;
        let rows = engine.term.size().rows as u64;
        (0..rows).map(|i| engine.term.row_text(oneterm_vt::RowId(top.0 + i)).trim_end().to_string()).collect()
    }

    /// The program's title (OSC 0 and 2); empty until it sets one.
    pub fn title(&self) -> Signal<String> {
        self.0.title
    }

    /// The working directory the shell says (OSC 7).
    pub fn cwd(&self) -> Signal<String> {
        self.0.cwd
    }

    /// How many bells rang; the app sets it back to 0 when they were seen.
    pub fn bells(&self) -> Signal<u32> {
        self.0.bells
    }

    /// OSC 9;4: none, or the program's progress.
    pub fn progress(&self) -> Signal<Option<Progress>> {
        self.0.progress
    }

    /// OSC 7501: the strongest state the program reported.
    pub fn status(&self) -> Signal<ProgramState> {
        self.0.status
    }

    /// The program's exit code, once it ended.
    pub fn exited(&self) -> Signal<Option<i32>> {
        self.0.exited
    }

    /// Runs `f` with each OSC 9 notification.
    pub fn on_notification(&self, f: impl Fn(&Notice) + 'static) {
        self.0.notices.borrow_mut().push(Rc::new(f));
    }

    pub(crate) fn engine(&self) -> MutexGuard<'_, Engine> {
        lock(&self.0.engine)
    }
}

impl Shared {
    /// A burst from the inbox: the signals follow it, and the view is painted again.
    fn take(&self, burst: &mut Vec<Event>) {
        for event in burst.drain(..) {
            match event {
                Event::Output => {
                    self.feeder.pending.store(false, Ordering::Release);
                    if let (Some(id), Some(handle)) = (self.view.get(), self.handle.borrow().clone()) {
                        handle.with(move |tree| {
                            crate::widget::repaint(tree, id);
                            tree.update_semantics(id, |_| {}); // a reader is told the rows
                        });
                    }
                }
                Event::Title(title) => self.title.set(title),
                Event::Cwd(cwd) => self.cwd.set(cwd),
                Event::Bell => self.bells.update(|n| *n += 1),
                Event::Notice(notice) => {
                    let notices = self.notices.borrow().clone();
                    for f in notices {
                        f(&notice);
                    }
                }
                Event::Progress(Progress::Remove) => self.progress.set(None),
                Event::Progress(p) => self.progress.set(Some(p)),
                Event::Status(state) => self.status.set(state),
            }
        }
    }
}

/// A session whose link writes what is typed back as its output (Enter as a new line): for
/// tests and the gallery.
pub fn echo() -> Session {
    let session = Session::new(ResizePolicy::BottomAnchor);
    let feeder = session.feeder();
    session.connect(Link::new(move |bytes: &[u8]| {
        let mut shown = Vec::with_capacity(bytes.len());
        for b in bytes {
            shown.extend_from_slice(if *b == b'\r' { b"\r\n" } else { std::slice::from_ref(b) });
        }
        feeder.feed(&shown, Instant::now());
    }));
    session
}
