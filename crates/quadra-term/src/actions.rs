//! What the app's menu of a terminal does (specs/18, "Input"), and how the view's pointer
//! and wheel behave as the app's settings say.

use std::time::Instant;

use crate::session::Session;

/// How a terminal view takes the pointer and the wheel. The default is what a view does
/// without settings: no copy on select, no paste by the middle button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TermSettings {
    /// A selection is copied to the clipboard when the button is released.
    pub copy_on_select: bool,
    /// The middle button pastes, unless the program takes the press.
    pub middle_paste: bool,
    /// How many times the rows of the wheel it scrolls.
    pub scroll: f32,
    /// On the alternate screen the wheel sends arrows, even when the program did not ask.
    pub alternate_scroll: bool,
}

impl Default for TermSettings {
    fn default() -> TermSettings {
        TermSettings { copy_on_select: false, middle_paste: false, scroll: 1.0, alternate_scroll: false }
    }
}

impl Session {
    /// The text selected, if any.
    pub fn selection_text(&self) -> Option<String> {
        self.engine().term.selection_text()
    }

    /// Pastes text into the program, bracketed when it asked, and shows the bottom again.
    pub fn paste(&self, text: &str) {
        let bytes = {
            let mut engine = self.engine();
            engine.term.scroll_viewport(i32::MAX);
            engine.term.encode_paste(text)
        };
        self.repaint();
        self.write(&bytes);
    }

    /// Selects everything, the history with the screen.
    pub fn select_all(&self) {
        self.engine().term.select_all();
        self.repaint();
    }

    /// Forgets the rows above the screen, as a program would with ED 3.
    pub fn clear_history(&self) {
        self.feeder().feed(b"\x1b[3J", Instant::now());
    }

    /// Paints the view of the session again, if one shows it.
    fn repaint(&self) {
        let handle = self.0.handle.borrow().clone();
        if let (Some(id), Some(handle)) = (self.0.view.get(), handle) {
            handle.with(move |tree| crate::widget::repaint(tree, id));
        }
    }
}
