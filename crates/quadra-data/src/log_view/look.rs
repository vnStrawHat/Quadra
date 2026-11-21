//! How the log view's rows look: the time, the size, and the colours of a line.

use quadra_design::token;
use quadra_prim::Color;
use quadra_tree::{Paint, Token};

use crate::ansi::{Ink, Pen};
use crate::log::{Level, clock};

/// How the rows look: whether the time shows, its colour and its offset from UTC in ms,
/// the size of the text and the row, and whether a line takes its level's colour.
#[derive(Clone, Copy)]
pub(super) struct Look {
    pub(super) timestamps: bool,
    pub(super) time: Token,
    pub(super) offset: i64,
    pub(super) size: f32,
    pub(super) row: f32,
    pub(super) levels: bool,
    /// The time to the millisecond, or to the second.
    pub(super) millis: bool,
    /// On the ground: the time at its edge, a line's text two letters after it.
    pub(super) ground: bool,
}

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
        (pad, if self.timestamps { letters * letter + gap } else { 0.0 })
    }
}

/// The colour of a line of a level.
pub(super) fn tone(level: Level) -> Token {
    match level {
        Level::Info => token::OK,
        Level::Warn => token::WARN,
        Level::Error => token::ERROR,
        Level::None | Level::Debug => token::TEXT,
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
