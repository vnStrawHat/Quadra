# 04 — Text

Module `quadra.text`. Shared pure-V logic on top of a per-OS `TextBackend`. DirectWrite is the Windows backend.

## Split of responsibilities

| Shared (pure V) | Backend (per OS) |
|---|---|
| Font registry and style matching API, word cache, line layout cache, wrapping, truncation, caret and hit-testing, NFC, grapheme segmentation, rich-text spans, glyph-to-atlas mapping | Font enumeration and loading, fallback, script itemization, shaping, glyph metrics, glyph rasterization, color glyph decomposition |

```v
pub interface TextBackend {
mut:
	load_font(bytes []u8) !FontFace                 // app-bundled fonts
	match_font(family string, w Weight, s FontStyle) !FontFace
	analyze(text string, face FontFace) []TextRun     // script, bidi level, fallback face, line-break opportunities
	shape(text string, face FontFace, size f32, features []Feature) ShapedRun
	metrics(face FontFace) FontMetrics
	raster(glyph GlyphKey) !GlyphBitmap             // R8 alpha or RGBA for color glyphs
}
```

## DirectWrite backend (`text/dwrite/`)
- Factory: `DWriteCreateFactory(SHARED)`, queried up to `IDWriteFactory5` (in-memory font loader for bundled fonts) with graceful fallback to `IDWriteFactory2`.
- Analysis: `AnalyzeScript`, `AnalyzeBidi` and `AnalyzeLineBreakpoints` on `IDWriteTextAnalyzer`, plus `IDWriteFontFallback::MapCharacters` over the system fallback and app-defined families, merged into `TextRun`s. As built: `DWriteBackend.shape` maps each non-ASCII script run with `MapCharacters` (system fallback, from the requested family) and shapes every range with its font; fallback faces are cached by family like any other face (`tests/fallback_test.v`). ASCII runs skip the lookup.
- Shaping: `IDWriteTextAnalyzer` for script itemization, then `GetGlyphs` and `GetGlyphPlacements`. This gives correct shaping for complex scripts and correct mark positioning for Vietnamese without hand-written stacking.
- Rasterization: `IDWriteFactory2::CreateGlyphRunAnalysis` with `NATURAL_SYMMETRIC` rendering.
  - **ClearType** (when Windows has it on, `SPI_GETFONTSMOOTHINGTYPE`): `CLEARTYPE` antialiasing read as `CLEARTYPE_3x1`. Per-subpixel coverage is stored as RGBA in the poly atlas (r, g, b in screen order, swapped for BGR panels; a = max). The shader blends it per channel with dual-source blending (`SV_Target1` is the per-channel weight; every other primitive outputs its alpha there, so it stays premultiplied-alpha blending). This needs an opaque target, which every Quadra surface is (`DXGI_ALPHA_MODE_IGNORE`, cleared offscreen targets). `DWriteBackend.cleartype` turns it off.
  - **Grayscale** otherwise: `GRAYSCALE` antialiasing read as `ALIASED_1x1` (8-bit alpha in the mono atlas; `CLEARTYPE_3x1` returns empty bounds in this mode).
  - Both get a coverage correction in the shader (glyphs only, not icons): DirectWrite's default enhanced contrast (0.5) for all text, and gamma 1.8 weighted by the text's luminance, since the background is unknown. Without it light text on dark planes looks thin.
  - Not handled yet: a ClearType setting change while running (needs a glyph-cache flush).
- Color emoji: for faces that report `IsColorFont`, `TranslateColorGlyphRun` splits a glyph into COLR layers. Each layer is rasterized as grayscale coverage and composed, in order, into one premultiplied RGBA tile in the poly atlas; the text color's alpha (and opacity) still applies. Layers in the text color (palette index 0xFFFF) are drawn black, because tiles are shared across colors. Not covered: bitmap (PNG, CBDT) and SVG glyphs, and COLRv1 gradients. Windows has no flag emoji: regional indicators show as letters.
- All calls go through the vtable-index COM helper (01-decisions D9).

## Caching strategy (where text performance comes from)
1. **Word cache.** Text is split at spaces into words, and each word is shaped once per `(word, face, size, features)`. Changing one word in a long paragraph reshapes one word. Scripts without spaces (CJK, Thai) are split at backend-reported break opportunities instead. Word, line and wrap caches are flat maps keyed by a hash of the text plus face and size bits (one lookup per hit, collisions detected); entries unused for two frames are swept every 32 frames (`quadra/text/cache.v`).
2. **Line cache.** A finished line layout is keyed by `(text hash, style hash, max width)` and survives while referenced. Cached views keep their lines, so a clean view never touches text code.
3. **Wrapping** walks cached word advances, which makes it O(words) with no shaping in the loop.
4. **Glyph tiles** are keyed by `(face, glyph, device size, subpixel x / 4)`. Device size is `size × scale`, so a 150% monitor gets its own sharp tiles.

## API used by elements

```v
pub struct TextStyle {
	family      string
	size        f32
	weight      Weight
	style       FontStyle
	color       Color
	line_height f32
	features    []Feature   // e.g. tabular numbers
}
pub fn (mut ts TextSystem) layout_line(s string, st TextStyle) &LineLayout
pub fn (mut ts TextSystem) layout_mono(s string, st TextStyle, cell_w f32) &LineLayout // on a cell grid
pub fn (mut ts TextSystem) wrap(s string, st TextStyle, max_w f32) &WrappedText
pub fn (mut ts TextSystem) layout_rich(runs []Run, max_w f32) &RichText  // styled runs as one flow; breaks at spaces only
pub fn (l &LineLayout) x_for_index(byte_i int) f32
pub fn (l &LineLayout) index_for_x(x f32) int
```
- Indices are UTF-8 byte offsets on **extended grapheme cluster** boundaries (`grapheme.v`, UAX #29 tables generated by a script).
- Monospace display width (`width.v`, tables in `width_table.v`): `rune_width` is 0 for combining marks, 2 for East Asian wide characters and emoji, 1 otherwise (the terminal grid, specs/17); `display_width` counts each grapheme as its widest code point (column-aligned text: the editor's monospace grid, specs/18), `grapheme_width(s, i, j)` one grapheme of it.
- `layout_mono(s, st, cell_w)` is `layout_line` on a monospace grid (the code editor, specs/18): caret stops at `display_width(s[..i]) × cell_w`, each grapheme's glyphs moved onto its column and, for a non-ASCII grapheme, centred in its cells, so a fallback glyph never shifts the columns after it. A pure ASCII line is the plain layout; others are cached under their own key, built once.
- All text entering the framework through setters, paste and IME commit is NFC-normalized. This is for buffer semantics (caret, search, equality); rendering does not depend on it, since DirectWrite shapes NFD input to the same glyphs (measured in M0).
- URLs in plain text (`text/url.v`): the one detector, with one rule set, for the terminal,
  the code editor and Markdown. A URL starts at a word boundary with a prefix (`https://`,
  `http://`, `www.`; no `ftp://`) and runs to whitespace, `<`, `>` or `"`; trailing
  `.,;:!?'")]}>` and backticks are stripped, except a closing `)` that has its `(` inside the
  URL (`.../Foo_(bar)`), and `www.` opens as `https://www.`. `find_urls(s)` gives byte ranges
  for a line and scans nothing when the line has no `://` or `www.`. `url_openable(url)` is the gate for user content: `http`, `https` and
  `mailto` only, with no control, bidi or space characters; no other scheme (`file:`, `ms-*:`,
  custom handlers) ever reaches the shell.
- Bidi: the backend reports bidi levels and lines are reordered for display in v1, so mixed RTL text renders correctly. RTL caret movement and editing are post-v1.

## IME
The focused text input draws the preedit string at the caret with a `Line` underline and does not modify its buffer until commit. After each frame it calls `set_ime_rect`.

## Tests
`word_cache_test.v`, `wrap_test.v`, `grapheme_test.v` (Vietnamese, emoji ZWJ sequences), `nfc_test.v`, and a Windows-only backend test that shapes "Tiếng Việt" and checks glyph count and advances are stable.
