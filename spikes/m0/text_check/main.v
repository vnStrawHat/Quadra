// M0 spike: DirectWrite through V-implemented COM objects.
// Checks analysis callbacks, shaping of Vietnamese (NFC and NFD), and grayscale rasterization.
module main

import quadra.platform.win
import quadra.text.dwrite

fn wide(s string) []u16 {
	p := s.to_wide()
	mut out := []u16{}
	for i := 0; unsafe { p[i] } != 0; i++ {
		out << unsafe { p[i] }
	}
	return out
}

fn report(name string, ok bool, detail string) {
	status := if ok { 'PASS' } else { 'FAIL' }
	suffix := if detail != '' { ': ' + detail } else { '' }
	println('${status} ${name}${suffix}')
}

fn shape_all(f &dwrite.Factory, face &dwrite.FontFace, text []u16) !dwrite.ShapedRun {
	a := f.analyze(text, 'vi-vn')!
	return f.shape(text, 0, u32(text.len), a.scripts[0].analysis, face, 16, false, 'vi-vn')
}

fn main() {
	println('session ${win.session_id()}')
	f := dwrite.new_factory() or {
		report('DWriteCreateFactory', false, err.msg())
		exit(1)
	}
	face := f.match_font('Segoe UI', dwrite.font_weight_regular, dwrite.font_style_normal) or {
		report('match Segoe UI', false, err.msg())
		exit(1)
	}
	asc, desc, gap := face.line_metrics(16)
	report('font metrics', face.metrics.design_units_per_em > 0,
		'upem ${face.metrics.design_units_per_em}, at 16px ascent ${asc:.2f} descent ${desc:.2f} gap ${gap:.2f}')

	nfc := wide('Xin chào Việt Nam')
	a := f.analyze(nfc, 'vi-vn') or {
		report('analysis via V COM objects', false, err.msg())
		exit(1)
	}
	mut breaks := 0
	for b in a.breaks {
		if (b >> 2) & 3 == 1 { // breakConditionAfter == CAN_BREAK
			breaks++
		}
	}
	report('analysis via V COM objects', a.scripts.len > 0 && a.levels.len == nfc.len,
		'${a.scripts.len} script run(s), script id ${a.scripts[0].analysis.script}, ${breaks} break opportunities')

	run := shape_all(f, &face, nfc) or {
		report('shape NFC', false, err.msg())
		exit(1)
	}
	report('shape NFC "Xin chào Việt Nam"', run.glyphs.len == nfc.len && 0 !in run.glyphs,
		'${run.glyphs.len} glyphs for ${nfc.len} units, width ${run.width():.2f}px')

	// NFD "Việt": e + U+0302 + U+0323. Shaping must place the marks (or compose them) itself.
	nfd := [u16(0x56), 0x69, 0x65, 0x0302, 0x0323, 0x74]
	rn := shape_all(f, &face, nfd) or { panic(err) }
	rc := shape_all(f, &face, wide('Việt')) or { panic(err) }
	mut marks := 0
	for i, p in rn.glyph_props {
		if (p >> 5) & 1 == 1 || rn.advances[i] == 0 {
			marks++
		}
	}
	same_width := f32_abs(rn.width() - rc.width()) < 0.5
	report('shape NFD combining marks', 0 !in rn.glyphs && same_width,
		'${rn.glyphs.len} glyphs, ${marks} zero-advance/diacritic, width ${rn.width():.2f}px vs NFC ${rc.width():.2f}px')

	// Finding: in GRAYSCALE antialias mode, coverage comes from ALIASED_1x1 (8-bit AA),
	// while CLEARTYPE_3x1 reports empty bounds.
	gray := f.raster(&face, run.glyphs[0], 32, 0, dwrite.texture_aliased_1x1) or { panic(err) }
	mut max := u8(0)
	mut partial := 0
	for v in gray.pixels {
		if v > max {
			max = v
		}
		if v > 0 && v < 255 {
			partial++
		}
	}
	report('grayscale AA raster via ALIASED_1x1', gray.width > 0 && max > 200 && partial > 0,
		'${gray.width}x${gray.height} at (${gray.left},${gray.top}), max ${max}, ${partial} partial-coverage px')
	ct := f.raster(&face, run.glyphs[0], 32, 0, dwrite.texture_cleartype_3x1) or { panic(err) }
	report('CLEARTYPE_3x1 is empty in grayscale mode (expected)', ct.width == 0,
		'${ct.width}x${ct.height}')
}
