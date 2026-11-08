// The notes library as a model shared by every window. Disk I/O runs in background tasks
// owned by this model; edits are saved after a short pause (debounced autosave).
module main

import time
import apps.notes.store
import quadra.core

pub const autosave_ms = 800

pub struct Notes {
pub mut:
	root    string // folder holding the notes
	lib     store.Library
	loaded  bool
	error   string
	dirty   map[string]bool // note ids with unsaved edits
	saving  int
	index   store.Index
	results []string // ids matching the last search, in library order
	query   string
	search  core.TaskId
}

pub fn (n &Notes) find(id string) ?store.Note {
	for note in n.lib.notes {
		if note.id == id {
			return note
		}
	}
	return none
}

fn (n &Notes) position(id string) int {
	for i, note in n.lib.notes {
		if note.id == id {
			return i
		}
	}
	return -1
}

// visible returns the notes of folder ('' = all) that match the current search.
pub fn (n &Notes) visible(folder string) []store.Note {
	matching := if n.query.trim_space() == '' {
		map[string]bool{}
	} else {
		arrays_set(n.results)
	}
	return n.lib.notes.filter((folder == '' || it.folder == folder)
		&& (n.query.trim_space() == '' || it.id in matching))
}

fn arrays_set(ids []string) map[string]bool {
	mut m := map[string]bool{}
	for id in ids {
		m[id] = true
	}
	return m
}

// load_notes reads the library in a task; the index is built in the same task.
pub fn load_notes(mut cx core.Cx, m core.Model[Notes]) {
	cx.update(m, fn (mut n Notes, mut cx core.ViewCx[Notes]) {
		root := n.root
		core.spawn_task[Notes, Loaded](mut cx, fn [root] (_ &core.TaskCx) !Loaded {
			lib := store.load(root)!
			return Loaded{lib, store.build_index(lib.notes)}
		}, fn (mut n Notes, r &core.TaskResult[Loaded], mut _ core.ViewCx[Notes]) {
			n.loaded = true
			if r.ok {
				n.lib, n.index = r.value.lib, r.value.index
			} else {
				n.error = r.error
			}
		})
	})
}

pub struct Loaded {
	lib   store.Library
	index store.Index
}

// edit changes a note's text and schedules its save.
pub fn edit(mut cx core.Cx, m core.Model[Notes], id string, body string) {
	cx.update(m, fn [id, body] (mut n Notes, mut cx core.ViewCx[Notes]) {
		i := n.position(id)
		if i < 0 || n.lib.notes[i].body == body {
			return
		}
		n.lib.notes[i].body = body
		n.lib.notes[i].modified = time.now().unix()
		n.dirty[id] = true
		cx.debounce('save:${id}', autosave_ms, fn [id] (mut n Notes, mut cx core.ViewCx[Notes]) {
			n.save_later(id, mut cx)
		})
	})
}

fn (mut n Notes) save_later(id string, mut cx core.ViewCx[Notes]) {
	note := n.find(id) or { return }
	root := n.root
	n.saving++
	core.spawn_task[Notes, string](mut cx, fn [root, note] (_ &core.TaskCx) !string {
		store.save(root, note)!
		return note.id
	}, fn [note] (mut n Notes, r &core.TaskResult[string], mut _ core.ViewCx[Notes]) {
		n.saving--
		if !r.ok {
			n.error = r.error
			return
		}
		// Edits made while saving keep the note dirty; their own save is already scheduled.
		if cur := n.find(note.id) {
			if cur.body == note.body {
				n.dirty.delete(note.id)
			}
		}
		n.index = store.build_index(n.lib.notes)
	})
}

// save_all writes every dirty note now, on the calling thread (used when closing).
pub fn save_all(mut cx core.Cx, m core.Model[Notes]) ! {
	n := cx.peek(m)
	for id, _ in n.dirty {
		note := n.find(id) or { continue }
		store.save(n.root, note)!
	}
	cx.update(m, fn (mut n Notes, mut _ core.ViewCx[Notes]) {
		n.dirty.clear()
	})
}

// create adds an empty note to folder and returns its id.
pub fn create(mut cx core.Cx, m core.Model[Notes], folder string) string {
	id := store.new_id()
	cx.update(m, fn [id, folder] (mut n Notes, mut _ core.ViewCx[Notes]) {
		f := if folder == '' { n.lib.folders[0] } else { folder }
		n.lib.notes.insert(0, store.Note{
			id:       id
			folder:   f
			modified: time.now().unix()
		})
		n.dirty[id] = true
	})
	return id
}

pub fn remove(mut cx core.Cx, m core.Model[Notes], id string) {
	cx.update(m, fn [id] (mut n Notes, mut cx core.ViewCx[Notes]) {
		i := n.position(id)
		if i < 0 {
			return
		}
		note := n.lib.notes[i]
		n.lib.notes.delete(i)
		n.dirty.delete(id)
		root := n.root
		core.spawn_task[Notes, bool](mut cx, fn [root, note] (_ &core.TaskCx) !bool {
			store.delete(root, note)!
			return true
		}, fn (mut n Notes, r &core.TaskResult[bool], mut _ core.ViewCx[Notes]) {
			if !r.ok {
				n.error = r.error
			}
		})
	})
}

pub fn move_to(mut cx core.Cx, m core.Model[Notes], id string, folder string) {
	cx.update(m, fn [id, folder] (mut n Notes, mut cx core.ViewCx[Notes]) {
		i := n.position(id)
		if i < 0 || n.lib.notes[i].folder == folder {
			return
		}
		old := n.lib.notes[i]
		n.lib.notes[i].folder = folder
		root := n.root
		// The saved file (if any) moves now; a pending save then writes the new folder.
		// ponytail: a save already running for the old folder can leave a stale copy there;
		// serialize saves per note if that ever shows up.
		core.spawn_task[Notes, bool](mut cx, fn [root, old, folder] (_ &core.TaskCx) !bool {
			store.move(root, old, folder)!
			return true
		}, fn (mut n Notes, r &core.TaskResult[bool], mut _ core.ViewCx[Notes]) {
			if !r.ok {
				n.error = r.error
			}
		})
	})
}

// import_paths reads dropped files into folder (in a task) and saves them as notes.
pub fn import_paths(mut cx core.Cx, m core.Model[Notes], paths []string, folder string) {
	cx.update(m, fn [paths, folder] (mut n Notes, mut cx core.ViewCx[Notes]) {
		root := n.root
		f := if folder == '' { n.lib.folders[0] } else { folder }
		core.spawn_task[Notes, []store.Note](mut cx, fn [root, paths, f] (_ &core.TaskCx) ![]store.Note {
			notes := store.import_files(paths, f)!
			for note in notes {
				store.save(root, note)!
			}
			return notes
		}, fn (mut n Notes, r &core.TaskResult[[]store.Note], mut _ core.ViewCx[Notes]) {
			if r.ok {
				n.lib.notes.prepend(r.value)
				n.index = store.build_index(n.lib.notes)
			} else {
				n.error = r.error
			}
		})
	})
}
