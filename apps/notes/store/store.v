// Notes on disk: <root>/<folder>/<id>.md, one plain-text file per note. The title is the
// first non-empty line. These functions do blocking I/O: call them from tasks.
module store

import os
import rand
import time

pub struct Note {
pub mut:
	id       string
	folder   string
	body     string
	modified i64 // unix seconds
}

pub fn (n &Note) title() string {
	for line in n.body.split_into_lines() {
		t := line.trim_space().trim_left('# ').trim_space()
		if t != '' {
			return clip(t, 80)
		}
	}
	return ''
}

// clip shortens s to at most n characters (never splitting a UTF-8 sequence).
pub fn clip(s string, n int) string {
	if s.len <= n {
		return s
	}
	r := s.runes()
	return if r.len <= n { s } else { r[..n].string() + '…' }
}

pub struct Library {
pub mut:
	folders []string
	notes   []Note
}

pub const default_folders = ['Personal', 'Work']

// load reads every note under root, creating the default folders on first run.
pub fn load(root string) !Library {
	os.mkdir_all(root)!
	mut lib := Library{}
	for name in os.ls(root)! {
		dir := os.join_path(root, name)
		if os.is_dir(dir) {
			lib.folders << name
		}
	}
	if lib.folders.len == 0 {
		for f in default_folders {
			os.mkdir_all(os.join_path(root, f))!
		}
		lib.folders = default_folders.clone()
	}
	lib.folders.sort()
	for folder in lib.folders {
		dir := os.join_path(root, folder)
		for file in os.ls(dir)! {
			if !file.ends_with('.md') {
				continue
			}
			path := os.join_path(dir, file)
			lib.notes << Note{
				id:       file.all_before_last('.md')
				folder:   folder
				body:     os.read_file(path)!
				modified: os.file_last_mod_unix(path)
			}
		}
	}
	sort_recent(mut lib.notes)
	return lib
}

// sort_recent orders notes by modification time, newest first.
pub fn sort_recent(mut notes []Note) {
	notes.sort_with_compare(fn (a &Note, b &Note) int {
		return if a.modified > b.modified {
			-1
		} else if a.modified < b.modified {
			1
		} else {
			compare_strings(a.id, b.id)
		}
	})
}

pub fn new_id() string {
	return '${time.now().unix_milli()}-${rand.u32() % 10000:04}'
}

pub fn note_path(root string, n &Note) string {
	return os.join_path(root, n.folder, '${n.id}.md')
}

// save writes a note atomically (temp file, then rename over the old one).
pub fn save(root string, n Note) ! {
	path := note_path(root, n)
	os.mkdir_all(os.dir(path))!
	tmp := path + '.tmp'
	os.write_file(tmp, n.body)!
	if os.exists(path) {
		os.rm(path)!
	}
	os.mv(tmp, path)!
}

pub fn delete(root string, n Note) ! {
	path := note_path(root, n)
	if os.exists(path) {
		os.rm(path)!
	}
}

// move puts a note into another folder on disk; a note never saved yet has no file to move.
pub fn move(root string, n Note, folder string) ! {
	if !os.exists(note_path(root, n)) {
		return
	}
	os.mkdir_all(os.join_path(root, folder))!
	os.mv(note_path(root, n), os.join_path(root, folder, '${n.id}.md'))!
}

pub fn add_folder(root string, name string) ! {
	os.mkdir_all(os.join_path(root, name))!
}

// import_files reads dropped .md and .txt files into new notes of folder.
pub fn import_files(paths []string, folder string) ![]Note {
	mut out := []Note{}
	for p in paths {
		ext := os.file_ext(p).to_lower()
		if ext != '.md' && ext != '.txt' {
			continue
		}
		out << Note{
			id:       new_id()
			folder:   folder
			body:     os.read_file(p)!
			modified: time.now().unix()
		}
	}
	return out
}
