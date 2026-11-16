//! The terminal as the engine's prompt (specs/19, "M7 in detail"): questions are written to
//! the space, and what is typed there up to Enter is the answer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

/// What the space sends the engine.
pub(crate) enum Cmd {
    Input(Vec<u8>),
    Resize(u16, u16),
    Close,
}

pub(crate) type Out = Arc<dyn Fn(&[u8]) + Send + Sync>;

pub(crate) struct Prompt {
    out: Out,
    pub(crate) input: Mutex<UnboundedReceiver<Cmd>>,
    /// Rows in the high half, columns in the low.
    size: AtomicU32,
}

impl Prompt {
    pub(crate) fn new(out: Out, input: UnboundedReceiver<Cmd>, rows: u16, cols: u16) -> Prompt {
        Prompt { out, input: Mutex::new(input), size: AtomicU32::new(pack(rows, cols)) }
    }

    /// Writes to the space; a line feed is a new line.
    pub(crate) fn say(&self, text: &str) {
        (self.out)(text.replace("\r\n", "\n").replace('\n', "\r\n").as_bytes());
    }

    pub(crate) fn write(&self, bytes: &[u8]) {
        (self.out)(bytes);
    }

    pub(crate) fn resized(&self, rows: u16, cols: u16) {
        self.size.store(pack(rows, cols), Ordering::Relaxed);
    }

    /// Rows and columns.
    pub(crate) fn size(&self) -> (u16, u16) {
        let both = self.size.load(Ordering::Relaxed);
        ((both >> 16) as u16, both as u16)
    }

    /// Asks a question; the line typed, or None when it was given up (Ctrl+C, Ctrl+D, the
    /// space closed). `echo` false: what is typed is not shown (a password).
    pub(crate) async fn ask(&self, question: &str, echo: bool) -> Option<String> {
        self.say(question);
        let mut line = Line::default();
        let mut input = self.input.lock().await;
        loop {
            match input.recv().await? {
                Cmd::Input(bytes) => {
                    for &byte in &bytes {
                        match line.take(byte) {
                            Took::Done => {
                                self.say("\n");
                                return Some(String::from_utf8_lossy(&line.text).into_owned());
                            }
                            Took::GaveUp => {
                                self.say("^C\n");
                                return None;
                            }
                            Took::Typed if echo => self.write(&[byte]),
                            Took::Erased if echo => self.write(b"\x08 \x08"),
                            _ => {}
                        }
                    }
                }
                Cmd::Resize(rows, cols) => self.resized(rows, cols),
                Cmd::Close => return None,
            }
        }
    }
}

fn pack(rows: u16, cols: u16) -> u32 {
    u32::from(rows) << 16 | u32::from(cols)
}

/// A line being typed: its bytes, and whether an escape sequence is being skipped.
#[derive(Default)]
struct Line {
    text: Vec<u8>,
    escape: bool,
}

#[derive(Debug, PartialEq)]
enum Took {
    Typed,
    Erased,
    Done,
    GaveUp,
    Ignored,
}

impl Line {
    fn take(&mut self, byte: u8) -> Took {
        if self.escape {
            // An arrow or the like: skipped up to its final byte.
            self.escape = !(byte.is_ascii_alphabetic() || byte == b'~');
            return Took::Ignored;
        }
        match byte {
            b'\r' | b'\n' => Took::Done,
            3 | 4 => Took::GaveUp,
            0x1b => {
                self.escape = true;
                Took::Ignored
            }
            0x7f | 8 => {
                // A whole character: its continuation bytes, then its first.
                while self.text.pop().is_some_and(|b| b & 0xc0 == 0x80) {}
                Took::Erased
            }
            0..0x20 => Took::Ignored,
            _ => {
                self.text.push(byte);
                Took::Typed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(bytes: &[u8]) -> (String, Took) {
        let mut line = Line::default();
        let mut last = Took::Ignored;
        for &b in bytes {
            last = line.take(b);
            if matches!(last, Took::Done | Took::GaveUp) {
                break;
            }
        }
        (String::from_utf8_lossy(&line.text).into_owned(), last)
    }

    #[test]
    fn a_line_is_edited_as_typed() {
        assert_eq!(typed(b"yes\r"), ("yes".into(), Took::Done));
        assert_eq!(typed(b"yex\x7fs\r"), ("yes".into(), Took::Done));
        assert_eq!(typed("mật\x7f\r".as_bytes()), ("mậ".into(), Took::Done));
        assert_eq!(typed(b"a\x1b[Db\r"), ("ab".into(), Took::Done));
        assert_eq!(typed(b"pw\x03"), ("pw".into(), Took::GaveUp));
        assert_eq!(typed(b"\x7f\x7fok\n"), ("ok".into(), Took::Done));
    }
}
