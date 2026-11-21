//! What is picked by typing (specs/17, "The components"): the texts a query matches, in
//! order, whatever their case and accents, the best first; the arrows go round the matches
//! with the focus in the field, keeping the one they are on in view, and Enter or a click
//! takes one. The command palette, a select that is searched and the combobox are this. The
//! look gives the field, the box of the rows, the face of each and what no match says.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_reactive::{Memo, Signal, signal};
use quadra_text::fold_diacritics;
use quadra_tree::Role;
use quadra_view::{Div, NodeRef, View, dynamic};

use crate::action::{Confirm, LIST, MoveNext, MovePrevious};

/// Where the letters of a query are in a text, in order, whatever their case and accents:
/// the first place of each, as indexes of characters. `None` when one is missing.
pub fn fuzzy_hits(query: &str, text: &str) -> Option<Vec<usize>> {
    let plain = |text: &str| fold_diacritics(text).to_lowercase();
    let (query, mut hits) = (plain(query), Vec::new());
    let mut wanted = query.chars().peekable();
    for (i, c) in plain(text).chars().enumerate() {
        if wanted.next_if_eq(&c).is_some() {
            hits.push(i);
        }
    }
    wanted.peek().is_none().then_some(hits)
}

/// How well a query matches a text: more for letters that follow one another and for a match
/// at the start. `None` when it does not match.
pub fn fuzzy(query: &str, text: &str) -> Option<i32> {
    let (mut score, mut last) = (0, None);
    for at in fuzzy_hits(query, text)? {
        score += if last.is_some_and(|last| at == last + 1) { 5 } else { 1 };
        score += if at == 0 { 3 } else { 0 };
        last = Some(at);
    }
    Some(score)
}

/// The texts a query matches, as their indexes: the best match first, and of equal ones the
/// one that was given first.
pub fn best<'a>(query: &str, texts: impl Iterator<Item = &'a str>) -> Vec<usize> {
    let scored = texts.enumerate().filter_map(|(i, text)| Some((fuzzy(query, text)?, i)));
    let mut scored: Vec<(i32, usize)> = scored.collect();
    scored.sort_by_key(|&(score, i)| (-score, i));
    scored.into_iter().map(|(_, i)| i).collect()
}

/// What goes before a row, given the text above it and its own.
pub type Head = Rc<dyn Fn(Option<usize>, usize) -> Option<Div>>;

/// The parts of the rows of what is found, from the look.
#[derive(Clone)]
pub struct Parts {
    /// The box of the rows, built again when the matches change.
    pub list: Rc<dyn Fn() -> Div>,
    /// The box around a row, if the look has one.
    pub line: Option<Rc<dyn Fn() -> Div>>,
    /// The face of the row at `k` of the matches, which is the text at `i`.
    pub row: Rc<dyn Fn(usize, usize) -> Div>,
    /// What the rows say when nothing matches.
    pub empty: Option<Rc<dyn Fn() -> Div>>,
    /// What goes before the row of text `i` after the row of text `before` (a heading of
    /// a group), if anything: no row of the list, and nothing the arrows stop at.
    pub head: Option<Head>,
}

/// What a query found: its matches, and the one of them the arrows are on.
#[derive(Clone)]
pub struct Found {
    pub matches: Memo<Vec<usize>>,
    pub on: Signal<usize>,
    refs: Rc<RefCell<Vec<NodeRef>>>,
}

impl Found {
    pub fn new(matches: Memo<Vec<usize>>) -> Found {
        Found { matches, on: signal(0), refs: Rc::default() }
    }

    /// The index of the text the arrows are on, if anything matches.
    pub fn taken(&self) -> Option<usize> {
        self.matches.get().get(self.on.get_untracked()).copied()
    }

    /// To the next match or the one before, round the ends, brought into view.
    pub fn step(&self, forward: bool) {
        let count = self.matches.get().len();
        if count > 0 {
            self.on.update(|at| *at = if forward { (*at + 1) % count } else { (*at + count - 1) % count });
            if let Some(row) = self.refs.borrow().get(self.on.get_untracked()) {
                row.scroll_into_view();
            }
        }
    }

    /// The rows of the matches, built again when they change; the arrows start from the one
    /// `start` gives. A click on a row takes it.
    pub fn rows<S, T>(&self, parts: Parts, start: S, take: T) -> impl View + use<S, T>
    where
        S: Fn(&[usize]) -> usize + 'static,
        T: Fn(usize) + 'static,
    {
        let (found, take) = (self.clone(), Rc::new(take));
        dynamic(move || {
            let matches = found.matches.get();
            let on = found.on;
            on.set(start(&matches));
            *found.refs.borrow_mut() = matches.iter().map(|_| NodeRef::new()).collect();
            let mut list = (parts.list)().role(Role::List);
            for (k, &i) in matches.iter().enumerate() {
                let before = k.checked_sub(1).map(|b| matches[b]);
                if let Some(head) = parts.head.as_ref().and_then(|head| head(before, i)) {
                    list = list.child(head);
                }
                let take = take.clone();
                let face = (parts.row)(k, i).role(Role::ListItem).selected(move || on.get() == k);
                let face = face.node_ref(&found.refs.borrow()[k]).on_click(move |_| take(k));
                list = list.child(match &parts.line {
                    Some(line) => line().child(face),
                    None => face,
                });
            }
            match (&parts.empty, matches.is_empty()) {
                (Some(empty), true) => list.child(empty()),
                _ => list,
            }
        })
    }

    /// The keys of what is found on `panel`, the box around the field and the rows: the
    /// arrows step, Enter takes the one they are on.
    pub fn keys(&self, panel: Div, take: impl Fn(usize) + 'static) -> Div {
        let (down, up, on) = (self.clone(), self.clone(), self.on);
        let panel = panel.key_context(LIST).on_action::<MoveNext>(move |_| down.step(true));
        let panel = panel.on_action::<MovePrevious>(move |_| up.step(false));
        panel.on_action::<Confirm>(move |_| take(on.get_untracked()))
    }
}
