// Fonts loaded from memory (text/dwrite/fonts_windows.c.v): a bundled font resolves by its
// family name, its weights and italics too, and draws. The kit's faces are named as its
// themes expect (kit.ui_face, kit.mono_face).
module main

import os
import quadra.text
import quadra.text.dwrite
import quadra.kit

const font_dir = os.join_path(os.dir(@FILE), '..', 'quadra', 'kit', 'fonts')

fn test_bundled_fonts_resolve_by_family() {
	mut b := dwrite.new_backend()!
	mut files := [][]u8{}
	for f in ['IBMPlexSans-Regular', 'IBMPlexSans-Italic', 'IBMPlexSans-SemiBold',
		'IBMPlexSans-SemiBoldItalic', 'Lilex-Regular', 'Lilex-Italic', 'Lilex-Bold',
		'Lilex-BoldItalic'] {
		files << os.read_bytes(os.join_path(font_dir, '${f}.ttf'))!
	}
	mut names := b.load_fonts(files[..1])! // one, then a batch
	names << b.load_fonts(files[1..])!
	assert names == [kit.ui_face, kit.ui_face, kit.ui_face, kit.ui_face, kit.mono_face, kit.mono_face,
		kit.mono_face, kit.mono_face]
	sans := kit.ui_face
	regular := b.face(text.FontKey{ family: sans, weight: 400 })!
	bold := b.face(text.FontKey{ family: sans, weight: 600 })!
	slanted := b.face(text.FontKey{ family: sans, weight: 400, italic: true })!
	segoe := b.face(text.FontKey{ family: 'Segoe UI' })!
	w := fn [mut b] (face text.FaceId) f32 {
		mut sum := f32(0)
		for g in b.shape('Hamburgefonstiv', face, 20) {
			sum += g.advance
		}
		return sum
	}
	assert w(regular) != w(segoe) // really Plex, not a fallback
	assert w(bold) > w(regular) // the bold instance
	assert w(slanted) != w(regular)
	assert b.load_fonts([[]u8{}]) or { ['none'] } == ['none']
	g := b.shape('a', regular, 32)
	bmp := b.raster(g[0].face, g[0].id, 32, 0)!
	assert bmp.width > 8 && bmp.height > 8
}

// contextual_off turns a family's contextual alternates off: Lilex draws -> as a ligature
// through calt, and as two plain glyphs without it.
fn test_contextual_off_drops_calt() {
	mut b := dwrite.new_backend()!
	b.load_fonts([os.read_bytes(os.join_path(font_dir, 'Lilex-Regular.ttf'))!])!
	mono := b.face(text.FontKey{ family: kit.mono_face, weight: 400 })!
	with := b.shape('->', mono, 20).map(it.id)
	b.contextual_off(kit.mono_face)
	without := b.shape('->', mono, 20).map(it.id)
	assert with != without
}
