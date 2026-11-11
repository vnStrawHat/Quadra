//! The clipboard, as plain text. The headless platform's is a variable in memory: no test
//! ever touches the real one.

use std::cell::RefCell;

pub trait Clipboard {
    /// The text on the clipboard, if it holds text.
    fn read_text(&self) -> Option<String>;
    fn write_text(&self, text: &str);
}

/// A clipboard that is a variable: what the headless platform gives.
#[derive(Default)]
pub struct MemoryClipboard(RefCell<Option<String>>);

impl Clipboard for MemoryClipboard {
    fn read_text(&self) -> Option<String> {
        self.0.borrow().clone()
    }

    fn write_text(&self, text: &str) {
        *self.0.borrow_mut() = Some(text.to_string());
    }
}
