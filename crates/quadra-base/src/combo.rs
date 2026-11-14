//! The combobox (specs/17, "The components"): a field whose text picks from options shown
//! under it (`pick`). Typing opens the list on the best match, unless the text is the option
//! just taken; the focus stays in the field, the arrows open the list and go round the
//! matches, Enter or a click takes one and writes it to the value; Escape or a press
//! elsewhere closes the list. The look gives the field, the panel of the rows and its place.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_layout::Display;
use quadra_reactive::{Signal, effect, memo, signal};
use quadra_tree::NodeId;
use quadra_ui::{AnyView, Div, MountCx, NodeRef, Place, View, column, div, overlay, show};

use crate::action::{Cancel, LIST, MoveNext, MovePrevious, POPUP};
use crate::pick::{Found, Parts, best};

type Field = Box<dyn FnOnce(Rc<dyn Fn()>) -> AnyView>;
type Panel = Rc<dyn Fn(&Found, AnyView) -> Div>;

pub struct Combo {
    options: Rc<Vec<String>>,
    value: Signal<String>,
    parts: Parts,
    field: Field,
    panel: Panel,
    place: Rc<dyn Fn(&NodeRef) -> Place>,
    pick: Option<Rc<dyn Fn(usize)>>,
}

/// A combobox of `value` over `options`. `field` builds the look's field of `value` and is
/// given what Enter in it does; `panel` puts the rows in the look's plane, which stands at
/// `place` against the field.
pub fn combo<V: View + 'static>(
    options: Rc<Vec<String>>,
    value: Signal<String>,
    parts: Parts,
    field: impl FnOnce(Rc<dyn Fn()>) -> V + 'static,
    panel: impl Fn(&Found, AnyView) -> Div + 'static,
    place: impl Fn(&NodeRef) -> Place + 'static,
) -> Combo {
    let field: Field = Box::new(move |submit| AnyView::new(field(submit)));
    Combo { options, value, parts, field, panel: Rc::new(panel), place: Rc::new(place), pick: None }
}

impl Combo {
    /// Runs `f` with the index of an option when it is taken.
    pub fn on_pick(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.pick = Some(Rc::new(f));
        self
    }
}

impl View for Combo {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (options, value, pick) = (self.options, self.value, self.pick);
        let (open, around) = (signal(false), NodeRef::new());
        let known = options.clone();
        let found = Found::new(memo(move || best(&value.get(), known.iter().map(String::as_str))));
        let (on, matches) = (found.on, found.matches);
        // Typing opens the list on the best match; taking an option is not typing.
        let taken: Rc<RefCell<Option<String>>> = Rc::default();
        let last = taken.clone();
        effect(move || {
            let text = value.get();
            on.set(0);
            open.set(!text.is_empty() && last.borrow().as_ref() != Some(&text));
        });
        let choose = {
            let (handle, field) = (cx.handle.clone(), around.clone());
            move |k: usize| {
                let Some(&i) = matches.get().get(k) else { return };
                *taken.borrow_mut() = Some(options[i].clone());
                value.set(options[i].clone());
                open.set(false);
                if let Some(pick) = &pick {
                    pick(i);
                }
                // The focus is in the field again after a click on an option.
                if let Some(around) = field.get() {
                    handle.with(move |tree| {
                        tree.focus_first(around);
                    });
                }
            }
        };
        let (parts, panel, place, anchor) = (self.parts, self.panel, self.place, around.clone());
        let (shown_found, clicked) = (found.clone(), choose.clone());
        let list = move || {
            let rows = AnyView::new(shown_found.rows(parts.clone(), |_| 0, clicked.clone()));
            overlay(place(&anchor), panel(&shown_found, rows))
                .opener(&anchor)
                .on_outside_press(move || open.set(false))
        };
        // The arrows open the list and go round the matches; Enter takes the highlighted one.
        let step = move |forward: bool| {
            let count = matches.get().len();
            if count > 0 && open.get_untracked() {
                found.step(forward);
            }
            open.set(count > 0);
        };
        let (down, up) = (step.clone(), step);
        let submit: Rc<dyn Fn()> = Rc::new(move || {
            if open.get_untracked() {
                choose(on.get_untracked());
            }
        });
        let shown = show(move || open.get() && !matches.get().is_empty(), list);
        let keys = div().key_context(LIST).on_action::<MoveNext>(move |_| down(true));
        let keys = keys.on_action::<MovePrevious>(move |_| up(false)).child((self.field)(submit));
        let whole = column().shrink(0.0).node_ref(&around).key_context(POPUP);
        let whole = whole.on_action::<Cancel>(move |_| open.set(false));
        whole.child(keys).child(shown.container(div().display(Display::None))).mount(cx)
    }
}
