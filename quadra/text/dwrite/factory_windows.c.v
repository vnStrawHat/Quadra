// DirectWrite factory, system font matching and font metrics.
module dwrite

import quadra.platform.win

type FnCreateFactory = fn (kind int, iid &win.Guid, out &voidptr) int

type FnOut1 = fn (this voidptr, out &voidptr) int

type FnGetCollection = fn (this voidptr, out &voidptr, check_updates int) int

type FnFindFamily = fn (this voidptr, name &u16, index &u32, exists &int) int

type FnGetFamily = fn (this voidptr, index u32, out &voidptr) int

type FnMatchFont = fn (this voidptr, weight int, stretch int, style int, out &voidptr) int

type FnGetMetrics = fn (this voidptr, out &FontMetrics)

@[heap]
pub struct Factory {
mut:
	f          voidptr // IDWriteFactory2
	analyzer   voidptr // IDWriteTextAnalyzer
	collection voidptr // system IDWriteFontCollection
	fallback   voidptr // system IDWriteFontFallback (nil before Windows 8.1)
	src_vtbl   &voidptr = unsafe { nil }
	sink_vtbl  &voidptr = unsafe { nil }
	fonts      Fonts // fonts loaded from memory (fonts_windows.c.v)
}

pub struct FontFace {
pub:
	face    voidptr // IDWriteFontFace
	metrics FontMetrics
	color   bool // has color layers (emoji, color_windows.c.v)
	no_calt bool // shaped with contextual alternates off (DWriteBackend.contextual_off)
}

pub fn new_factory() !&Factory {
	lib := win.load_system_dll('dwrite.dll')!
	create := unsafe { FnCreateFactory(win.dll_proc(lib, 'DWriteCreateFactory')!) }
	mut f := unsafe { nil }
	win.check(create(0, &iid_factory2, &f), 'DWriteCreateFactory(IDWriteFactory2)')! // 0 = shared
	get_collection := unsafe { FnGetCollection(win.slot(f, factory_get_system_font_collection)) }
	mut collection := unsafe { nil }
	win.check(get_collection(f, &collection, 0), 'GetSystemFontCollection')!
	create_analyzer := unsafe { FnOut1(win.slot(f, factory_create_text_analyzer)) }
	mut analyzer := unsafe { nil }
	win.check(create_analyzer(f, &analyzer), 'CreateTextAnalyzer')!
	return &Factory{
		f:          f
		analyzer:   analyzer
		collection: collection
		fallback:   system_fallback(f)
		src_vtbl:   source_vtable()
		sink_vtbl:  sink_vtable()
	}
}

// match_font finds the font closest to family/weight/style: among the loaded fonts first,
// then the installed ones.
pub fn (f &Factory) match_font(family string, weight int, style int) !FontFace {
	if f.fonts.collection != unsafe { nil } {
		if ff := match_in(f.fonts.collection, family, weight, style) {
			return ff
		}
	}
	return match_in(f.collection, family, weight, style)
}

fn match_in(coll voidptr, family string, weight int, style int) !FontFace {
	find := unsafe { FnFindFamily(win.slot(coll, collection_find_family_name)) }
	mut index := u32(0)
	mut exists := 0
	win.check(find(coll, family.to_wide(), &index, &exists), 'FindFamilyName')!
	if exists == 0 {
		return error('font family not installed: ${family}')
	}
	get_family := unsafe { FnGetFamily(win.slot(coll, collection_get_font_family)) }
	mut fam := unsafe { nil }
	win.check(get_family(coll, index, &fam), 'GetFontFamily')!
	defer {
		win.release(fam)
	}
	match_ := unsafe { FnMatchFont(win.slot(fam, family_get_first_matching_font)) }
	mut font := unsafe { nil }
	win.check(match_(fam, weight, font_stretch_normal, style, &font), 'GetFirstMatchingFont')!
	defer {
		win.release(font)
	}
	create_face := unsafe { FnOut1(win.slot(font, font_create_font_face)) }
	mut face := unsafe { nil }
	win.check(create_face(font, &face), 'CreateFontFace')!
	get_metrics := unsafe { FnGetMetrics(win.slot(face, face_get_metrics)) }
	mut m := FontMetrics{}
	get_metrics(face, &m)
	return FontFace{
		face:    face
		metrics: m
		color:   is_color_font(face)
	}
}

// ascent, descent and line gap scaled to an em size in px.
pub fn (ff &FontFace) line_metrics(size f32) (f32, f32, f32) {
	k := size / f32(ff.metrics.design_units_per_em)
	return f32(ff.metrics.ascent) * k, f32(ff.metrics.descent) * k, f32(ff.metrics.line_gap) * k
}
