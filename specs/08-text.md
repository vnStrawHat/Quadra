# 08 — Text

Text is shaped and rasterized by the OS and laid out by shared Rust code. A text node keeps its
result; nothing is looked up by content every frame.

## Layers of the text system

| Layer | Owner | Does |
|---|---|---|
| `TextBackend` | per OS (DirectWrite) | Font enumeration and loading, fallback, script analysis, shaping, rasterization |
| Shaping cache | `quadra-text` | Shaped words by text, face, size and features |
| Paragraph | the text node | Shaped runs, wrapped lines, hit positions |
| Glyph tiles | `quadra-render` atlases | Rasterized glyphs by face, glyph, device size and subpixel phase |

The rules for fallback, line breaking, wrapping, graphemes, cell widths, normalization and URL
detection are those of behaviour/04 and carry over. Script, line-break and bidirectional analysis
come from the backend.

The height of a line is the style's multiple of the size, in whole pixels
(`TextStyle::line_px`): lines of any size keep to the pixel grid, and the kit's 1.618 gives
23 px for 14, 19 for 12, 16 for 10.

As built for normalization (`normalize.rs`, tables from `tools/gen_nfc.py` for the Latin
blocks, which hold every Vietnamese letter): `normalize_nfc` takes letters apart, puts their
marks in canonical order and composes them again, so "e" with a circumflex and a dot below in
either order, or "ê" with a dot below, is one letter; text without marks is given back as it
is. An edit buffer normalizes what is inserted, and a mark typed after a letter replaces the
letter by what the two compose to. `fold_diacritics` takes the accents off for a search
("Hải Phòng" is "Hai Phong", "đ" is "d"). The tables for graphemes, cell widths and normalization are generated
from the Unicode data files by scripts in `tools/`, as in the V tree, and emitted as Rust
(01, R15).

## `TextBackend`

```rust
pub trait TextBackend {
    fn default_family(&self) -> String;
    fn families(&self) -> Vec<String>;
    fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>>;
    fn face(&mut self, key: &FontKey) -> Option<FaceId>;
    /// Per em: multiply by the font size.
    fn metrics(&self, face: FaceId) -> FaceMetrics;
    /// Shapes a word. Characters the face lacks come back in fallback faces: each glyph names
    /// its own face.
    fn shape(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph>;
    /// Added with the atlases.
    fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap>;
}
```

The backend is asked for one word at a time and does script analysis and fallback inside
`shape`, as the V backend did: the shared code never sees runs. Implementations: DirectWrite
(`quadra-native-win`); a fake with fixed metrics in `quadra-text` itself, for layout tests; a
monospace fake for terminal tests.

## The DirectWrite backend

The rules of behaviour/04 carry over. What differs from the V backend:

- **Fallback is asked only for what the face lacks.** A script run whose characters all have
  glyphs in the face is shaped with it; only other runs go through the system's fallback. A
  bundled font so keeps its own glyphs for every script it covers (the V backend sent every
  non-ASCII run through the fallback of the installed fonts, which does not know a bundled
  family).
- Printable ASCII skips script analysis: its analysis is known.
- The text source and sink DirectWrite calls back are one object that owns a copy of the
  word: no pointer into borrowed text crosses a COM boundary.
- Fonts from memory are a collection searched before the installed ones.
- How Windows smooths fonts (per subpixel or not, and the order of the stripes) is read again
  when a setting changes (`WM_SETTINGCHANGE`; `TextBackend::system_changed`): when glyphs
  come out otherwise, the app forgets every tile and each window paints again, as after a
  lost device. Shaping and lines are untouched.
- **OpenType features per family** (`Feature::on(b"tnum")`, `Feature::off(b"calt")`;
  `App::font_features(family, &[..])`, `TextSystem::set_features`): every run of the family
  is shaped with them besides the font's defaults, as one range of `GetGlyphs` and
  `GetGlyphPlacements`; glyphs of a fallback face are not the family's. Setting them empties
  the shaping cache; a paragraph keeps its words until its text or style changes, so
  features are set as fonts are loaded. Per family, not per style: a style stays what it is
  and a face is still one key of the cache.
- **Fallback families** (`App::font_fallbacks(&[..])`, `TextSystem::set_fallbacks`): families
  the app names are tried, in order, for every character a face lacks, before the system's
  fallback (a terminal's prompt with the icons of a patched font, a script the user prefers
  in one face). A family not installed is passed over. DirectWrite: a fallback of its builder
  maps the whole of Unicode to the families, then takes the system's mappings. Like features,
  setting them empties the shaping cache, and a paragraph that holds its words keeps them
  until its text or style changes.
- **Bidirectional text**, for display (`bidi.rs`; caret movement and editing in
  right-to-left text stay After-v1, below):
  - A text with a right-to-left character (`has_rtl`: Hebrew, Arabic and the scripts beside
    them, and the marks that turn text right to left) gets the level of each byte from the
    backend (`TextBackend::bidi_levels(text, rtl)`; DirectWrite's `AnalyzeBidi`). Its
    direction is that of its first letter (`reads_rtl`, rule P2 with letters as the strong
    characters). A text without one costs nothing more: no analysis, no reordering.
  - A word is shaped in visual order by the backend: each run of it in its own direction
    (`isRightToLeft`, which also mirrors brackets), the runs ordered as the levels of the
    word alone say. Its glyphs stay where they are on the endless line; caret stops stay
    logical, and a token starts at its pen.
  - A line puts its words and spaces in visual order (`visual_order`: rule L2 on tokens, the
    level of a word its lowest; spaces at the end of a line at the text's level, rule L1).
    The glyphs of such a line are placed again in a copy (`line_glyphs` gives it), so a
    text without right-to-left characters keeps one array.
  - One direction for the whole text, not one per line between newlines; a word longer than
    its line, broken inside, keeps its order; alignment is the view's.

## Paragraphs

A text node owns a `Paragraph`:

- **Shaped runs**, from the backend through the shaping cache. Rebuilt when the text or a
  shaping style (family, weight, size, features, language) changes.
- **Lines**, from breaking the shaped runs at a width. Rebuilt when the width changes; breaking
  uses the cached advances and shapes nothing. The same lines answer every width from the
  widest line up to the width they were broken at, which is what a layout asks: first at the
  space it has, then at the width it was told.
- **Glyph handles**, one per distinct glyph tile it paints with. Acquired at paint, kept until
  the paragraph changes or the node is removed (07, "Atlases").

What a change costs:

| Change | Work |
|---|---|
| Colour, decoration, selection range | Paint only: the lines are reused |
| Width | Break lines again; no shaping |
| One character typed in a long text | Shape the words that changed; break from that line on |
| Scale or device size | New glyph tiles; lines unchanged |

Rich text (spans of different style in one flow) is one paragraph with several style runs
(`Paragraph::rich`): each run says what it has otherwise than the paragraph's style, of the
family, the size, the weight and the slant (`Shaping`).

- A word is still what stands between two spaces: one that goes over the end of a run is
  shaped in pieces, a piece a run, set on one pen. It never breaks where the runs meet.
- A style of the paragraph that changes shapes the runs again from it: a run that says no
  size has the new one.
- The lines are as high as the paragraph's style says, and every run stands on their
  baseline; a line with a run larger than the style is as high as that run's line would
  be (its part over the baseline and under it, each grown by the run's size over the
  style's), on whole pixels, as a line with a box that reaches further is.
- **Cells** (`width.rs`): `char_width`, `grapheme_width`, `display_width` and `cell_step`
  count the cells of text where every cell is as wide as the next (code, a table written
  in text, a terminal): a mark that joins the letter before it has none, an East Asian
  wide character or an emoji two, a grapheme those of its widest character. The tables
  are carried from the V tree.
- **A run can be a box** (`Shaping::advance`): so wide, with no glyphs. Its text is one
  character, U+FFFC, which stands for what is no text; the pen goes past it and nothing is
  shaped. It is part of the word around it, has a caret stop before and after it, and
  `Paragraph::spans` gives where it is. What is drawn there is its maker's (05, rich text).

## Shaping cache

- Keyed by the word's text and the shaping style. Shared by all paragraphs, so repeated words
  across a document or a table are shaped once.
- Bounded by entry count with least-recently-used eviction. It is not tied to frames: a
  paragraph that exists holds its shaped runs itself and does not need the cache to stay warm.
- Numbers are the text that changes every frame (tables, meters, clocks), and each new one
  would be shaped. A number (digits and `. , - + % : /`) is composed from glyphs shaped once
  per size, when the face shapes numbers glyph by glyph: every pair of those characters must
  shape to its two glyphs at their own advances, checked once per face, and only once a face
  has been asked for some dozens of numbers: a window with a few never pays for the check.
  Composed numbers do not enter the cache, and a paragraph composes them into a buffer the
  text system keeps, not into a word of their own: shaping a number allocates nothing. The
  lines a wrap breaks a paragraph into are found in a kept buffer too.

## Glyph tiles

- A tile is identified by face, glyph, device size and subpixel phase (four horizontal
  phases). Vertical position is snapped to whole pixels.
- A rasterized glyph is one of three formats: coverage (a byte a pixel), subpixel coverage
  (the three stripes of an LCD pixel, blended per channel in the text's colour; what the
  backend gives when the OS smooths fonts that way) or a colour image (emoji). Coverage goes
  to the coverage atlas; the other two to the colour atlas.
- Lifetime is by use: counted handles, then a least-recently-used list under a byte budget
  (07). A long session at many zoom levels does not grow without bound.
- When the atlas epoch changes (device loss), paragraphs drop their handles and acquire again
  at the next paint.
- `quadra_tree::paint_glyph(cx, (face, glyph), size, pen, colour)` paints one glyph with its
  pen at a place in device pixels and gives the tile to hold: what paragraphs paint with, and
  what a widget that places glyphs itself (a formula) paints with.
- **Ink.** `TextSystem::ink(face, glyph, size)` is where the ink of a glyph is against its pen
  position on the baseline, from its bitmap, kept by face, glyph and size. A formula is laid
  out by ink: the ascent and descent of a face are the same for a minus and for an integral,
  and far more than either has.

## Editing support

`Paragraph` answers the questions an input needs: position of a byte offset, offset of a point,
the rectangles of a range, the next and previous grapheme, word and line boundaries, the visual
order of a mixed-direction line. Caret movement and editing in right-to-left text stay on the
After-v1 list (behaviour/13); mixed-direction text is displayed correctly.

## URLs

`find_urls(line)` lists the URLs of a line of plain text as byte ranges, each with what it
opens, by the rules of behaviour/04: a prefix (`https://`, `http://`, `www.`) after a word
boundary, up to a space, `<`, `>` or `"`, without the punctuation it ends with (a `)`
whose `(` is inside stays); `www.` opens as `https://www.`. A line with neither `://` nor
`www.` is not scanned. `url_openable(url)` is the gate for links in content the user did
not write: `http`, `https` and `mailto` only, with no control, direction or space
characters. What `find_urls` lists has passed it. One detector, for Markdown, the editor
and the terminal.

## Terminal and editor text

A cell grid does not use paragraphs. `quadra-term` and `quadra-code` use the lower pieces
directly:

- shaping per cell run with their own cache keyed for monospace use (an ASCII cell is a table
  lookup);
- glyph handles held per screen row or per visible line;
- instances written straight into their surface layer (07).

The text crate exposes those pieces as public API for this purpose; it is not a private detail
of the paragraph.

## IME

Composition text is a styled range inside the input's paragraph, shaped like any other text.
The candidate window is placed from the caret rectangle (09, 10).

## Tests

With the fake backend: wrapping and breaking rules, paragraph reuse counters (a width change
shapes nothing; a colour change breaks nothing), cache bounds, handle counts returning to zero
when nodes are removed. With DirectWrite, in the native test run: fallback, colour glyphs,
shaping of complex scripts, pixel snapshots.
