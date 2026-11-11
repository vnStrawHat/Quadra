//! The thread's runtime: arenas of nodes, scopes and events, and the state of a flush.
//!
//! Rule of this crate: no user code runs inside `rt`. A closure, a `Clone`, a `PartialEq` or a
//! `Drop` of a user's type may use the runtime, which would borrow it twice. So values and
//! closures are cloned out (they are `Rc`s) and called outside, and what is removed is returned
//! and dropped outside.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::VecDeque;

use std::panic::Location;
use std::rc::Rc;

use crate::arena::{Arena, Id};
use crate::error::Hook;

pub(crate) type Loc = &'static Location<'static>;
pub(crate) type Value = Rc<RefCell<dyn Any>>;
/// Runs a memo or an effect; true when a memo's value changed.
pub(crate) type Compute = Rc<RefCell<dyn FnMut() -> bool>>;
pub(crate) type Handler = Rc<RefCell<dyn FnMut(&dyn Any)>>;
pub(crate) type Notify = Rc<dyn Fn()>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum State {
    Clean,
    /// A source further up may have changed: ask the sources before running.
    Check,
    Dirty,
}

pub(crate) enum Kind {
    Signal,
    Memo,
    Effect,
    /// An effect its owner runs later; it is told once when it has become due.
    Deferred(Notify),
}

pub(crate) struct Node {
    pub kind: Kind,
    pub state: State,
    pub value: Value,
    pub compute: Option<Compute>,
    /// What the last run read, in order.
    pub sources: Vec<Id>,
    pub subscribers: Vec<Id>,
    pub seq: u64,
    /// The flush `runs` counts in.
    pub flush: u64,
    pub runs: u32,
    pub created: Loc,
    pub woken_by: Option<Loc>,
}

impl Node {
    pub fn new(kind: Kind, state: State, value: Value, compute: Option<Compute>, created: Loc) -> Node {
        Node {
            kind,
            state,
            value,
            compute,
            sources: Vec::new(),
            subscribers: Vec::new(),
            seq: 0,
            flush: 0,
            runs: 0,
            created,
            woken_by: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct ScopeData {
    pub parent: Option<Id>,
    pub children: Vec<Id>,
    pub nodes: Vec<Id>,
    pub events: Vec<Id>,
    pub cleanups: Vec<Box<dyn FnOnce()>>,
    pub contexts: Vec<(TypeId, Rc<dyn Any>)>,
}

pub(crate) struct EventData {
    pub subs: Vec<(u64, Handler)>,
}

/// What a disposed scope owned, to be dropped outside the runtime borrow.
pub(crate) type Garbage = (ScopeData, Vec<Node>, Vec<EventData>);

/// The reads of the node that is running, compared with its last run (graph.rs).
#[derive(Default)]
pub(crate) struct Tracking {
    pub index: usize,
    pub new: Vec<Id>,
}

pub(crate) struct Inner {
    pub nodes: Arena<Node>,
    pub scopes: Arena<ScopeData>,
    pub events: Arena<EventData>,
    /// By node slot: where its latest occupant was created (for the disposed-handle panic).
    pub created: Vec<Loc>,
    pub root: Id,
    pub scope: Id,
    pub observer: Option<Id>,
    pub tracking: Tracking,
    pub stack: Vec<(Id, State)>,
    pub batch: u32,
    pub flushing: bool,
    pub flush: u64,
    pub effects: Vec<Id>,
    pub deliveries: VecDeque<Box<dyn FnOnce()>>,
    pub seq: u64,
    pub hook: Option<Hook>,
}

impl Inner {
    fn new() -> Inner {
        let mut scopes = Arena::new();
        let root = scopes.insert(ScopeData::default());
        Inner {
            nodes: Arena::new(),
            scopes,
            events: Arena::new(),
            created: Vec::new(),
            root,
            scope: root,
            observer: None,
            tracking: Tracking::default(),
            stack: Vec::new(),
            batch: 0,
            flushing: false,
            flush: 0,
            effects: Vec::new(),
            deliveries: VecDeque::new(),
            seq: 0,
            hook: None,
        }
    }

    pub fn scope_mut(&mut self) -> &mut ScopeData {
        let scope = self.scopes.get_mut(self.scope);
        scope.expect("the current scope was disposed: nothing can be created in it")
    }

    pub fn add_node(&mut self, mut node: Node) -> Id {
        self.seq += 1;
        node.seq = self.seq;
        let created = node.created;
        self.scope_mut(); // fail before anything is stored
        let id = self.nodes.insert(node);
        self.scope_mut().nodes.push(id);
        if self.created.len() <= id.index() {
            self.created.resize(id.index() + 1, created);
        }
        self.created[id.index()] = created;
        id
    }

    pub fn unlink(&mut self, source: Id, subscriber: Id) {
        if let Some(node) = self.nodes.get_mut(source)
            && let Some(at) = node.subscribers.iter().position(|&s| s == subscriber)
        {
            node.subscribers.swap_remove(at);
        }
    }

    /// Removes a node and its edges to its sources. Its readers keep a stale id, which every
    /// walk skips and their next run replaces.
    fn take_node(&mut self, id: Id) -> Option<Node> {
        let node = self.nodes.remove(id)?;
        for &source in &node.sources {
            self.unlink(source, id);
        }
        Some(node)
    }

    pub fn remove_scope(&mut self, id: Id) -> Option<Garbage> {
        let scope = self.scopes.remove(id)?;
        if let Some(parent) = scope.parent.and_then(|p| self.scopes.get_mut(p)) {
            parent.children.retain(|&c| c != id);
        }
        let nodes = scope.nodes.iter().filter_map(|&n| self.take_node(n)).collect();
        let events = scope.events.iter().filter_map(|&e| self.events.remove(e)).collect();
        Some((scope, nodes, events))
    }
}

thread_local! {
    static RUNTIME: RefCell<Inner> = RefCell::new(Inner::new());
}

/// Borrows the runtime for `f`. See the rule at the top of this file.
pub(crate) fn rt<R>(f: impl FnOnce(&mut Inner) -> R) -> R {
    RUNTIME.with(|cell| f(&mut cell.borrow_mut()))
}

/// The panic for a handle used after its scope ended: a programming error (specs/03).
#[cold]
pub(crate) fn disposed(what: &str, id: Id, at: Loc) -> ! {
    let created = rt(|r| if r.nodes.freed(id) { r.created.get(id.index()).copied() } else { None });
    match created {
        Some(created) => panic!("{what} used at {at} after it was disposed (created at {created})"),
        None => panic!("{what} used at {at} after it was disposed"),
    }
}

/// How much the runtime holds: tests assert it returns to where it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    /// Live signals, memos and effects.
    pub nodes: usize,
    /// Live scopes, the root included.
    pub scopes: usize,
    pub events: usize,
}

pub fn stats() -> Stats {
    rt(|r| Stats { nodes: r.nodes.len(), scopes: r.scopes.len(), events: r.events.len() })
}
