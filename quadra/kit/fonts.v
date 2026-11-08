// The bundled faces (specs/15): IBM Plex Sans for the UI and Lilex for code, both under
// the SIL Open Font License (fonts/OFL-plex-sans.txt, fonts/OFL-lilex.txt).
module kit

import quadra.core

pub const ui_face = 'IBM Plex Sans'
pub const mono_face = 'Lilex'

// load_faces registers the bundled fonts with the app's text system and returns the family
// name of each file. kit.install calls it; themes name the faces by ui_face and mono_face.
// The UI face is shaped without contextual alternates (specs/15): labels keep plain forms.
pub fn load_faces(mut app core.App) ![]string {
	mut files := [
		$embed_file('fonts/IBMPlexSans-Regular.ttf'),
		$embed_file('fonts/IBMPlexSans-Italic.ttf'),
		$embed_file('fonts/IBMPlexSans-SemiBold.ttf'),
		$embed_file('fonts/IBMPlexSans-SemiBoldItalic.ttf'),
		$embed_file('fonts/Lilex-Regular.ttf'),
		$embed_file('fonts/Lilex-Italic.ttf'),
		$embed_file('fonts/Lilex-Bold.ttf'),
		$embed_file('fonts/Lilex-BoldItalic.ttf'),
	]
	// Views of the embedded bytes, not copies: the text system copies what it keeps, and
	// 1.6 MB of copies would grow the collected heap at start-up.
	mut views := [][]u8{cap: files.len}
	for mut f in files {
		views << unsafe { f.data().vbytes(f.len) }
	}
	app.text.contextual_off(ui_face)
	return app.text.load_fonts(views)
}
