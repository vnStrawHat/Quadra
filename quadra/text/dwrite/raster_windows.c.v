// Glyph rasterization to 8-bit coverage, through IDWriteGlyphRunAnalysis.
module dwrite

import quadra.platform.win

type FnCreateRunAnalysis = fn (this voidptr, run &GlyphRun, transform voidptr, rendering int, measuring int, grid_fit int, antialias int, x f32, y f32, out &voidptr) int

type FnTextureBounds = fn (this voidptr, kind int, out &C.RECT) int

type FnCreateTexture = fn (this voidptr, kind int, bounds &C.RECT, buf &u8, size u32) int

// GlyphBitmap is coverage in [0, 255]; left/top are relative to the pen position on the
// baseline. Subpixel (ClearType) bitmaps hold RGBA: per-subpixel coverage in r, g, b
// (always in red-green-blue screen order) and their maximum in a.
pub struct GlyphBitmap {
pub:
	left     int
	top      int
	width    int
	height   int
	subpixel bool
	color    bool // premultiplied RGBA
	pixels   []u8
}

// raster renders one glyph at size_px with a horizontal subpixel offset in [0, 1), as
// grayscale coverage or, with cleartype, as per-subpixel coverage (bgr: the panel runs
// blue-green-red).
pub fn (f &Factory) raster(face &FontFace, glyph u16, size_px f32, subpixel_x f32, cleartype bool, bgr bool) !GlyphBitmap {
	if face.color {
		if bmp := f.raster_color(face, glyph, size_px, subpixel_x) {
			return bmp
		}
	}
	texture_type := if cleartype { texture_cleartype_3x1 } else { texture_aliased_1x1 }
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
	create := unsafe { FnCreateRunAnalysis(win.slot(f.f, factory2_create_glyph_run_analysis)) }
	mut analysis := unsafe { nil }
	win.check(create(f.f, &run, unsafe { nil }, rendering_natural_symmetric, measuring_natural,
		grid_fit_default, if cleartype { antialias_cleartype } else { antialias_grayscale },
		subpixel_x, 0, &analysis), 'CreateGlyphRunAnalysis')!
	defer {
		win.release(analysis)
	}
	mut r := C.RECT{}
	bounds := unsafe { FnTextureBounds(win.slot(analysis, run_analysis_get_alpha_texture_bounds)) }
	win.check(bounds(analysis, texture_type, &r), 'GetAlphaTextureBounds')!
	w, h := r.right - r.left, r.bottom - r.top
	if w <= 0 || h <= 0 {
		return GlyphBitmap{
			left: r.left
			top:  r.top
		}
	}
	channels := if texture_type == texture_cleartype_3x1 { 3 } else { 1 }
	mut raw := []u8{len: w * h * channels}
	tex := unsafe { FnCreateTexture(win.slot(analysis, run_analysis_create_alpha_texture)) }
	win.check(tex(analysis, texture_type, &r, raw.data, u32(raw.len)), 'CreateAlphaTexture')!
	if channels == 1 {
		return GlyphBitmap{
			left:   r.left
			top:    r.top
			width:  w
			height: h
			pixels: raw
		}
	}
	mut rgba := []u8{len: w * h * 4}
	for i in 0 .. w * h {
		mut cr, cg, mut cb := raw[i * 3], raw[i * 3 + 1], raw[i * 3 + 2]
		if bgr {
			cr, cb = cb, cr
		}
		rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2] = cr, cg, cb
		rgba[i * 4 + 3] = if cr > cg {
			if cr > cb { cr } else { cb }
		} else {
			if cg > cb { cg } else { cb }
		}
	}
	return GlyphBitmap{
		left:     r.left
		top:      r.top
		width:    w
		height:   h
		subpixel: true
		pixels:   rgba
	}
}
