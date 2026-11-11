//! Requests and answers (specs/06, "Requests and answers"): what a parent asks a child, and the
//! few results a node keeps.

use quadra_prim::Size;

/// Results kept per node. A flex container asks a child for its base size, then for its cross
/// size at the final main size, then performs it; nested containers repeat that.
const SLOTS: usize = 4;

/// What a parent asks: a size it fixes per axis, or the space available on that axis
/// (infinite: no limit).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Request {
    pub kw: Option<f32>,
    pub kh: Option<f32>,
    pub aw: f32,
    pub ah: f32,
}

impl Request {
    /// A size fixed by the parent makes the space available on that axis irrelevant, so
    /// requests that differ only there are one request.
    pub fn new(kw: Option<f32>, kh: Option<f32>, aw: f32, ah: f32) -> Request {
        Request { kw, kh, aw: kw.unwrap_or(aw), ah: kh.unwrap_or(ah) }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Entry {
    pub req: Request,
    pub out: Size,
}

/// What a node told its parent: its cached results and its last perform (the request at which
/// it placed its children). A node that gives the same answers leaves its parent's layout as
/// it was.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Answers {
    cache: [Entry; SLOTS],
    valid: u8,
    next: u8,
    pslot: u8,
    /// Performed: the rects below are that layout's.
    pub done: bool,
    /// Measured since that perform: a leaf may hold the state of the measure (a text wrap).
    probed: bool,
    /// The cache overwrote an answer: what the parent was told is no longer all known.
    pub lost: bool,
}

impl Answers {
    fn has(&self, i: usize) -> bool {
        self.valid & (1 << i) != 0
    }

    /// True when the node was never asked anything: no answer of any ancestor depends on it.
    pub fn is_empty(&self) -> bool {
        self.valid == 0
    }

    pub fn cached(&self, req: &Request) -> Option<Size> {
        (0..SLOTS).find(|&i| self.has(i) && self.cache[i].req == *req).map(|i| self.cache[i].out)
    }

    /// The result of the last perform if it was at this request and nothing measured the node
    /// since: every rect under it still holds.
    pub fn performed(&self, req: &Request) -> Option<Size> {
        let entry = &self.cache[self.pslot as usize];
        (self.done && !self.probed && entry.req == *req).then_some(entry.out)
    }

    /// Records a result. A perform's entry is never the one replaced.
    pub fn store(&mut self, req: Request, out: Size, perform: bool) {
        self.probed = !perform;
        if perform && let Some(i) = (0..SLOTS).find(|&i| self.has(i) && self.cache[i].req == req) {
            self.cache[i].out = out;
            self.pslot = i as u8;
            self.done = true;
            return;
        }
        if self.done && self.next == self.pslot {
            self.next = (self.next + 1) % SLOTS as u8;
        }
        let i = self.next as usize;
        if self.has(i) {
            self.lost = true;
        }
        self.cache[i] = Entry { req, out };
        self.valid |= 1 << i;
        self.next = (self.next + 1) % SLOTS as u8;
        if perform {
            self.pslot = i as u8;
            self.done = true;
        }
    }

    /// The measure answers, without the perform.
    pub fn measures(&self) -> impl Iterator<Item = Entry> + '_ {
        (0..SLOTS).filter(|&i| self.has(i) && !(self.done && i == self.pslot as usize)).map(|i| self.cache[i])
    }

    pub fn perform(&self) -> Option<Entry> {
        self.done.then(|| self.cache[self.pslot as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(w: f32) -> Request {
        Request::new(None, None, w, f32::INFINITY)
    }

    #[test]
    fn a_fixed_size_makes_the_available_space_irrelevant() {
        assert_eq!(Request::new(Some(10.0), None, 500.0, 80.0), Request::new(Some(10.0), None, 9.0, 80.0));
    }

    #[test]
    fn the_perform_entry_survives_and_a_lost_answer_is_noticed() {
        let mut a = Answers::default();
        assert!(a.is_empty());
        a.store(req(1.0), Size::new(1.0, 1.0), true);
        assert_eq!(a.performed(&req(1.0)), Some(Size::new(1.0, 1.0)));
        for w in 2..=4 {
            a.store(req(w as f32), Size::new(w as f32, 1.0), false);
        }
        assert!(!a.lost && a.performed(&req(1.0)).is_none()); // measured since: perform again
        a.store(req(5.0), Size::ZERO, false); // a fifth answer replaces a measure, not the perform
        assert!(a.lost && a.cached(&req(1.0)).is_some() && a.cached(&req(2.0)).is_none());
        assert_eq!(a.perform().map(|e| e.req), Some(req(1.0)));
        assert_eq!(a.measures().count(), 3);
    }
}
