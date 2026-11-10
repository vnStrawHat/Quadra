// The list of font families, for font pickers.
module dwrite

import quadra.platform.win

type FnGetCount = fn (this voidptr) u32

fn family_names_of(coll voidptr) []string {
	if coll == unsafe { nil } {
		return []
	}
	get_count := unsafe { FnGetCount(win.slot(coll, collection_get_font_family_count)) }
	count := get_count(coll)
	get_family := unsafe { FnGetFamily(win.slot(coll, collection_get_font_family)) }
	mut out := []string{cap: int(count)}
	for i in 0 .. count {
		mut fam := unsafe { nil }
		if get_family(coll, i, &fam) != 0 {
			continue
		}
		name := names_of_family(fam)
		win.release(fam)
		if name != '' {
			out << name
		}
	}
	return out
}

// font_families lists the installed families and the loaded ones, sorted, each once.
pub fn (mut b DWriteBackend) font_families() []string {
	mut all := family_names_of(b.f.collection)
	all << family_names_of(b.f.fonts.collection)
	all.sort()
	mut out := []string{cap: all.len}
	for name in all {
		if out.len == 0 || out.last() != name {
			out << name
		}
	}
	return out
}
