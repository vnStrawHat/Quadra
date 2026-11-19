//! Find in Preview (specs/21, "Find"): a bar at the top right of the preview, its query, its
//! switches, the count and the way to the next match. A tab's own.

use quadra::{
    Action, Align, Div, Key, KeyEvent, Mods, NodeRef, Role, Signal, View, column, effect, memo, row, show,
    signal, text, untrack,
};
use quadra_code::Query;
use quadra_design::{Size, Variant, button, icon_button, icons, input, size, token};

/// Finds in the document (Ctrl+F): the editor's bar while it shows, else Preview's.
#[derive(Clone, Copy, Debug)]
pub struct Find;

impl Action for Find {
    const NAME: &'static str = "markdown::Find";
}

/// The next match (F3), or with `true` the one before (Shift+F3).
#[derive(Clone, Copy, Debug)]
pub struct FindNext(pub bool);

impl Action for FindNext {
    const NAME: &'static str = "markdown::FindNext";
}

/// What Preview's find holds.
#[derive(Clone)]
pub struct Search {
    pub open: Signal<bool>,
    pub text: Signal<String>,
    pub case: Signal<bool>,
    pub word: Signal<bool>,
    /// What the preview looks for: the bar's words while it is open.
    pub query: Signal<Option<Query>>,
    pub current: Signal<usize>,
    pub count: Signal<usize>,
    bar: NodeRef,
}

impl Default for Search {
    fn default() -> Search {
        Search::new()
    }
}

impl Search {
    /// Call it in the scope of the tab.
    pub fn new() -> Search {
        let s = Search {
            open: signal(false),
            text: signal(String::new()),
            case: signal(false),
            word: signal(false),
            query: signal(None),
            current: signal(0),
            count: signal(0),
            bar: NodeRef::new(),
        };
        let (open, words, case, word, query, current) = (s.open, s.text, s.case, s.word, s.query, s.current);
        effect(move || {
            let now = open.get().then(|| Query {
                text: words.get(),
                case: case.get(),
                word: word.get(),
                regex: false,
            });
            if untrack(|| query.get()) != now {
                query.set(now);
                current.set(0);
            }
        });
        s
    }

    /// What the preview is given.
    pub fn find(&self) -> quadra_md::Find {
        quadra_md::Find { query: self.query, current: self.current, count: self.count }
    }

    /// Opens the bar, or puts the focus back in its query.
    pub fn show(&self) {
        if self.open.get_untracked() {
            self.bar.focus_first();
        }
        self.open.set(true);
    }

    /// The next match, or with `back` the one before, round.
    pub fn step(&self, back: bool) {
        let (n, at) = (self.count.get_untracked(), self.current.get_untracked());
        if n > 0 {
            self.current.set(if back { (at + n - 1) % n } else { (at + 1) % n });
        }
    }
}

/// The bar while it is open, standing at the top right of what holds it.
pub(crate) fn bar(s: &Search) -> impl View + use<> {
    let shown = {
        let s = s.clone();
        move || face(&s)
    };
    let open = s.open;
    show(move || open.get(), shown).container(row().absolute().top(6.0).right(16.0))
}

fn face(s: &Search) -> Div {
    let switch = |label: &'static str, name: &'static str, on: Signal<bool>| {
        let flip = move || on.set(!on.get_untracked());
        button(label).variant(Variant::Ghost).size(Size::Sm).toggled(on).label(name).on_click(flip)
    };
    let (count, current, words) = (s.count, s.current, s.text);
    let said = memo(move || match (count.get(), words.with(String::is_empty)) {
        (_, true) => String::new(),
        (0, false) => "No results".to_string(),
        (n, false) => format!("{} of {n}", current.get().min(n - 1) + 1),
    });
    let said = text(move || said.get()).size(size::TEXT_SM.size).color(token::TEXT_3);
    let (next, back, close) = (s.clone(), s.clone(), s.open);
    let submit = s.clone();
    let query =
        input(s.text).width(200.0).placeholder("Find").label("Find").on_submit(move |_| submit.step(false));
    let keys = {
        let s = s.clone();
        move |e: &mut KeyEvent| match e.key {
            Key::Escape => {
                s.open.set(false);
                e.stop();
            }
            Key::F3 => {
                s.step(e.mods.contains(Mods::SHIFT));
                e.stop();
            }
            // Enter is the next match (the field's submit); with Shift, the one before.
            Key::Enter if e.mods.contains(Mods::SHIFT) => {
                s.step(true);
                e.stop();
            }
            _ => {}
        }
    };
    let first = row().align_items(Align::Center).gap(4.0).child(query);
    let first = first.child(switch("Aa", "Match case", s.case)).child(switch("W", "Whole word", s.word));
    let first = first.child(said);
    let first = first.child(icon_button(icons::ARROW_UP, "Previous match").on_click(move || back.step(true)));
    let first = first.child(icon_button(icons::ARROW_DOWN, "Next match").on_click(move || next.step(false)));
    let first = first.child(icon_button(icons::X, "Close").on_click(move || close.set(false)));
    let plane = column().padding(4.0).bg(token::OVERLAY_BG).border(1.0).border_color(token::LINE_STRONG);
    let plane = plane.radius(token::RADIUS_MD).elevation(1).role(Role::Group).label("Find in the document");
    plane
        .node_ref(&s.bar)
        .on_mount(|tree, id| tree.after_layout(move |tree| _ = tree.focus_first(id)))
        .on_key_down(keys)
        .child(first)
}
