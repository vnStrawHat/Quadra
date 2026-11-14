//! The command palette (specs/behaviour/15, "Picker / command palette"): what the app can
//! do, found by typing a few letters of it. A query over the rows that match, hung under the
//! top of the window.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, memo, signal};
use quadra_text::fold_diacritics;
use quadra_tree::{Role, State};
use quadra_ui::{Div, NodeRef, Place, View, column, div, dynamic, row, scroll, text, text_input};

use crate::action::{Confirm, LIST, MoveNext, MovePrevious};
use crate::display::key_label;
use crate::popup::{EDGE, popup};
use crate::token;

/// The height of a row, and the most of the list that shows.
const ROW: f32 = 32.0;
const MOST: f32 = 347.0;

/// Something the app can do: what it is called, the keys that do it too, and the doing.
#[derive(Clone)]
pub struct Command {
    title: String,
    keys: Option<String>,
    run: Rc<dyn Fn()>,
}

impl Command {
    pub fn new(title: impl Into<String>, run: impl Fn() + 'static) -> Command {
        Command { title: title.into(), keys: None, run: Rc::new(run) }
    }

    /// The binding that runs it, as bindings are written: "ctrl-shift-p".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }
}

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
pub(crate) fn best<'a>(query: &str, texts: impl Iterator<Item = &'a str>) -> Vec<usize> {
    let scored = texts.enumerate().filter_map(|(i, text)| Some((fuzzy(query, text)?, i)));
    let mut scored: Vec<(i32, usize)> = scored.collect();
    scored.sort_by_key(|&(score, i)| (-score, i));
    scored.into_iter().map(|(_, i)| i).collect()
}

/// A text with the characters at `hits` in the accent: where a query matched it.
pub fn marked(words: &str, hits: &[usize]) -> Div {
    let mut line = row();
    let (mut run, mut on) = (String::new(), false);
    for (i, c) in words.chars().enumerate() {
        let hit = hits.contains(&i);
        if hit != on && !run.is_empty() {
            let piece = text(std::mem::take(&mut run));
            line = line.child(if on { piece.color(token::ACCENT_TEXT) } else { piece });
        }
        on = hit;
        run.push(c);
    }
    let piece = text(run);
    line.child(if on { piece.color(token::ACCENT_TEXT) } else { piece })
}

/// The palette while `open` is true. `commands` is asked each time it opens.
pub fn command_palette<F>(open: Signal<bool>, commands: F) -> impl View + use<F>
where
    F: Fn() -> Vec<Command> + 'static,
{
    popup(open, (Place::Top(80.0), 2), None, move || {
        let (all, query, on) = (Rc::new(commands()), signal(String::new()), signal(0usize));
        // The commands that match, the best first; the order they were given in decides ties.
        let known = all.clone();
        let found = memo(move || best(&query.get(), known.iter().map(|command| command.title.as_str())));
        let run = {
            let all = all.clone();
            move |k: usize| {
                let Some(command) = found.get().get(k).map(|&i| all[i].clone()) else { return };
                open.set(false); // the focus is back where it was before the command acts
                (command.run)();
            }
        };
        let refs: Rc<RefCell<Vec<NodeRef>>> = Rc::default();
        let (lines, chosen) = (refs.clone(), run.clone());
        let rows = dynamic(move || {
            let (matches, query) = (found.get(), query.get_untracked());
            on.set(0); // another query: the best match is the one to take
            *lines.borrow_mut() = matches.iter().map(|_| NodeRef::new()).collect();
            // As wide as the palette: a row is filled, and its keys are at its end.
            let mut list = column().grow(1.0).padding_y(4.0).role(Role::List);
            for (k, &i) in matches.iter().enumerate() {
                let (command, chosen) = (&all[i], chosen.clone());
                let hits = fuzzy_hits(&query, &command.title).unwrap_or_default();
                let line = row().align_items(Align::Center).gap(8.0).grow(1.0).height(ROW).padding_x(6.0);
                let line = line.radius(4.0).role(Role::ListItem).label(command.title.clone());
                let line = line.selected(move || on.get() == k).node_ref(&lines.borrow()[k]);
                let line =
                    line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
                let line = line.on_click(move |_| chosen(k));
                let line =
                    line.child(crate::display::chosen_mark(quadra_tree::Side::Left, move || on.get() == k));
                let line = line.child(marked(&command.title, &hits)).child(div().grow(1.0));
                let line = match &command.keys {
                    Some(keys) => line.child(text(key_label(keys)).color(token::TEXT_2)),
                    None => line,
                };
                list = list.child(div().padding_x(4.0).child(line));
            }
            if matches.is_empty() {
                let none = text("No commands").color(token::TEXT_2);
                list = list.child(div().padding_x(10.0).padding_y(4.0).child(none));
            }
            list
        });
        // The arrows go round the matches and keep the one they are on in view.
        let step = move |forward: bool| {
            let count = found.get().len();
            if count > 0 {
                on.update(|at| *at = if forward { (*at + 1) % count } else { (*at + count - 1) % count });
                refs.borrow()[on.get_untracked()].scroll_into_view();
            }
        };
        let (down, up, enter, submit) = (step.clone(), step, run.clone(), run);
        let field =
            text_input(query).placeholder("Type a command…").on_submit(move |_| submit(on.get_untracked()));
        let ask = row().align_items(Align::Center).height(37.0).padding_x(10.0);
        let ask = ask.border_bottom(1.0).border_color(token::LINE).child(div().grow(1.0).child(field));
        let tall = move || Length::Px((found.get().len().max(1) as f32 * ROW + 8.0).min(MOST));
        let list = scroll(signal(0.0), rows).container(div().height(tall));
        let panel = column().width(608.0 - EDGE).key_context(LIST).selection_color(token::SELECTION);
        let panel = panel.on_action::<MoveNext>(move |_| down(true));
        let panel = panel.on_action::<MovePrevious>(move |_| up(false));
        panel.on_action::<Confirm>(move |_| enter(on.get_untracked())).child(ask).child(list)
    })
}
