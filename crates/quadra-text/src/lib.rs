//! Quadra's text (specs/08-text.md): shaping by the operating system behind a trait, and the
//! layout shared by every backend.
//!
//! A text node owns a [`Paragraph`]: shaped once, broken into lines at a width without shaping
//! again. Shaped words are shared through the [`TextSystem`]'s cache.

mod fake;
mod grapheme;
mod paragraph;
mod system;
mod types;

pub use fake::FakeBackend;
pub use grapheme::{boundaries, next_grapheme, prev_grapheme};
pub use paragraph::{Line, Paragraph, PositionedGlyph};
pub use system::{TextStats, TextSystem, Word};
pub use types::{FaceId, FaceMetrics, FontKey, GlyphBitmap, ShapedGlyph, TextBackend, TextStyle};
