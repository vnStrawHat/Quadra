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
come from the backend. The tables for graphemes, cell widths and normalization are generated
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

## Paragraphs

A text node owns a `Paragraph`:

- **Shaped runs**, from the backend through the shaping cache. Rebuilt when the text or a
  shaping style (family, weight, size, features, language) changes.
- **Lines**, from breaking the shaped runs at a width. Rebuilt when the width changes; breaking
  uses the cached advances and shapes nothing.
- **Glyph handles**, one per distinct glyph tile it paints with. Acquired at paint, kept until
  the paragraph changes or the node is removed (07, "Atlases").

What a change costs:

| Change | Work |
|---|---|
| Colour, decoration, selection range | Paint only: the lines are reused |
| Width | Break lines again; no shaping |
| One character typed in a long text | Shape the words that changed; break from that line on |
| Scale or device size | New glyph tiles; lines unchanged |

Rich text (spans of different style in one flow) is one paragraph with several style runs.

## Shaping cache

- Keyed by the word's text and the shaping style. Shared by all paragraphs, so repeated words
  across a document or a table are shaped once.
- Bounded by entry count with least-recently-used eviction. It is not tied to frames: a
  paragraph that exists holds its shaped runs itself and does not need the cache to stay warm.
- ASCII text in a face known to have no ligatures or kerning for it takes a fast path that
  places glyphs from an advance table.

## Glyph tiles

- A tile is identified by face, glyph, device size and subpixel phase (four horizontal
  phases). Vertical position is snapped to whole pixels.
- Coverage glyphs go to the coverage atlas; colour glyphs to the colour atlas.
- Lifetime is by use: counted handles, then a least-recently-used list under a byte budget
  (07). A long session at many zoom levels does not grow without bound.
- When the atlas epoch changes (device loss), paragraphs drop their handles and acquire again
  at the next paint.

## Editing support

`Paragraph` answers the questions an input needs: position of a byte offset, offset of a point,
the rectangles of a range, the next and previous grapheme, word and line boundaries, the visual
order of a mixed-direction line. Caret movement and editing in right-to-left text stay on the
After-v1 list (behaviour/13); mixed-direction text is displayed correctly.

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
