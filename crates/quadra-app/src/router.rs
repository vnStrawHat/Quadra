//! Pages (specs/15, "Pages"): a stack of routes the app goes back and forth along, and the
//! view that shows the current one.

use quadra_platform::MouseButton;
use quadra_reactive::{Signal, memo, signal, untrack};
use quadra_tree::{GoBack, GoForward, PointerKind};
use quadra_ui::{Div, View, div, dynamic};

/// Where the app has been, and where it is: `R` is any value of the app that names a page.
/// A handle: copies are the same router.
pub struct Router<R: 'static> {
    stack: Signal<Vec<R>>,
    at: Signal<usize>,
}

impl<R> Clone for Router<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for Router<R> {}

/// A router at its first page.
pub fn router<R: Clone + PartialEq + 'static>(first: R) -> Router<R> {
    Router { stack: signal(vec![first]), at: signal(0) }
}

impl<R: Clone + PartialEq + 'static> Router<R> {
    /// The page shown; read as a signal is, so what reads it follows it.
    pub fn current(&self) -> R {
        let at = self.at.get();
        self.stack.with(|stack| stack[at].clone())
    }

    /// Goes to `route`; the pages ahead of the current one are forgotten. Going to the page
    /// shown does nothing.
    pub fn push(&self, route: R) {
        let at = self.at.get_untracked();
        if self.stack.with_untracked(|stack| stack[at] == route) {
            return;
        }
        self.stack.update(|stack| {
            stack.truncate(at + 1);
            stack.push(route);
        });
        self.at.set(at + 1);
    }

    /// Shows `route` in place of the current page: going back does not come to the old one.
    pub fn replace(&self, route: R) {
        let at = self.at.get_untracked();
        self.stack.update(|stack| stack[at] = route);
    }

    pub fn can_back(&self) -> bool {
        self.at.get() > 0
    }

    pub fn can_forward(&self) -> bool {
        self.at.get() + 1 < self.stack.with(Vec::len)
    }

    pub fn back(&self) {
        if untrack(|| self.can_back()) {
            self.at.update(|at| *at -= 1);
        }
    }

    pub fn forward(&self) {
        if untrack(|| self.can_forward()) {
            self.at.update(|at| *at += 1);
        }
    }

    /// The current page, made by `page`, made again only when the page changes; Alt-Left and
    /// Alt-Right ([`GoBack`], [`GoForward`]) and the back and forward buttons of a mouse go
    /// back and forth while the focus or the pointer is in it.
    pub fn view<V: View + 'static>(self, page: impl Fn(R) -> V + 'static) -> Div {
        let current = memo(move || self.current());
        let shown = dynamic(move || {
            let route = current.get();
            untrack(|| page(route))
        });
        let view = div().grow(1.0).child(shown);
        let view = view.on_action::<GoBack>(move |_| self.back());
        let view = view.on_action::<GoForward>(move |_| self.forward());
        view.on_pointer(PointerKind::Down, move |e| match e.button {
            MouseButton::Back => self.back(),
            MouseButton::Forward => self.forward(),
            _ => {}
        })
    }
}
