// Color glyphs (emoji): TranslateColorGlyphRun splits a glyph of a color font (COLR) into
// layers, one color each. Each layer is rasterized as grayscale coverage and the layers
// are composed, in order, into one premultiplied RGBA bitmap for the poly atlas.
module dwrite

import quadra.platform.win

type FnTranslateColor = fn (this voidptr, x f32, y f32, run &GlyphRun, desc voidptr, measuring int, transform voidptr, palette u32, out &voidptr) int

type FnMoveNext = fn (this voidptr, has &int) int

type FnCurrentRun = fn (this voidptr, out &&ColorGlyphRun) int

type FnIsColor = fn (this voidptr) int

// ColorGlyphRun is DWRITE_COLOR_GLYPH_RUN.
struct ColorGlyphRun {
	run     GlyphRun
	desc    voidptr
	x       f32
	y       f32
	r       f32
	g       f32
	b       f32
	a       f32
	palette u16
}

const iid_font_face2 = win.guid('d8b768ff-64bc-4e66-982b-ec8e87f693f7')
const factory2_translate_color_glyph_run = 28
const face2_is_color_font = 30
const color_enum_move_next = 3
const color_enum_current_run = 4

// is_color_font tells whether a face has color layers (IDWriteFontFace2::IsColorFont).
fn is_color_font(face voidptr) bool {
	f2 := win.query_interface(face, &iid_font_face2) or { return false }
	defer {
		win.release(f2)
	}
	f := unsafe { FnIsColor(win.slot(f2, face2_is_color_font)) }
	return f(f2) != 0
}

struct Layer {
	left   int
	top    int
	width  int
	height int
	cov    []u8
	color  [4]f32
}

// raster_color renders a color glyph; none when the glyph has no color layers.
fn (f &Factory) raster_color(face &FontFace, glyph u16, size_px f32, subpixel_x f32) ?GlyphBitmap {
	index := glyph
	advance := f32(0)
	offset := GlyphOffset{}
	run := GlyphRun{
		face:     face.face
		em_size:  size_px
		count:    1
		indices:  &index
		advances: &advance
		offsets:  &offset
	}
	translate := unsafe { FnTranslateColor(win.slot(f.f, factory2_translate_color_glyph_run)) }
	mut layers_enum := unsafe { nil }
	if translate(f.f, subpixel_x, 0, &run, unsafe { nil }, measuring_natural, unsafe { nil }, 0,
		&layers_enum) < 0 {
		return none // DWRITE_E_NOCOLOR: a plain glyph
	}
	defer {
		win.release(layers_enum)
	}
	move_next := unsafe { FnMoveNext(win.slot(layers_enum, color_enum_move_next)) }
	current := unsafe { FnCurrentRun(win.slot(layers_enum, color_enum_current_run)) }
	mut layers := []Layer{}
	for {
		mut has := 0
		if move_next(layers_enum, &has) < 0 || has == 0 {
			break
		}
		mut cr := &ColorGlyphRun(unsafe { nil })
		if current(layers_enum, &cr) < 0 {
			break
		}
		cov := f.coverage(&cr.run, cr.x, cr.y) or { continue }
		if cov.width == 0 {
			continue
		}
		// Palette index 0xFFFF means the text color: black here (tiles are shared).
		color := if cr.palette == 0xFFFF { [f32(0), 0, 0, 1]! } else { [cr.r, cr.g, cr.b, cr.a]! }
		layers << Layer{cov.left, cov.top, cov.width, cov.height, cov.pixels, color}
	}
	return compose(layers)
}

// coverage rasterizes a glyph run as grayscale coverage at (x, y) from the pen.
fn (f &Factory) coverage(run &GlyphRun, x f32, y f32) !GlyphBitmap {
	create := unsafe { FnCreateRunAnalysis(win.slot(f.f, factory2_create_glyph_run_analysis)) }
	mut analysis := unsafe { nil }
	win.check(create(f.f, run, unsafe { nil }, rendering_natural_symmetric, measuring_natural,
		grid_fit_default, antialias_grayscale, x, y, &analysis), 'CreateGlyphRunAnalysis')!
	defer {
		win.release(analysis)
	}
	mut r := C.RECT{}
	bounds := unsafe { FnTextureBounds(win.slot(analysis, run_analysis_get_alpha_texture_bounds)) }
	win.check(bounds(analysis, texture_aliased_1x1, &r), 'GetAlphaTextureBounds')!
	w, h := r.right - r.left, r.bottom - r.top
	if w <= 0 || h <= 0 {
		return GlyphBitmap{}
	}
	mut raw := []u8{len: w * h}
	tex := unsafe { FnCreateTexture(win.slot(analysis, run_analysis_create_alpha_texture)) }
	win.check(tex(analysis, texture_aliased_1x1, &r, raw.data, u32(raw.len)), 'CreateAlphaTexture')!
	return GlyphBitmap{
		left:   r.left
		top:    r.top
		width:  w
		height: h
		pixels: raw
	}
}

// compose paints the layers in order (source over) into one premultiplied RGBA bitmap
// covering all of them.
fn compose(layers []Layer) ?GlyphBitmap {
	if layers.len == 0 {
		return none
	}
	mut x0, mut y0 := layers[0].left, layers[0].top
	mut x1, mut y1 := x0 + layers[0].width, y0 + layers[0].height
	for l in layers {
		x0, y0 = if l.left < x0 { l.left } else { x0 }, if l.top < y0 { l.top } else { y0 }
		x1 = if l.left + l.width > x1 { l.left + l.width } else { x1 }
		y1 = if l.top + l.height > y1 { l.top + l.height } else { y1 }
	}
	w, h := x1 - x0, y1 - y0
	mut px := []f32{len: w * h * 4}
	for l in layers {
		for y in 0 .. l.height {
			for x in 0 .. l.width {
				cov := f32(l.cov[y * l.width + x]) / 255 * l.color[3]
				if cov == 0 {
					continue
				}
				i := ((l.top - y0 + y) * w + (l.left - x0 + x)) * 4
				keep := 1 - cov
				px[i] = l.color[0] * cov + px[i] * keep
				px[i + 1] = l.color[1] * cov + px[i + 1] * keep
				px[i + 2] = l.color[2] * cov + px[i + 2] * keep
				px[i + 3] = cov + px[i + 3] * keep
			}
		}
	}
	mut out := []u8{len: px.len}
	for i, v in px {
		out[i] = u8(v * 255 + 0.5)
	}
	return GlyphBitmap{
		left:   x0
		top:    y0
		width:  w
		height: h
		color:  true
		pixels: out
	}
}
