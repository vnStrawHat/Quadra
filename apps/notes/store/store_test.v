module store

import os

fn scratch(name string) string {
	d := os.join_path(os.temp_dir(), 'quadra-tests', 'notes-store', name)
	os.rmdir_all(d) or {}
	return d
}

fn test_first_run_creates_default_folders() {
	root := scratch('first')
	lib := load(root)!
	assert lib.folders == default_folders
	assert lib.notes.len == 0
}

fn test_save_load_move_delete() {
	root := scratch('cycle')
	mut n := Note{
		id:       new_id()
		folder:   'Work'
		body:     '# Kế hoạch quý\n\n- việc 1'
		modified: 100
	}
	save(root, n)!
	save(root, n)! // overwriting an existing note works
	mut lib := load(root)!
	assert lib.notes.len == 1
	assert lib.notes[0].title() == 'Kế hoạch quý'
	move(root, n, 'Personal')!
	n.folder = 'Personal'
	lib = load(root)!
	assert lib.notes[0].folder == 'Personal'
	delete(root, n)!
	assert load(root)!.notes.len == 0
}

fn test_import_and_search() {
	root := scratch('import')
	os.mkdir_all(root)!
	a := os.join_path(root, 'a.md')
	os.write_file(a, 'Họp với Đà Nẵng')!
	os.write_file(os.join_path(root, 'b.png'), 'x')!
	notes := import_files([a, os.join_path(root, 'b.png')], 'Work')!
	assert notes.len == 1
	ix := build_index([notes[0], Note{
		id:   'z'
		body: 'Shopping list'
	}])
	never := Never{}
	assert ix.query('da nang', never) == [notes[0].id] // no diacritics needed
	assert ix.query('HOP', never).len == 2 // substring: also matches 'shopping'
	assert ix.query('HOP voi', never) == [notes[0].id]
	assert ix.query('list shopping', never) == ['z']
	assert ix.query('', never).len == 2
}

struct Never {}

fn (n Never) cancelled() bool {
	return false
}
