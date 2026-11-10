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

// load_notes reads the library in a task; the index is built in the same task. Callbacks
// take their data as arguments (`_with`): V frees no closure made in a handler (specs/14).
pub fn load_notes(mut cx core.Cx, m core.Model[Notes]) {
	cx.update(m, fn (mut n Notes, mut cx core.ViewCx[Notes]) {
		core.spawn_task_with[Notes, string, Loaded](mut cx, n.root, fn (root string, _ &core.TaskCx) !Loaded {
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

struct Edit {
	id   string
	body string
}

// edit changes a note's text and schedules its save.
pub fn edit(mut cx core.Cx, m core.Model[Notes], id string, body string) {
	cx.update_with(m, Edit{id, body}, fn (mut n Notes, e Edit, mut cx core.ViewCx[Notes]) {
		i := n.position(e.id)
		if i < 0 || n.lib.notes[i].body == e.body {
			return
		}
		n.lib.notes[i].body = e.body
		n.lib.notes[i].modified = time.now().unix()
		n.dirty[e.id] = true
		core.debounce_with(mut cx, 'save:${e.id}', autosave_ms, e.id, fn (mut n Notes, id string, mut cx core.ViewCx[Notes]) {
			n.save_later(id, mut cx)
		})
	})
}

// NoteJob is a task's data: the library folder and a note.
struct NoteJob {
	root   string
	note   store.Note
	folder string
	paths  []string
}

fn (mut n Notes) save_later(id string, mut cx core.ViewCx[Notes]) {
	note := n.find(id) or { return }
	n.saving++
	core.spawn_task_with[Notes, NoteJob, store.Note](mut cx, NoteJob{ root: n.root, note: note }, fn (j NoteJob, _ &core.TaskCx) !store.Note {
		store.save(j.root, j.note)!
		return j.note
	}, fn (mut n Notes, r &core.TaskResult[store.Note], mut _ core.ViewCx[Notes]) {
		n.saving--
		if !r.ok {
			n.error = r.error
			return
		}
		// Edits made while saving keep the note dirty; their own save is already scheduled.
		note := r.value
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
	cx.update_with(m, NoteJob{
		note:   store.Note{
			id: id
		}
		folder: folder
	}, fn (mut n Notes, j NoteJob, mut _ core.ViewCx[Notes]) {
		f := if j.folder == '' { n.lib.folders[0] } else { j.folder }
		n.lib.notes.insert(0, store.Note{
			id:       j.note.id
			folder:   f
			modified: time.now().unix()
		})
		n.dirty[j.note.id] = true
	})
	return id
}

fn report_error(mut n Notes, r &core.TaskResult[bool], mut _ core.ViewCx[Notes]) {
	if !r.ok {
		n.error = r.error
	}
}

pub fn remove(mut cx core.Cx, m core.Model[Notes], id string) {
	cx.update_with(m, id, fn (mut n Notes, id string, mut cx core.ViewCx[Notes]) {
		i := n.position(id)
		if i < 0 {
			return
		}
		note := n.lib.notes[i]
		n.lib.notes.delete(i)
		n.dirty.delete(id)
		core.spawn_task_with[Notes, NoteJob, bool](mut cx, NoteJob{ root: n.root, note: note }, fn (j NoteJob, _ &core.TaskCx) !bool {
			store.delete(j.root, j.note)!
			return true
		}, report_error)
	})
}

pub fn move_to(mut cx core.Cx, m core.Model[Notes], id string, folder string) {
	cx.update_with(m, NoteJob{
		note:   store.Note{
			id: id
		}
		folder: folder
	}, fn (mut n Notes, j NoteJob, mut cx core.ViewCx[Notes]) {
		i := n.position(j.note.id)
		if i < 0 || n.lib.notes[i].folder == j.folder {
			return
		}
		old := n.lib.notes[i]
		n.lib.notes[i].folder = j.folder
		// The saved file (if any) moves now; a pending save then writes the new folder.
		// ponytail: a save already running for the old folder can leave a stale copy there;
		// serialize saves per note if that ever shows up.
		core.spawn_task_with[Notes, NoteJob, bool](mut cx, NoteJob{
			root:   n.root
			note:   old
			folder: j.folder
		}, fn (j NoteJob, _ &core.TaskCx) !bool {
			store.move(j.root, j.note, j.folder)!
			return true
		}, report_error)
	})
}

// import_paths reads dropped files into folder (in a task) and saves them as notes.
pub fn import_paths(mut cx core.Cx, m core.Model[Notes], paths []string, folder string) {
	cx.update_with(m, NoteJob{ paths: paths, folder: folder }, fn (mut n Notes, j NoteJob, mut cx core.ViewCx[Notes]) {
		f := if j.folder == '' { n.lib.folders[0] } else { j.folder }
		core.spawn_task_with[Notes, NoteJob, []store.Note](mut cx, NoteJob{
			root:   n.root
			paths:  j.paths
			folder: f
		}, fn (j NoteJob, _ &core.TaskCx) ![]store.Note {
			notes := store.import_files(j.paths, j.folder)!
			for note in notes {
				store.save(j.root, note)!
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
