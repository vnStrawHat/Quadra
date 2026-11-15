//! The command palette (specs/behaviour/15, "Picker / command palette"): what the app can
//! do, found by typing a few letters of it. A query over the rows that match, hung under the
//! top of the window.

use std::rc::Rc;

pub(crate) use quadra_headless::pick::best;
use quadra_headless::pick::{Found, Parts};
pub use quadra_headless::pick::{fuzzy, fuzzy_hits};
use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, memo, signal};
use quadra_tree::{NodeId, State};
use quadra_ui::{Div, MountCx, Place, View, by_text, column, div, row, scroll, text, text_input};

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
    detail: Option<String>,
    run: Rc<dyn Fn()>,
}

impl Command {
    pub fn new(title: impl Into<String>, run: impl Fn() + 'static) -> Command {
        Command { title: title.into(), keys: None, detail: None, run: Rc::new(run) }
    }

    /// The binding that runs it, as bindings are written: "ctrl-shift-p".
    pub fn keys(mut self, keys: impl Into<String>) -> Self {
        self.keys = Some(keys.into());
        self
    }

    /// Words after the title, muted and not searched: where a file is, what a branch was.
    pub fn detail(mut self, words: impl Into<String>) -> Self {
        self.detail = Some(words.into());
        self
    }
}

/// The palette: of commands, or of anything else to pick (files, branches, symbols) as
/// commands whose doing is to take it, with words of its own for the query and for no match.
pub struct Palette<F> {
    open: Signal<bool>,
    commands: F,
    ask: String,
    none: String,
}

impl<F> Palette<F> {
    /// What the empty query says.
    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.ask = words.into();
        self
    }

    /// What shows when nothing matches.
    pub fn empty(mut self, words: impl Into<String>) -> Self {
        self.none = words.into();
        self
    }
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
pub fn command_palette<F>(open: Signal<bool>, commands: F) -> Palette<F>
where
    F: Fn() -> Vec<Command> + 'static,
{
    Palette { open, commands, ask: "Type a command…".into(), none: "No commands".into() }
}

impl<F: Fn() -> Vec<Command> + 'static> View for Palette<F> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Palette { open, commands, ask, none } = self;
        palette(open, commands, ask, Rc::from(none)).mount(cx)
    }
}

fn palette<F>(open: Signal<bool>, commands: F, ask: String, none: Rc<str>) -> impl View + use<F>
where
    F: Fn() -> Vec<Command> + 'static,
{
    popup(open, (Place::Top(80.0), 2), None, move || {
        let (all, query) = (Rc::new(commands()), signal(String::new()));
        // The commands that match, the best first; the order they were given in decides ties.
        let known = all.clone();
        let found =
            Found::new(memo(move || best(&query.get(), known.iter().map(|command| command.title.as_str()))));
        let run = {
            let (all, matches) = (all.clone(), found.matches);
            move |k: usize| {
                let Some(command) = matches.get().get(k).map(|&i| all[i].clone()) else { return };
                open.set(false); // the focus is back where it was before the command acts
                (command.run)();
            }
        };
        let on = found.on;
        // As wide as the palette: a row is filled, and its keys are at its end.
        let face = move |k: usize, i: usize| {
            let command = &all[i];
            let hits = fuzzy_hits(&query.get_untracked(), &command.title).unwrap_or_default();
            let line =
                row().align_items(Align::Center).gap(8.0).grow(1.0).height(by_text(ROW)).padding_x(6.0);
            let line = line.radius(4.0).label(command.title.clone());
            let line = line.hover(|s| s.bg(token::PANEL_HOVER)).when(State::SELECTED, |s| s.bg(token::PRESS));
            let line =
                line.child(crate::display::chosen_mark(quadra_tree::Side::Left, move || on.get() == k));
            let line = line.child(marked(&command.title, &hits));
            let line = match &command.detail {
                Some(detail) => line.child(text(detail.clone()).color(token::TEXT_2).ellipsis()),
                None => line,
            };
            let line = line.child(div().grow(1.0));
            match &command.keys {
                Some(keys) => line.child(text(key_label(keys)).color(token::TEXT_2)),
                None => line,
            }
        };
        let parts = Parts {
            list: Rc::new(|| column().grow(1.0).padding_y(4.0)),
            line: Some(Rc::new(|| div().padding_x(4.0))),
            row: Rc::new(face),
            empty: Some(Rc::new({
                let none = none.clone();
                move || {
                    div().padding_x(10.0).padding_y(4.0).child(text(none.to_string()).color(token::TEXT_2))
                }
            })),
        };
        // Another query: the best match is the one to take.
        let rows = found.rows(parts, |_| 0, run.clone());
        let (enter, submit, matches) = (run.clone(), run, found.matches);
        let field = text_input(query).placeholder(&ask).on_submit(move |_| submit(on.get_untracked()));
        let ask = row().align_items(Align::Center).height(by_text(37.0)).padding_x(10.0);
        let ask = ask.border_bottom(1.0).border_color(token::LINE).child(div().grow(1.0).child(field));
        let row_px = by_text(ROW);
        let tall = move || Length::Px((matches.get().len().max(1) as f32 * row_px() + 8.0).min(MOST));
        let list = scroll(signal(0.0), rows).container(div().height(tall));
        let panel = column().width(608.0 - EDGE).selection_color(token::SELECTION);
        found.keys(panel, enter).child(ask).child(list)
    })
}
