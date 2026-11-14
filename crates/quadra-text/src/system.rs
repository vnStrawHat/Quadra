//! The text system: faces, and the shaping cache shared by every paragraph (specs/08,
//! "Shaping cache").

use std::collections::HashMap;
use std::rc::Rc;

use quadra_prim::Rect;

use crate::numeric::Numeric;
use crate::types::{FaceId, Feature, FontKey, GlyphBitmap, ShapedGlyph, TextBackend, TextStyle};

/// A shaped token: a word, or a run of one separator.
#[derive(Debug, Default, PartialEq)]
pub struct Word {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
}

/// What the text system did since the last [`TextSystem::take_stats`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextStats {
    /// Words sent to the backend.
    pub words_shaped: u32,
    /// Words found in the cache.
    pub word_hits: u32,
    /// Numbers composed from glyphs shaped once.
    pub words_composed: u32,
    /// Paragraphs whose lines were broken.
    pub paragraphs_broken: u32,
}

type Words = HashMap<(FaceId, u32), HashMap<Box<str>, Rc<Word>>>;

/// Shaped words by text, face and size, in two generations: a word used again moves to the
/// current one, and when that is half the limit the previous one is dropped. So the cache is
/// bounded, a lookup is one hash, and nothing is tied to frames: a paragraph holds its words
/// itself.
struct WordCache {
    current: Words,
    previous: Words,
    count: usize,
    limit: usize,
}

impl WordCache {
    fn get(&mut self, key: (FaceId, u32), text: &str) -> Option<Rc<Word>> {
        if let Some(word) = self.current.get(&key).and_then(|m| m.get(text)) {
            return Some(word.clone());
        }
        let (text, word) = self.previous.get_mut(&key)?.remove_entry(text)?;
        self.put(key, text, word.clone());
        Some(word)
    }

    fn put(&mut self, key: (FaceId, u32), text: Box<str>, word: Rc<Word>) {
        if self.count >= self.limit / 2 {
            self.previous = std::mem::take(&mut self.current);
            self.count = 0;
        }
        self.current.entry(key).or_default().insert(text, word);
        self.count += 1;
    }
}

pub struct TextSystem {
    backend: Box<dyn TextBackend>,
    default_family: Rc<str>,
    faces: HashMap<FontKey, FaceId>,
    words: WordCache,
    numeric: Numeric,
    /// Where the ink of glyphs is, by face, glyph and size.
    inks: HashMap<(FaceId, u16, u32), Rect>,
    pub(crate) stats: TextStats,
}

impl TextSystem {
    pub fn new(backend: Box<dyn TextBackend>) -> TextSystem {
        let default_family = Rc::from(backend.default_family());
        let words = WordCache { current: Words::new(), previous: Words::new(), count: 0, limit: 16_384 };
        let (faces, numeric, stats) = (HashMap::new(), Numeric::default(), TextStats::default());
        TextSystem { backend, default_family, faces, words, numeric, inks: HashMap::new(), stats }
    }

    /// How many shaped words the cache may hold (16,384 unless set).
    pub fn set_cache_limit(&mut self, words: usize) {
        self.words.limit = words.max(2);
    }

    pub fn backend(&mut self) -> &mut dyn TextBackend {
        &mut *self.backend
    }

    /// The OpenType features a family is shaped with (`Feature::on(b"tnum")`). Words shaped
    /// before are shaped again when asked for; a paragraph that holds its words keeps them
    /// until its text or style changes, so features are set as fonts are loaded.
    pub fn set_features(&mut self, family: &str, features: &[Feature]) {
        self.backend.set_features(family, features);
        self.words.current.clear();
        self.words.previous.clear();
        self.words.count = 0;
        self.numeric = Numeric::default();
    }

    /// The face of a style. An unknown family falls back to the default one at the same
    /// weight.
    pub fn face_for(&mut self, style: &TextStyle) -> FaceId {
        let family = if style.family.is_empty() { self.default_family.clone() } else { style.family.clone() };
        let key = FontKey { family, weight: style.weight, italic: style.italic };
        if let Some(&face) = self.faces.get(&key) {
            return face;
        }
        let fallback = FontKey { family: self.default_family.clone(), ..key.clone() };
        let face = self.backend.face(&key).or_else(|| self.backend.face(&fallback)).unwrap_or_default();
        self.faces.insert(key, face);
        face
    }

    /// The ascent and descent of a style's lines, in pixels.
    pub fn line_metrics(&mut self, style: &TextStyle) -> (f32, f32) {
        let face = self.face_for(style);
        let m = self.backend.metrics(face);
        (m.ascent * style.size, m.descent * style.size)
    }

    /// The shaped token: a number composed from its characters' glyphs when the face allows
    /// it, otherwise from the cache or the backend.
    pub fn word(&mut self, token: &str, face: FaceId, size: f32) -> Rc<Word> {
        if let Some(word) = self.numeric.word(&mut *self.backend, token, face, size) {
            self.stats.words_composed += 1;
            return Rc::new(word);
        }
        let key = (face, size.to_bits());
        if let Some(word) = self.words.get(key, token) {
            self.stats.word_hits += 1;
            return word;
        }
        let glyphs = self.backend.shape(token, face, size);
        let width = glyphs.iter().map(|g| g.advance).sum();
        let word = Rc::new(Word { glyphs, width });
        self.words.put(key, token.into(), word.clone());
        self.stats.words_shaped += 1;
        word
    }

    /// A glyph's bitmap, from the backend: not cached here, the glyph atlas keeps tiles.
    pub fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        self.backend.rasterize(face, glyph, size, phase)
    }

    /// Where the ink of a glyph is at a size, against its pen position on the baseline (y
    /// grows down: the top of a letter is above 0); a glyph with no ink has no size. What
    /// lays out by ink and not by the lines of a face asks for the same few glyphs over
    /// and over, so the answers are kept.
    pub fn ink(&mut self, face: FaceId, glyph: u16, size: f32) -> Rect {
        let key = (face, glyph, size.to_bits());
        if let Some(ink) = self.inks.get(&key) {
            return *ink;
        }
        let bitmap = self.backend.rasterize(face, glyph, size, 0).unwrap_or_default();
        let ink = Rect::new(bitmap.left as f32, bitmap.top as f32, bitmap.width as f32, bitmap.height as f32);
        // ponytail: all are dropped when there are 8192; two generations, as the words
        // have, if sizes ever change so often that this shows.
        if self.inks.len() >= 8192 {
            self.inks.clear();
        }
        self.inks.insert(key, ink);
        ink
    }

    /// The counters since the last call, which resets them.
    pub fn take_stats(&mut self) -> TextStats {
        std::mem::take(&mut self.stats)
    }
}
