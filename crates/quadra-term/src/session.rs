//! A session (specs/18, "Pieces", "Delivery"): one terminal engine behind a lock, what a
//! transport's reader thread feeds it with (`Feeder`), what the session writes back through
//! (`Link`), and the signals an app reads, kept on the UI thread from one inbox.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use oneterm_vt::{Config, EventBatch, OscRoute, Progress, ResizePolicy, Size, Terminal};
use quadra_async::inbox;
use quadra_reactive::{Signal, signal};
use quadra_tree::{Handle, NodeId};

use crate::feed::{Engine, Event, Feeder, Notice, lock};
use crate::status::{ProgramState, Statuses};

mod signals;

type WriteFn = dyn FnMut(&[u8]);
type NoticeFn = dyn Fn(&Notice);
type Waiting = Vec<Box<dyn FnOnce(Size)>>;

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
    pub feeder: Feeder,
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
    status_name: Signal<String>,
    exited: Signal<Option<i32>>,
    link_note: Signal<String>,
    notices: RefCell<Vec<Rc<NoticeFn>>>,
    /// What the app keeps with the session.
    kept: RefCell<Vec<Rc<dyn std::any::Any>>>,
    /// The view's clipboard and settings, for OSC 52; the link under the other button's press.
    pub clipboard: RefCell<Option<Rc<dyn quadra_platform::Clipboard>>>,
    pub settings: Cell<Option<Signal<crate::TermSettings>>>,
    pub pressed: RefCell<Option<crate::TermLink>>,
    pub line: crate::line::Follow,
    /// What waits for a view to size the grid; `None` once one has.
    pub sized: RefCell<Option<Waiting>>,
    pub find: crate::find::Finding,
    /// Told what the user sends (`Session::on_input`).
    pub input: RefCell<Option<Rc<crate::input::InputFn>>>,
    /// The images it shows, taken from the engine when its view refreshes.
    pub images: RefCell<crate::images::Images>,
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
        let engine = Engine {
            term,
            batch: EventBatch::new(),
            statuses: Statuses::default(),
            stamps: Default::default(),
            marked: false,
            status: None,
            images: Default::default(),
            frame: oneterm_vt::SnapshotState::new(),
            framed: false,
            alternate: false,
        };
        let engine = Arc::new(Mutex::new(engine));
        let events = inbox::<Event>();
        let feeder = Feeder {
            engine: engine.clone(),
            events: events.sender(),
            pending: Arc::new(AtomicBool::new(false)),
            log: Arc::default(),
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
            status_name: signal(String::new()),
            exited: signal(None),
            link_note: signal(String::new()),
            notices: RefCell::new(Vec::new()),
            kept: RefCell::new(Vec::new()),
            clipboard: RefCell::new(None),
            settings: Cell::new(None),
            pressed: RefCell::new(None),
            line: crate::line::Follow::new(),
            sized: RefCell::new(Some(Vec::new())),
            find: crate::find::Finding::new(),
            input: RefCell::new(None),
            images: RefCell::default(),
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
                    Session(s).take(burst);
                }
            });
        Session(shared)
    }

    /// What a transport feeds the output with, on any thread.
    pub fn feeder(&self) -> Feeder {
        self.0.feeder.clone()
    }

    /// Where the session writes keys and replies, and tells a resize and the end. The link is
    /// told the grid's size at once: a transport started before layout sized the grid hears it.
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
        self.refind(); // a reflow gives the rows new ids
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

    /// Keeps a value of the app's with the session, for as long as the session lives: a
    /// remote space's connection, for one.
    pub fn keep(&self, value: Rc<dyn std::any::Any>) {
        self.0.kept.borrow_mut().push(value);
    }

    /// The value of type `T` kept with the session, if any.
    pub fn kept<T: 'static>(&self) -> Option<Rc<T>> {
        self.0.kept.borrow().iter().find_map(|it| it.clone().downcast::<T>().ok())
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

impl Session {
    /// A burst from the inbox: the signals follow it, and the view is painted again.
    fn take(&self, burst: &mut Vec<Event>) {
        for event in burst.drain(..) {
            match event {
                Event::Output => {
                    self.0.feeder.pending.store(false, Ordering::Release);
                    self.refind_later();
                    if let (Some(id), Some(handle)) = (self.0.view.get(), self.0.handle.borrow().clone()) {
                        handle.with(move |tree| {
                            crate::widget::repaint(tree, id);
                            tree.update_semantics(id, |_| {}); // a reader is told the rows
                        });
                    }
                }
                Event::Title(title) => self.0.title.set(title),
                Event::Cwd(cwd) => self.0.cwd.set(cwd),
                Event::Bell => self.0.bells.update(|n| *n += 1),
                Event::Notice(notice) => {
                    let notices = self.0.notices.borrow().clone();
                    for f in notices {
                        f(&notice);
                    }
                }
                Event::Progress(Progress::Remove) => self.0.progress.set(None),
                Event::Progress(p) => self.0.progress.set(Some(p)),
                Event::Status(state, name) => {
                    self.0.status_name.set(name);
                    self.0.status.set(state);
                }
                Event::ClipStore(text) => self.clip_store(&text),
                Event::ClipLoad => self.clip_load(),
                Event::Pointer(name) => {
                    if let (Some(id), Some(handle)) = (self.0.view.get(), self.0.handle.borrow().clone()) {
                        handle.with(move |tree| crate::link::shape(tree, id, &name));
                    }
                }
            }
        }
    }
}
