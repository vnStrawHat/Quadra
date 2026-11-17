//! The dialog (specs/17, "The components"): a question the user answers before going on, in a
//! popup (`popup`) under the top of the window or along an edge of it. Its cross, Escape and a
//! press outside give it up; an answer does what it does and closes it. The look gives the
//! page, the cross and the buttons of the answers.

use std::rc::Rc;

use quadra_reactive::Signal;
use quadra_tree::Role;
use quadra_view::Div;

/// The page of a dialog on its face: a dialog to a reader, named by its title.
pub fn page(face: Div, title: impl Into<String>) -> Div {
    face.role(Role::Dialog).label(title.into())
}

/// The cross of a dialog on its face: a button named "Close" that gives the dialog up.
pub fn close(face: Div, open: Signal<bool>) -> Div {
    face.role(Role::Button).label("Close").on_click(move |_| open.set(false))
}

/// What an answer does: `act`, then the dialog closes (unless `act` closed what it was of).
pub fn answer(open: Signal<bool>, act: Rc<dyn Fn()>) -> impl Fn() + 'static {
    move || {
        act();
        open.try_set(false);
    }
}
