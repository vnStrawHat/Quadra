//! The text size the user asks for (specs/12, "Text size"): the whole UI zoomed, or text
//! alone larger, and which of the two the system's text size makes larger.

use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, use_context};
use quadra_render::Renderer;
use std::rc::Rc;

use quadra_tree::{Keymap, Tree};

use crate::App;

/// How much larger the user wants the UI: `zoom` the whole of it, `text` only its text; and
/// which of them the system's text size multiplies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSize {
    pub zoom: f32,
    pub text: f32,
    pub system: SystemText,
}

/// What the system's text size makes larger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemText {
    Zoom,
    Text,
    None,
}

impl Default for TextSize {
    fn default() -> TextSize {
        TextSize { zoom: 1.0, text: 1.0, system: SystemText::Zoom }
    }
}

impl TextSize {
    /// The zoom and the factor of text, with the system's text size, each held between 0.5
    /// and 4.
    pub fn factors(&self, system: f32) -> (f32, f32) {
        let (zoom, text) = match self.system {
            SystemText::Zoom => (self.zoom * system, self.text),
            SystemText::Text => (self.zoom, self.text * system),
            SystemText::None => (self.zoom, self.text),
        };
        (zoom.clamp(0.5, 4.0), text.clamp(0.5, 4.0))
    }
}

/// The app's keymap, from the context of a view: a page of settings puts another, and every
/// window follows.
pub fn app_keymap() -> Signal<Rc<Keymap>> {
    use_context::<Signal<Rc<Keymap>>>().expect("app_keymap() is called inside an app")
}

/// The app's text size, from the context of a view: a settings page writes it.
pub fn text_size() -> Signal<TextSize> {
    use_context::<Signal<TextSize>>().expect("text_size() is called inside an app")
}

impl<R: Renderer> App<R> {
    /// The text size the app starts with; a view changes it by writing `text_size()`.
    pub fn text_size(self, size: TextSize) -> App<R> {
        self.root.run(text_size).set(size);
        self
    }

    /// What a window follows of the app: its theme, its text size and its keymap, now and
    /// whenever they change.
    pub(crate) fn follow(&self, tree: &mut Tree, scope: &Scope) {
        let (theme, size, appearance, handle) =
            (self.theme, self.root.run(text_size), self.appearance, tree.handle());
        let factors = move || size.get().factors(appearance.get().text_scale);
        let keymap = self.keymap;
        tree.set_theme(theme.get_untracked());
        tree.set_keymap(keymap.get_untracked());
        let (zoom, text) = scope.run(|| quadra_reactive::untrack(factors));
        tree.set_zoom(zoom);
        tree.set_text_scale(text);
        scope.run(|| {
            let writer = handle.clone();
            handle.bind(move || {
                let theme = theme.get();
                writer.with(move |tree| tree.set_theme(theme));
            });
            let writer = handle.clone();
            handle.bind(move || {
                let keymap = keymap.get();
                writer.with(move |tree| tree.set_keymap(keymap));
            });
            let writer = handle.clone();
            handle.bind(move || {
                let (zoom, text) = factors();
                writer.with(move |tree| {
                    tree.set_zoom(zoom);
                    tree.set_text_scale(text);
                });
            });
        });
    }
}

/// A point in the window's logical pixels, in the tree's.
pub(crate) fn inward(tree: &Tree, at: Point) -> Point {
    Point::new(at.x / tree.zoom(), at.y / tree.zoom())
}
