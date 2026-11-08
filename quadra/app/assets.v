// Assets: files embedded in the executable, looked up by path.
//   s.assets.add('i18n/vi.json', $embed_file('i18n/vi.json').to_bytes())
// Build with `-d embed_only_metadata` to read them from disk at run time instead, so
// editing an asset needs no rebuild.
module app

pub struct Assets {
mut:
	files map[string][]u8
}

pub fn (mut a Assets) add(path string, data []u8) {
	a.files[path] = data
}

pub fn (a &Assets) get(path string) ?[]u8 {
	return a.files[path] or { return none }
}

pub fn (a &Assets) text(path string) ?string {
	b := a.get(path)?
	return b.bytestr()
}

pub fn (a &Assets) paths() []string {
	return a.files.keys()
}
