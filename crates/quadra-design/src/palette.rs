//! The command palette (specs/design, "Picker / command palette"): what the app can
//! do, found by typing a few letters of it. A query over the rows that match, in groups
//! under headings, hung under the top of the window or under the control that opens it.

use std::collections::HashMap;
use std::rc::Rc;

use quadra_headless::pick::Found;
pub(crate) use quadra_headless::pick::best;
pub use quadra_headless::pick::{fuzzy, fuzzy_hits};
use quadra_layout::Length;
use quadra_reactive::{Memo, Signal, memo, signal};
use quadra_tree::NodeId;
use quadra_view::{Div, MountCx, NodeRef, Place, View, by_text, column, div, row, scroll, text, text_input};

use crate::popup::{EDGE, below, listing, popup};
use crate::size::TEXT_SM;
use crate::{Status, icon, icons, key_hint, token};

mod face;
pub use face::marked;

/// The most of the list that shows.
const MOST: f32 = 347.0;

/// How its rows look: their height and their headings', whether they are dense (a state's
/// mark first, a slot for the check), and whether the row the arrows are on says "Enter".
#[derive(Clone, Copy)]
pub(crate) struct Look {
    row: f32,
    head: f32,
    dense: bool,
    enter: bool,
}

/// Something the app can do: what it is called, the keys that do it too, and the doing.
#[derive(Clone)]
pub struct Command {
    title: String,
    keys: Option<String>,
    detail: Option<String>,
    group: Option<Rc<str>>,
    lead: Option<Rc<dyn Fn() -> Div>>,
    status: Option<(Status, String)>,
    current: bool,
    mono: bool,
    run: Rc<dyn Fn()>,
}

impl Command {
    pub fn new(title: impl Into<String>, run: impl Fn() + 'static) -> Command {
        let (keys, detail, group, lead, status) = (None, None, None, None, None);
        let (current, mono, run) = (false, false, Rc::new(run));
        Command { title: title.into(), keys, detail, group, lead, status, current, mono, run }
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

    /// The heading it is under. The commands of a group stay together, in the order given;
    /// the groups come in the order of their first commands.
    pub fn group(mut self, heading: impl Into<String>) -> Self {
        self.group = Some(Rc::from(heading.into()));
        self
    }

    /// What goes before the title (`badge("Node", Tone::Neutral)`), built for each row.
    pub fn lead(mut self, part: impl Fn() -> Div + 'static) -> Self {
        self.lead = Some(Rc::new(part));
        self
    }

    /// A state at the end of its row, with its mark: `status_label`'s.
    pub fn status(mut self, status: Status, words: impl Into<String>) -> Self {
        self.status = Some((status, words.into()));
        self
    }

    /// What is chosen now: first, with a check at the end of its row.
    pub fn current(mut self) -> Self {
        self.current = true;
        self
    }

    /// Its title in the mono face: a name that is code-like (a cluster, a resource).
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
}

type Foot = (Rc<dyn Fn() -> Div>, Rc<dyn Fn(usize) -> String>);

/// The palette: of commands, or of anything else to pick (files, branches, symbols) as
/// commands whose doing is to take it, with words of its own for the query and for no match.
pub struct Palette<F> {
    open: Signal<bool>,
    commands: F,
    ask: String,
    none: String,
    note: Option<Rc<dyn Fn() -> String>>,
    foot: Option<Foot>,
    footnote: Option<String>,
    under: Option<NodeRef>,
    width: f32,
    look: Look,
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

    /// Muted words at the end of the query: where it searches ("in prod-cluster").
    pub fn note(mut self, words: impl Fn() -> String + 'static) -> Self {
        self.note = Some(Rc::new(words));
        self
    }

    /// A line under the rows: `keys` at its start ("Enter open · Esc close"), and at its end
    /// what `count` says of the number of matches ("6 results").
    pub fn foot(mut self, keys: impl Into<String>, count: impl Fn(usize) -> String + 'static) -> Self {
        let keys = keys.into();
        let says = move || {
            row().min_width(0.0).child(text(keys.clone()).color(token::TEXT_3).size(TEXT_SM.size).ellipsis())
        };
        self.foot = Some((Rc::new(says), Rc::new(count)));
        self
    }

    /// The line under the rows with each key a `key_hint` ("enter" and "open"), then the count.
    pub fn foot_hints<K: Into<String>, D: Into<String>>(
        mut self,
        hints: impl IntoIterator<Item = (K, D)>,
        count: impl Fn(usize) -> String + 'static,
    ) -> Self {
        let hints: Vec<(String, String)> = hints.into_iter().map(|(k, d)| (k.into(), d.into())).collect();
        let says = move || {
            row().gap(token::SPACE_LG).children(hints.iter().map(|(keys, does)| key_hint(keys, does.clone())))
        };
        self.foot = Some((Rc::new(says), Rc::new(count)));
        self
    }

    /// Muted words under the rows, wrapped: what choosing means.
    pub fn footnote(mut self, words: impl Into<String>) -> Self {
        self.footnote = Some(words.into());
        self
    }

    /// Rows of 23 px, a state's mark first and a slot for the check: a picker of names.
    pub fn dense(mut self) -> Self {
        (self.look.row, self.look.head, self.look.dense) = (24.0, 24.0, true);
        self
    }

    /// Whether the row the arrows are on says "Enter": not in a picker that only chooses.
    pub fn hint(mut self, on: bool) -> Self {
        self.look.enter = on;
        self
    }

    /// Hung under `opener`, a control (the name in a title bar), instead of under the top of
    /// the window; a click on the opener does not open it again.
    pub fn under(mut self, opener: &NodeRef) -> Self {
        self.under = Some(opener.clone());
        self
    }

    /// Its width: 608 px unless told (280 under a control).
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }
}

/// The palette while `open` is true. `commands` is asked each time it opens.
pub fn command_palette<F>(open: Signal<bool>, commands: F) -> Palette<F>
where
    F: Fn() -> Vec<Command> + 'static,
{
    let (ask, none) = ("Type a command…".into(), "No commands".into());
    let look = Look { row: 28.0, head: 24.0, dense: false, enter: true };
    let (note, foot, footnote, under) = (None, None, None, None);
    Palette { open, commands, ask, none, note, foot, footnote, under, width: 608.0, look }
}

impl<F: Fn() -> Vec<Command> + 'static> View for Palette<F> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        palette(self).mount(cx)
    }
}

/// The commands that match, the best first, the order given deciding ties; the rows of a
/// group together, the groups in the order of their first commands; the current one first.
fn order(all: Rc<Vec<Command>>, query: Signal<String>) -> Memo<Vec<usize>> {
    let mut firsts = HashMap::new();
    let rank: Vec<usize> = all
        .iter()
        .enumerate()
        .map(|(i, command)| *firsts.entry(command.group.clone()).or_insert(i))
        .collect();
    let lead = all.iter().position(|command| command.current).map(|i| rank[i]);
    memo(move || {
        let mut found = best(&query.get(), all.iter().map(|command| command.title.as_str()));
        found.sort_by_key(|&i| (Some(rank[i]) != lead, rank[i], !all[i].current));
        found
    })
}

fn palette<F: Fn() -> Vec<Command> + 'static>(p: Palette<F>) -> impl View + use<F> {
    let Palette { open, commands, ask, none, note, foot, footnote, under, width, look } = p;
    let (none, place) = (Rc::<str>::from(none), under.as_ref().map_or(Place::Top(80.0), below));
    popup(open, listing(place), under, move || {
        let (all, query) = (Rc::new(commands()), signal(String::new()));
        let found = Found::new(order(all.clone(), query));
        let (on, matches) = (found.on, found.matches);
        let run = {
            let all = all.clone();
            move |k: usize| {
                let Some(command) = matches.get().get(k).map(|&i| all[i].clone()) else { return };
                open.set(false); // the focus is back where it was before the command acts
                (command.run)();
            }
        };
        // Another query: the best match is the one to take.
        let rows = found.rows(face::parts(all.clone(), query, on, none.clone(), look), |_| 0, run.clone());
        let (enter, submit) = (run.clone(), run);
        let field = text_input(query).placeholder(&ask).on_submit(move |_| submit(on.get_untracked()));
        let ask = crate::layout::hstack().height(by_text(37.0)).padding_x(token::SPACE_LG);
        let ask = ask.border_bottom(1.0).border_color(token::LINE);
        let ask = ask.child(icon(icons::SEARCH).size(14.0).color(token::TEXT_3));
        let ask = ask.child(div().grow(1.0).child(field));
        let ask = match note.clone() {
            Some(note) => {
                let words = text(move || note()).color(token::TEXT_2).size(TEXT_SM.size);
                ask.child(row().shrink(0.0).font_family(crate::fonts::MONO).child(words))
            }
            None => ask,
        };
        // As tall as the rows and the headings, up to the most.
        let (row_px, head_px) = (by_text(look.row), by_text(look.head));
        let tall = move || {
            let heads = matches.with(|found| {
                let before = |k: usize| k.checked_sub(1).map(|b| found[b]);
                (0..found.len()).filter(|&k| face::heading(&all, before(k), found[k]).is_some()).count()
            });
            let rows = matches.get().len().max(1) as f32 * row_px() + heads as f32 * head_px();
            Length::Px((rows + 8.0).min(MOST))
        };
        let list = scroll(signal(0.0), rows).container(div().height(tall));
        let panel = column().width(width - EDGE).selection_color(token::SELECTION);
        let panel = found.keys(panel, enter).child(ask).child(list);
        let panel = match footnote.clone() {
            Some(words) => {
                let words = text(words).color(token::TEXT_3).size(TEXT_SM.size);
                panel.child(
                    div()
                        .padding_x(token::SPACE_LG)
                        .padding_y(token::SPACE_MD)
                        .border_top(1.0)
                        .border_color(token::LINE)
                        .child(words),
                )
            }
            None => panel,
        };
        match foot.clone() {
            Some(foot) => panel.child(foot_line(foot, matches)),
            None => panel,
        }
    })
}

/// The line under the rows: the keys, and the count of the matches at its end.
fn foot_line((keys, count): Foot, matches: Memo<Vec<usize>>) -> Div {
    let small = |line: quadra_view::TextView| line.color(token::TEXT_3).size(TEXT_SM.size);
    let says = row().shrink(0.0).child(small(text(move || count(matches.with(Vec::len)))));
    let line = crate::layout::hstack().height(by_text(28.0)).padding_x(token::SPACE_LG);
    let line = line.border_top(1.0).border_color(token::LINE);
    line.child(keys()).child(div().grow(1.0)).child(says)
}
