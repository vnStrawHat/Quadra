//! Quadra's text (specs/08-text.md): shaping by the operating system behind a trait, and the
//! layout shared by every backend.
//!
//! A text node owns a [`Paragraph`]: shaped once, broken into lines at a width without shaping
//! again. Shaped words are shared through the [`TextSystem`]'s cache.

mod edit;
mod fake;
mod grapheme;
mod nfc_table;
mod normalize;
mod numeric;
mod paragraph;
mod places;
mod system;
mod types;

pub use edit::{EditBuffer, Motion};
pub use fake::FakeBackend;
pub use grapheme::{boundaries, next_grapheme, prev_grapheme};
pub use normalize::{fold_diacritics, is_mark, normalize_nfc};
pub use paragraph::{Line, Paragraph, PositionedGlyph};
pub use system::{TextStats, TextSystem, Word};
pub use types::{
    FaceId, FaceMetrics, FontKey, GlyphBitmap, GlyphFormat, ShapedGlyph, Shaping, TextBackend, TextStyle,
};
