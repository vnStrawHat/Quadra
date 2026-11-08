// The document on disk: reading (line endings and a BOM noted), atomic saving and the
// modification time used to notice outside changes (specs/21, Files).
module main

import os
import quadra.platform

pub struct Disk {
pub:
	text  string // with \n line endings
	crlf  bool   // the file used \r\n
	mtime i64
}

// read_doc reads a Markdown file.
pub fn read_doc(path string) !Disk {
	raw := os.read_file(path)!
	mut s := if raw.starts_with('\xEF\xBB\xBF') { raw[3..] } else { raw }
	crlf := s.contains('\r\n')
	if crlf {
		s = s.replace('\r\n', '\n')
	}
	return Disk{s, crlf, os.file_last_mod_unix(path)}
}

// write_doc saves text (with \n endings) to path atomically: a temp file next to it, then
// moved over it. It returns the new modification time.
pub fn write_doc(path string, text string, crlf bool) !i64 {
	tmp := path + '.folio-tmp'
	os.write_file(tmp, if crlf { text.replace('\n', '\r\n') } else { text })!
	platform.replace_file(tmp, path) or {
		os.rm(tmp) or {}
		return err
	}
	return os.file_last_mod_unix(path)
}

// reading_minutes is the reading time at 230 words a minute, at least 1.
pub fn reading_minutes(words int) int {
	return if words <= 230 { 1 } else { (words + 229) / 230 }
}

// display_path shortens the home folder to ~.
pub fn display_path(path string) string {
	home := os.home_dir()
	return if home != '' && path.starts_with(home) { '~' + path[home.len..] } else { path }
}
