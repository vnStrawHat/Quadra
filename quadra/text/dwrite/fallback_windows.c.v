// Font fallback (specs/04): IDWriteFontFallback::MapCharacters splits a run into ranges that
// one installed font covers, starting from the requested family.
module dwrite

import quadra.platform.win

// IDWriteFactory2
const factory2_get_system_font_fallback = 26
// IDWriteFontFallback
const fallback_map_characters = 3
// IDWriteFont, IDWriteFontFamily, IDWriteLocalizedStrings
const font_get_font_family = 3
const family_get_family_names = 6
const strings_find_locale_name = 4
const strings_get_string_length = 7
const strings_get_string = 8

type FnMapCharacters = fn (this voidptr, source voidptr, pos u32, len u32, collection voidptr, family &u16, weight int, style int, stretch int, mapped &u32, font &voidptr, scale &f32) int

type FnFindLocale = fn (this voidptr, name &u16, index &u32, exists &int) int

type FnStringLength = fn (this voidptr, index u32, len &u32) int

type FnGetString = fn (this voidptr, index u32, buf &u16, size u32) int

// FontRun is a range of UTF-16 units and the family that draws it ('' = the requested one).
pub struct FontRun {
pub:
	start  u32
	len    u32
	family string
}

fn system_fallback(f voidptr) voidptr {
	get := unsafe { FnOut1(win.slot(f, factory2_get_system_font_fallback)) }
	mut fb := unsafe { nil }
	if get(f, &fb) != 0 {
		return unsafe { nil }
	}
	return fb
}

// map_fonts covers text[start..start+len] with fonts, beginning with family.
pub fn (f &Factory) map_fonts(text []u16, start u32, len u32, family string, weight int, style int, locale string) []FontRun {
	whole := [FontRun{start, len, ''}]
	if f.fallback == unsafe { nil } {
		return whole
	}
	src := &AnalysisSource{
		ComObject: win.ComObject{
			vtbl: f.src_vtbl
			iid:  iid_analysis_source
		}
		text:      unsafe { &u16(text.data) }
		len:       u32(text.len)
		locale:    locale.to_wide()
	}
	map_ := unsafe { FnMapCharacters(win.slot(f.fallback, fallback_map_characters)) }
	base := family.to_wide()
	mut out := []FontRun{}
	mut pos := start
	end := start + len
	for pos < end {
		mut mapped := u32(0)
		mut font := unsafe { nil }
		mut scale := f32(1)
		if map_(f.fallback, voidptr(src), pos, end - pos, f.collection, base, weight, style,
			font_stretch_normal, &mapped, &font, &scale) != 0 {
			return whole
		}
		if mapped == 0 {
			mapped = 1
		}
		mut name := ''
		if font != unsafe { nil } {
			name = family_name(font)
			win.release(font)
		}
		if name == family {
			name = ''
		}
		if out.len > 0 && out.last().family == name {
			last := out.pop()
			out << FontRun{last.start, last.len + mapped, name}
		} else {
			out << FontRun{pos, mapped, name}
		}
		pos += mapped
	}
	return out
}

// family_name reads the English (or first) family name of an IDWriteFont.
fn family_name(font voidptr) string {
	get_family := unsafe { FnOut1(win.slot(font, font_get_font_family)) }
	mut fam := unsafe { nil }
	if get_family(font, &fam) != 0 {
		return ''
	}
	defer {
		win.release(fam)
	}
	return names_of_family(fam)
}

// names_of_family is a font family's name (English when it has one).
fn names_of_family(fam voidptr) string {
	get_names := unsafe { FnOut1(win.slot(fam, family_get_family_names)) }
	mut names := unsafe { nil }
	if get_names(fam, &names) != 0 {
		return ''
	}
	defer {
		win.release(names)
	}
	find := unsafe { FnFindLocale(win.slot(names, strings_find_locale_name)) }
	mut index := u32(0)
	mut exists := 0
	find(names, 'en-us'.to_wide(), &index, &exists)
	if exists == 0 {
		index = 0
	}
	length := unsafe { FnStringLength(win.slot(names, strings_get_string_length)) }
	mut n := u32(0)
	if length(names, index, &n) != 0 {
		return ''
	}
	mut buf := []u16{len: int(n) + 1}
	get := unsafe { FnGetString(win.slot(names, strings_get_string)) }
	if get(names, index, buf.data, n + 1) != 0 {
		return ''
	}
	return unsafe { string_from_wide2(&u16(buf.data), int(n)) }
}
