//! How the log view's rows look: the time, the size, and the colours of a line.

use quadra_design::token;
use quadra_prim::Color;
use quadra_reactive::Signal;
use quadra_tree::{Paint, Token};

use crate::ansi::{Ink, Pen};
use crate::log::{Level, Log, clock};

/// How the rows look: whether the time shows, its colour and its offset from UTC in ms,
/// the size of the text and the row, and whether a line takes its level's colour.
#[derive(Clone, Copy)]
pub(super) struct Look {
    pub(super) timestamps: Signal<bool>,
    pub(super) time: Token,
    pub(super) offset: i64,
    pub(super) size: f32,
    pub(super) row: f32,
    pub(super) levels: bool,
    /// The time to the millisecond, or to the second.
    pub(super) millis: bool,
    /// On the ground: the time at its edge, a line's text two letters after it.
    pub(super) ground: bool,
    /// A long line goes on to the rows under it.
    pub(super) wrap: Signal<bool>,
}

/// The room of a source's dot, and of the level column in letters.
pub(super) const DOT: f32 = 16.0;
pub(super) const COLUMN: f32 = 7.0;

impl Look {
    /// The time of `ms` as a line shows it.
    pub(super) fn when(self, ms: i64) -> String {
        let said = clock(ms + self.offset);
        if self.millis { said } else { said[..8].to_string() }
    }

    /// Before a line's text: the row's own room, and the time with the gap after it.
    pub(super) fn lead(self, letter: f32) -> (f32, f32) {
        let pad = if self.ground { 0.0 } else { 8.0 };
        let letters = if self.millis { 12.0 } else { 8.0 };
        let gap = if self.ground { 2.0 * letter } else { 12.0 };
        (pad, if self.timestamps.get() { letters * letter + gap } else { 0.0 })
    }

    /// Whether a line has a source's dot and a level column.
    pub(super) fn parts(self, log: &Log) -> (bool, bool) {
        (log.sources() > 1, self.levels && log.has_levels())
    }

    /// All that is before a line's text.
    pub(super) fn before(self, log: &Log, letter: f32) -> f32 {
        let ((pad, time), (dot, column)) = (self.lead(letter), self.parts(log));
        pad + time + if dot { DOT } else { 0.0 } + if column { COLUMN * letter } else { 0.0 }
    }
}

/// The colour of a line of a level: only warn, error and fatal colour what it says.
pub(super) fn tone(level: Level) -> Token {
    match level {
        Level::Warn => token::WARN,
        Level::Error | Level::Fatal => token::ERROR,
        Level::Debug => token::TEXT_2,
        Level::None | Level::Info => token::TEXT,
    }
}

/// The colour of a level's word in the level column.
pub(super) fn word_tone(level: Level) -> Token {
    match level {
        Level::Info => token::OK,
        Level::Warn => token::WARN,
        Level::Error | Level::Fatal => token::ERROR,
        Level::None | Level::Debug => token::TEXT_3,
    }
}

/// The colour a pen writes in; `fade` is how much of it a faint line keeps.
pub(super) fn paint(pen: Pen, fade: f32, base: Token) -> Paint {
    let fade = if pen.faint { fade * 0.6 } else { fade };
    match pen.ink {
        Ink::Text => base.alpha(fade),
        Ink::Palette(n) => token::ANSI[n as usize % 16].alpha(fade),
        Ink::Rgb(r, g, b) => Paint::Color(Color::rgb(u32::from_be_bytes([0, r, g, b])).alpha(fade)),
    }
}
