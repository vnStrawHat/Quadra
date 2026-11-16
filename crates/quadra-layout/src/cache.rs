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

/// The requests an answer also holds for, beside its own (specs/06, "Measuring leaves"): the
/// available widths in `aw`, both ends included, and with `any_h` any available height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Span {
    pub aw: (f32, f32),
    pub any_h: bool,
}

impl Span {
    /// Only the request itself: no width is in an empty range.
    pub const EXACT: Span = Span { aw: (f32::INFINITY, f32::NEG_INFINITY), any_h: false };

    /// Holds for every request `old` held for.
    pub fn covers(&self, old: &Span) -> bool {
        let width = old.aw.0 > old.aw.1 || (self.aw.0 <= old.aw.0 && old.aw.1 <= self.aw.1);
        width && (self.any_h || !old.any_h)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Entry {
    pub req: Request,
    pub out: Size,
    pub span: Span,
}

impl Entry {
    fn answers(&self, r: &Request) -> bool {
        let (e, span) = (&self.req, &self.span);
        let aw = e.aw == r.aw || (span.aw.0 <= r.aw && r.aw <= span.aw.1);
        e.kw == r.kw && e.kh == r.kh && aw && (e.ah == r.ah || span.any_h)
    }
}

/// What a node told its parent: its cached results and its last perform (the request at which
/// it placed its children). A node that gives the same answers leaves its parent's layout as
/// it was.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Answers {
    cache: [Entry; SLOTS],
    valid: u8,
    /// Entries a measure asked for. One only a perform asked for is forgotten when the node
    /// is performed at another request: the parent that placed it there has moved on.
    measured: u8,
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

    /// The answer to a measure, if it is known.
    pub fn cached(&mut self, req: &Request) -> Option<Size> {
        let i = (0..SLOTS).find(|&i| self.has(i) && self.cache[i].answers(req))?;
        self.measured |= 1 << i;
        Some(self.cache[i].out)
    }

    /// What the known answer to a measure holds for: the entry `cached` answers it from.
    pub fn span(&self, req: &Request) -> Option<Span> {
        (0..SLOTS).find(|&i| self.has(i) && self.cache[i].answers(req)).map(|i| self.cache[i].span)
    }

    /// The result of the last perform if it was at this request and nothing measured the node
    /// since: every rect under it still holds.
    pub fn performed(&self, req: &Request) -> Option<Size> {
        let entry = &self.cache[self.pslot as usize];
        (self.done && !self.probed && entry.req == *req).then_some(entry.out)
    }

    /// Records a result. A perform's entry is never the one replaced.
    pub fn store(&mut self, req: Request, out: Size, perform: bool, span: Span) {
        self.probed = !perform;
        let known = (0..SLOTS).find(|&i| self.has(i) && self.cache[i].req == req);
        if perform
            && self.done
            && known != Some(self.pslot as usize)
            && self.measured & (1 << self.pslot) == 0
        {
            self.valid &= !(1 << self.pslot); // the last perform, which nothing else asked for
        }
        if perform && let Some(i) = known {
            self.cache[i].out = out;
            self.cache[i].span = span;
            self.pslot = i as u8;
            self.done = true;
            return;
        }
        if self.done && self.has(self.pslot as usize) && self.next == self.pslot {
            self.next = (self.next + 1) % SLOTS as u8;
        }
        let i = self.next as usize;
        if self.has(i) {
            self.lost = true;
        }
        self.cache[i] = Entry { req, out, span };
        self.valid |= 1 << i;
        self.measured = if perform { self.measured & !(1 << i) } else { self.measured | 1 << i };
        self.next = (self.next + 1) % SLOTS as u8;
        if perform {
            self.pslot = i as u8;
            self.done = true;
        }
    }

    /// The measure answers, without the perform, in the order they were asked: a parent asks
    /// for a size first and then, with that size, for more.
    pub fn measures(&self) -> impl Iterator<Item = Entry> + '_ {
        let oldest_first = (0..SLOTS).map(|i| (self.next as usize + i) % SLOTS);
        oldest_first
            .filter(|&i| self.has(i) && !(self.done && i == self.pslot as usize))
            .map(|i| self.cache[i])
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
        a.store(req(1.0), Size::new(1.0, 1.0), true, Span::EXACT);
        assert_eq!(a.performed(&req(1.0)), Some(Size::new(1.0, 1.0)));
        for w in 2..=4 {
            a.store(req(w as f32), Size::new(w as f32, 1.0), false, Span::EXACT);
        }
        assert!(!a.lost && a.performed(&req(1.0)).is_none()); // measured since: perform again
        a.store(req(5.0), Size::ZERO, false, Span::EXACT); // a fifth answer replaces a measure, not the perform
        assert!(a.lost && a.cached(&req(1.0)).is_some() && a.cached(&req(2.0)).is_none());
        assert_eq!(a.perform().map(|e| e.req), Some(req(1.0)));
        assert_eq!(a.measures().count(), 3);
    }
}
