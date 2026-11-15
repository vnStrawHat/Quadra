//! A session's log (specs/18, "Logging"): the text a person would have seen, a line at a time
//! with a stamp the app gives, written on the thread that feeds the session.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use oneterm_vt::parser::{Dispatch, Parser};

use crate::session::Session;

/// What a feeder writes the output to, while a log is open.
pub(crate) type Slot = Arc<Mutex<Option<Log>>>;

pub(crate) struct Log {
    writer: BufWriter<File>,
    parser: Parser,
    lines: Lines,
    stamp: Box<dyn Fn() -> String + Send>,
}

/// Printable runs in, sequences dropped; a line ends at CR or LF, backspace takes one back.
#[derive(Default)]
struct Lines {
    current: String,
    done: Vec<String>,
}

impl Dispatch for Lines {
    fn print_str(&mut self, text: &str) {
        self.current.extend(text.chars().filter(|c| !c.is_control()));
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'\r' => self.end(),
            0x08 => _ = self.current.pop(),
            _ => {}
        }
    }
}

impl Lines {
    fn end(&mut self) {
        if !self.current.is_empty() {
            self.done.push(std::mem::take(&mut self.current));
        }
    }
}

impl Log {
    /// Output fed: its finished lines written. A failure closes the log.
    pub(crate) fn write(slot: &Slot, bytes: &[u8]) {
        let mut open = slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(log) = open.as_mut() {
            log.parser.advance(&mut log.lines, bytes);
            if log.flush().is_err() {
                *open = None;
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for line in self.lines.done.drain(..) {
            writeln!(self.writer, "[{}] {line}", (self.stamp)())?;
        }
        self.writer.flush()
    }
}

impl Session {
    /// Logs the output from now on to `path` (its folder made), after what it holds or over
    /// it; each line is told with `stamp()`. A log open before is closed first.
    pub fn start_log(
        &self,
        path: &Path,
        append: bool,
        stamp: impl Fn() -> String + Send + 'static,
    ) -> std::io::Result<()> {
        self.stop_log();
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let file = OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(path)?;
        let log = Log {
            writer: BufWriter::new(file),
            parser: Parser::new(),
            lines: Lines::default(),
            stamp: Box::new(stamp),
        };
        *self.0.feeder.log.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(log);
        Ok(())
    }

    /// Closes the log, with the line not ended yet.
    pub fn stop_log(&self) {
        let gone = self.0.feeder.log.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        if let Some(mut log) = gone {
            log.lines.end();
            _ = log.flush();
        }
    }

    /// Whether a log is open: false again after a write failed.
    pub fn logging(&self) -> bool {
        self.0.feeder.log.lock().is_ok_and(|it| it.is_some())
    }
}
