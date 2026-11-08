// Fonts loaded from memory (bundled with an app): an in-memory font file loader feeds a
// font set builder, whose font set becomes a font collection searched before the system's
// (DirectWrite 3, Windows 10 1703 and later).
module dwrite

import quadra.platform.win

type FnRegisterLoader = fn (this voidptr, loader voidptr) int

type FnMemReference = fn (this voidptr, factory voidptr, data voidptr, size u32, owner voidptr, out &voidptr) int

type FnAddFile = fn (this voidptr, file voidptr) int

type FnFromSet = fn (this voidptr, set voidptr, out &voidptr) int

const iid_factory5 = win.guid('958db99a-be2a-4f09-af7d-65189803d1d3')

// IDWriteFactory3 / 5
const factory_register_font_file_loader = 13
const factory3_create_font_collection_from_font_set = 37
const factory5_create_font_set_builder = 43
const factory5_create_in_memory_font_file_loader = 44
// IDWriteInMemoryFontFileLoader, IDWriteFontSetBuilder1
const mem_loader_create_reference = 4
const set_builder_create_font_set = 6
const set_builder1_add_font_file = 7

// Fonts is the state of the loaded fonts; it starts on the first load.
struct Fonts {
mut:
	f5         voidptr // IDWriteFactory5
	loader     voidptr // IDWriteInMemoryFontFileLoader, registered with the factory
	builder    voidptr // IDWriteFontSetBuilder1 holding every loaded file
	collection voidptr // IDWriteFontCollection1 of the loaded files
}

fn (mut f Factory) start_fonts() ! {
	if f.fonts.f5 != unsafe { nil } {
		return
	}
	f5 := win.query_interface(f.f, &iid_factory5) or {
		return error('loading fonts needs Windows 10 1703 or later')
	}
	mut loader := unsafe { nil }
	create_loader := unsafe { FnOut1(win.slot(f5, factory5_create_in_memory_font_file_loader)) }
	win.check(create_loader(f5, &loader), 'CreateInMemoryFontFileLoader')!
	register := unsafe { FnRegisterLoader(win.slot(f5, factory_register_font_file_loader)) }
	win.check(register(f5, loader), 'RegisterFontFileLoader')!
	mut builder := unsafe { nil }
	create_builder := unsafe { FnOut1(win.slot(f5, factory5_create_font_set_builder)) }
	win.check(create_builder(f5, &builder), 'CreateFontSetBuilder')!
	f.fonts = Fonts{f5, loader, builder, unsafe { nil }}
}

// load_fonts adds TrueType or OpenType fonts from memory and returns the family name of
// each. The bytes are copied. The searched collection is rebuilt once per call, so loading
// a family's files together costs one rebuild.
pub fn (mut f Factory) load_fonts(files [][]u8) ![]string {
	f.start_fonts()!
	fs := f.fonts
	reference := unsafe { FnMemReference(win.slot(fs.loader, mem_loader_create_reference)) }
	add := unsafe { FnAddFile(win.slot(fs.builder, set_builder1_add_font_file)) }
	mut names := []string{cap: files.len}
	for bytes in files {
		if bytes.len == 0 {
			return error('empty font data')
		}
		mut file := unsafe { nil }
		win.check(reference(fs.loader, fs.f5, bytes.data, u32(bytes.len), unsafe { nil }, &file),
			'CreateInMemoryFontFileReference')!
		names << f.add_file(file, add) or {
			win.release(file)
			return err
		}
		win.release(file)
	}
	all := f.builder_collection(fs.builder)!
	if fs.collection != unsafe { nil } {
		win.release(fs.collection)
	}
	f.fonts.collection = all
	return names
}

// add_file reads a font file's family name, from a collection of that file alone, and
// adds the file to the loaded set.
fn (f &Factory) add_file(file voidptr, add FnAddFile) !string {
	single := f.collection_of([file])!
	defer {
		win.release(single)
	}
	name := first_family_name(single)!
	win.check(add(f.fonts.builder, file), 'AddFontFile')!
	return name
}

// collection_of builds a collection from font files.
fn (f &Factory) collection_of(files []voidptr) !voidptr {
	mut builder := unsafe { nil }
	create_builder := unsafe { FnOut1(win.slot(f.fonts.f5, factory5_create_font_set_builder)) }
	win.check(create_builder(f.fonts.f5, &builder), 'CreateFontSetBuilder')!
	defer {
		win.release(builder)
	}
	add := unsafe { FnAddFile(win.slot(builder, set_builder1_add_font_file)) }
	for file in files {
		win.check(add(builder, file), 'AddFontFile')!
	}
	return f.builder_collection(builder)!
}

fn (f &Factory) builder_collection(builder voidptr) !voidptr {
	mut set := unsafe { nil }
	create_set := unsafe { FnOut1(win.slot(builder, set_builder_create_font_set)) }
	win.check(create_set(builder, &set), 'CreateFontSet')!
	defer {
		win.release(set)
	}
	mut coll := unsafe { nil }
	from_set := unsafe {
		FnFromSet(win.slot(f.fonts.f5, factory3_create_font_collection_from_font_set))
	}
	win.check(from_set(f.fonts.f5, set, &coll), 'CreateFontCollectionFromFontSet')!
	return coll
}

fn first_family_name(coll voidptr) !string {
	get_family := unsafe { FnGetFamily(win.slot(coll, collection_get_font_family)) }
	mut fam := unsafe { nil }
	win.check(get_family(coll, 0, &fam), 'GetFontFamily')!
	defer {
		win.release(fam)
	}
	name := names_of_family(fam)
	return if name == '' { error('the font has no family name') } else { name }
}
