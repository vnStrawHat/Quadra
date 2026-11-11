//! The combobox (specs/behaviour/09, "Forms"): an input whose text filters a few options
//! shown under it. The focus stays in the input: the arrows move a highlight over the
//! options, and Enter or a click takes one.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_layout::{Align, Length};
use quadra_reactive::{Signal, effect, memo, signal};
use quadra_tree::{Along, NodeId, Role, Side, State};
use quadra_ui::{MountCx, NodeRef, Place, View, column, div, dynamic, overlay, row, scroll, show};

use crate::action::{Cancel, LIST, MoveNext, MovePrevious, POPUP};
use crate::input::input;
use crate::palette::{best, fuzzy_hits, marked};
use crate::popup::surface;
use crate::{icons, token};

/// The height of an option, and the most of the list that shows.
const ROW: f32 = 23.0;
const MOST: f32 = 240.0;

type Pick = Rc<dyn Fn(usize)>;

pub struct Combobox {
    options: Rc<Vec<String>>,
    value: Signal<String>,
    placeholder: String,
    pick: Option<Pick>,
}

/// An input of `value`, 280 px wide, that offers the `options` its text matches.
pub fn combobox<S: Into<String>>(options: impl IntoIterator<Item = S>, value: Signal<String>) -> Combobox {
    let options = Rc::new(options.into_iter().map(Into::into).collect());
    Combobox { options, value, placeholder: String::new(), pick: None }
}

impl Combobox {
    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.placeholder = words.into();
        self
    }

    /// Runs `f` with the index of an option when it is taken.
    pub fn on_pick(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.pick = Some(Rc::new(f));
        self
    }
}

impl View for Combobox {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (options, value, pick) = (self.options, self.value, self.pick);
        let (open, on, around) = (signal(false), signal(0usize), NodeRef::new());
        let known = options.clone();
        let found = memo(move || best(&value.get(), known.iter().map(String::as_str)));
        // Typing opens the list on the best match; taking an option is not typing.
        let taken: Rc<RefCell<Option<String>>> = Rc::default();
        let last = taken.clone();
        effect(move || {
            let text = value.get();
            on.set(0);
            open.set(!text.is_empty() && last.borrow().as_ref() != Some(&text));
        });
        let choose = {
            let (options, handle, field) = (options.clone(), cx.handle.clone(), around.clone());
            move |k: usize| {
                let Some(&i) = found.get().get(k) else { return };
                *taken.borrow_mut() = Some(options[i].clone());
                value.set(options[i].clone());
                open.set(false);
                if let Some(pick) = &pick {
                    pick(i);
                }
                // The focus is in the input again after a click on an option.
                if let Some(around) = field.get() {
                    handle.with(move |tree| {
                        tree.focus_first(around);
                    });
                }
            }
        };
        let refs: Rc<RefCell<Vec<NodeRef>>> = Rc::default();
        let (lines, clicked, anchor) = (refs.clone(), choose.clone(), around.clone());
        let list = move || {
            let (lines, clicked, options) = (lines.clone(), clicked.clone(), options.clone());
            let rows = dynamic(move || {
                let (matches, query) = (found.get(), value.get_untracked());
                *lines.borrow_mut() = matches.iter().map(|_| NodeRef::new()).collect();
                let mut all = column().grow(1.0).padding_y(4.0).role(Role::List);
                for (k, &i) in matches.iter().enumerate() {
                    let (hits, clicked) =
                        (fuzzy_hits(&query, &options[i]).unwrap_or_default(), clicked.clone());
                    let line =
                        row().align_items(Align::Center).grow(1.0).height(ROW).padding_x(6.0).radius(4.0);
                    let line = line.role(Role::ListItem).label(options[i].clone());
                    let line = line.selected(move || on.get() == k).node_ref(&lines.borrow()[k]);
                    let line = line
                        .hover(|s| s.bg(token::PANEL_HOVER))
                        .when(State::SELECTED, |s| s.bg(token::PRESS));
                    let line = line.on_click(move |_| clicked(k)).child(marked(&options[i], &hits));
                    all = all.child(div().padding_x(4.0).child(line));
                }
                all
            });
            let tall = move || Length::Px((found.get().len() as f32 * ROW + 8.0).min(MOST));
            let panel =
                surface(1).width(280.0).child(scroll(signal(0.0), rows).container(div().height(tall)));
            let place = Place::Beside(anchor.clone(), Side::Below, Along::Start, 4.0);
            overlay(place, panel).opener(&anchor).on_outside_press(move || open.set(false))
        };
        // The arrows open the list and go round the matches; Enter takes the highlighted one.
        let step = move |forward: bool| {
            let count = found.get().len();
            if count > 0 && open.get_untracked() {
                on.update(|at| *at = if forward { (*at + 1) % count } else { (*at + count - 1) % count });
                refs.borrow()[on.get_untracked()].scroll_into_view();
            }
            open.set(count > 0);
        };
        let (down, up, submit) = (step.clone(), step, choose);
        let field = input(value).placeholder(&self.placeholder).icon(icons::SEARCH).width(280.0);
        let field = field.on_submit(move |_| {
            if open.get_untracked() {
                submit(on.get_untracked());
            }
        });
        let shown = show(move || open.get() && !found.get().is_empty(), list);
        let keys = div().key_context(LIST).on_action::<MoveNext>(move |_| down(true));
        let keys = keys.on_action::<MovePrevious>(move |_| up(false)).child(field);
        let whole = column().shrink(0.0).node_ref(&around).key_context(POPUP);
        let whole = whole.on_action::<Cancel>(move |_| open.set(false));
        whole.child(keys).child(shown.container(div().display(quadra_layout::Display::None))).mount(cx)
    }
}
