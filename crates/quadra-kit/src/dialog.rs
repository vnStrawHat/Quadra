//! The dialog (specs/behaviour/15, "Dialog"): a question the user answers before going on.
//! It hangs 80 px from the top of the window, centred, with no backdrop: Escape or a press
//! outside gives it up.

use std::rc::Rc;

use quadra_layout::Justify;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Role};
use quadra_ui::{MountCx, Place, View, column, div, row, text};

use crate::button::{Size, Variant, button};
use crate::popup::popup;
use crate::{size, token};

type Act = Rc<dyn Fn()>;

pub struct Dialog<F> {
    open: Signal<bool>,
    title: String,
    body: F,
    actions: Vec<(String, Variant, Act)>,
}

/// A dialog shown while `open` is true: a title over what `body` builds, each time it opens.
pub fn dialog<V, F>(open: Signal<bool>, title: impl Into<String>, body: F) -> Dialog<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    Dialog { open, title: title.into(), body, actions: Vec::new() }
}

impl<F> Dialog<F> {
    /// A button at the foot of the dialog, from the left to the right in the order given.
    /// Pressing it runs `f` and closes the dialog.
    pub fn action(mut self, label: impl Into<String>, variant: Variant, f: impl Fn() + 'static) -> Self {
        self.actions.push((label.into(), variant, Rc::new(f)));
        self
    }
}

/// A question with two answers: `yes` does `on_yes`, Cancel does nothing, and both close the
/// dialog.
pub fn confirm(
    open: Signal<bool>,
    title: impl Into<String>,
    message: impl Into<String>,
    yes: impl Into<String>,
    on_yes: impl Fn() + 'static,
) -> impl View {
    let message = message.into();
    let ask = dialog(open, title, move || text(message.clone()));
    ask.action("Cancel", Variant::Secondary, || {}).action(yes, Variant::Primary, on_yes)
}

impl<V, F> View for Dialog<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (open, title, body, actions) = (self.open, self.title, self.body, Rc::new(self.actions));
        let content = move || {
            let headline = text(title.clone()).size(size::TEXT_LG.size).weight(size::WEIGHT_STRONG);
            let head = div().padding_x(12.0).padding_y(8.0).child(headline);
            let middle = div().padding_x(12.0).padding_y(4.0).text_color(token::TEXT_2).child(body());
            let mut foot = row().gap(4.0).padding(8.0).justify(Justify::End);
            foot = foot.border_top(1.0).border_color(token::LINE);
            for (label, variant, act) in actions.iter() {
                let act = act.clone();
                let press = move || {
                    act();
                    open.set(false);
                };
                foot = foot.child(button(label.clone()).variant(*variant).size(Size::Sm).on_click(press));
            }
            let page = column().width(440.0).role(Role::Dialog).label(title.clone());
            page.child(head).child(middle).child(div().height(8.0)).child(foot)
        };
        popup(open, (Place::Top(80.0), 2), None, content).mount(cx)
    }
}
