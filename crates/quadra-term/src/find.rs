//! Finding in a terminal (specs/18, "Find"): the engine's matcher over a copy of the active
//! screen's text, history with it; the matches by row id, the current one, the count as a
//! signal; the view scrolled to the current match, and the matches painted under the text.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use oneterm_vt::SnapshotRow;
use oneterm_vt::search::{GridText, SearchMatch, SearchOptions, SearchPattern, search_grid_text};
use quadra_async::{Timers, timeout};
use quadra_reactive::{Signal, signal, use_context};
use quadra_render::Quad;
use quadra_tree::PaintCx;

use crate::session::Session;
use crate::widget::TerminalWidget;

/// The wait after output before its text is matched again.
const AFTER_OUTPUT: Duration = Duration::from_millis(150);

/// What is looked for: a literal `text`, matched exactly with `case` (else lower-cased), and
/// only as a whole word with `word`. An empty text looks for nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindQuery {
    pub text: String,
    pub case: bool,
    pub word: bool,
}

/// A session's finding: the query, its matches top to bottom, the current one.
pub(crate) struct Finding {
    query: RefCell<FindQuery>,
    matches: RefCell<Vec<SearchMatch>>,
    current: Cell<Option<usize>>,
    found: Signal<(usize, usize)>,
    /// A search again after output is on its way.
    waiting: Cell<bool>,
}

impl Finding {
    pub fn new() -> Finding {
        Finding {
            query: RefCell::default(),
            matches: RefCell::default(),
            current: Cell::new(None),
            found: signal((0, 0)),
            waiting: Cell::new(false),
        }
    }
}

impl Session {
    /// Looks for `query` (an empty text ends finding): the current match is the first at or
    /// below the top of the view, else the last, scrolled into view.
    pub fn find(&self, query: &FindQuery) {
        *self.0.find.query.borrow_mut() = query.clone();
        let matches = self.search();
        let top = self.engine().term.screen().visible_top();
        let first = matches.partition_point(|m| m.row < top);
        let current = (!matches.is_empty()).then(|| first.min(matches.len() - 1));
        self.set_matches(matches, current);
        self.reveal();
        self.repaint();
    }

    /// The next match (`back`: the one before), wrapping, scrolled into view.
    pub fn find_next(&self, back: bool) {
        let count = self.0.find.matches.borrow().len();
        let Some(at) = self.0.find.current.get().filter(|_| count > 0) else { return };
        let at = if back { (at + count - 1) % count } else { (at + 1) % count };
        let matches = std::mem::take(&mut *self.0.find.matches.borrow_mut());
        self.set_matches(matches, Some(at));
        self.reveal();
        self.repaint();
    }

    /// What is looked for; the default when nothing is.
    pub fn find_query(&self) -> FindQuery {
        self.0.find.query.borrow().clone()
    }

    /// The current match counted from 1, and how many there are; `(0, 0)` for none.
    pub fn found(&self) -> Signal<(usize, usize)> {
        self.0.find.found
    }

    /// Matches the text again, keeping the current match where it still is; no scrolling.
    pub(crate) fn refind(&self) {
        if self.0.find.query.borrow().text.is_empty() {
            return;
        }
        let kept = self.0.find.current.get().and_then(|at| self.0.find.matches.borrow().get(at).copied());
        let matches = self.search();
        let current = match kept {
            _ if matches.is_empty() => None,
            Some(m) => Some(matches.partition_point(|n| (n.row, n.start_col) < (m.row, m.start_col))),
            None => Some(0),
        };
        let current = current.map(|at| at.min(matches.len().saturating_sub(1)));
        self.set_matches(matches, current);
        self.repaint();
    }

    /// Output came: matched again after a wait, once for the output of that wait.
    pub(crate) fn refind_later(&self) {
        let f = &self.0.find;
        if f.query.borrow().text.is_empty() || f.waiting.get() {
            return;
        }
        if use_context::<Rc<Timers>>().is_none() {
            return self.refind();
        }
        f.waiting.set(true);
        let weak = Rc::downgrade(&self.0);
        timeout(AFTER_OUTPUT, move || {
            if let Some(shared) = weak.upgrade() {
                shared.find.waiting.set(false);
                Session(shared).refind();
            }
        });
    }

    /// The matches of the query: the text copied under the lock, matched after it.
    fn search(&self) -> Vec<SearchMatch> {
        let query = self.0.find.query.borrow().clone();
        if query.text.is_empty() {
            return Vec::new();
        }
        let text = GridText::from_terminal(&self.engine().term);
        let mut options = SearchOptions::default();
        (options.case_sensitive, options.whole_word) = (query.case, query.word);
        // ponytail: matched on the UI thread; on the pool (`background`) if long histories
        // make a search take more than a frame.
        search_grid_text(&text, SearchPattern::Literal(&query.text), options)
    }

    fn set_matches(&self, matches: Vec<SearchMatch>, current: Option<usize>) {
        let f = &self.0.find;
        f.found.set(current.map_or((0, 0), |at| (at + 1, matches.len())));
        *f.matches.borrow_mut() = matches;
        f.current.set(current);
    }

    /// Scrolls the view so the current match stands in its middle, when it is out of view.
    fn reveal(&self) {
        let Some(m) = self.0.find.current.get().and_then(|at| self.0.find.matches.borrow().get(at).copied())
        else {
            return;
        };
        let mut engine = self.engine();
        let screen = engine.term.screen();
        let (top, rows) = (screen.visible_top(), u64::from(screen.size().rows));
        if m.row >= top && m.row.0 < top.0 + rows {
            return;
        }
        let want = screen.screen_top().0 as i64 - m.row.0 as i64 + (rows / 2) as i64;
        let want = want.clamp(0, i64::from(screen.history_len()));
        let delta = i64::from(screen.scroll_offset()) - want;
        engine.term.scroll_viewport(delta.clamp(i32::MIN.into(), i32::MAX.into()) as i32);
    }
}

/// The matches on a row: a quad over the cells of each (on each row of one that wraps), the
/// current one in its own colour.
pub(crate) fn paint(w: &TerminalWidget, cx: &mut PaintCx, row: &SnapshotRow, at: (f32, f32)) {
    let f = &w.session.0.find;
    let matches = f.matches.borrow();
    // Matches do not overlap, so their ends run in order as their starts do.
    let from = matches.partition_point(|m| m.end_row < row.id);
    for (i, m) in matches.iter().enumerate().skip(from).take_while(|(_, m)| m.row <= row.id) {
        let start = if m.row == row.id { m.start_col } else { 0 };
        let end = if m.end_row == row.id { m.end_col } else { row.cells.len() };
        if end > start {
            let background = if f.current.get() == Some(i) { w.find.1 } else { w.find.0 };
            let bounds = crate::paint::cells(w, at, start, end - start);
            cx.quad(Quad { bounds, background, ..Quad::default() });
        }
    }
}
