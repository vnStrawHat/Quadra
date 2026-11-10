// Opening, saving, reloading and closing (specs/21, Files).
module main

import os
import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.platform

const md_filters = [
	platform.FileFilter{
		name:     'Markdown'
		patterns: '*.md;*.markdown;*.mdown;*.txt'
	},
	platform.FileFilter{
		name:     'All files'
		patterns: '*.*'
	},
]

// here is the folder of the shown file, where dialogs start.
fn (f &Folio) here() string {
	return if f.tabs.len > 0 && f.cur().path != '' { os.dir(f.cur().path) } else { '' }
}

fn (mut f Folio) open_dialog(mut cx core.ViewCx[Folio]) {
	paths := cx.pick_files(platform.FileDialog{
		title:    'Open Markdown'
		filters:  md_filters
		multiple: true
		dir:      f.here()
	})
	for path in paths {
		f.open_path(path, mut cx)
	}
}

// open_path shows path: its tab when it is open, else a new tab (or the untouched new
// document's tab).
fn (mut f Folio) open_path(path string, mut cx core.ViewCx[Folio]) {
	full := os.real_path(path)
	for i, t in f.tabs {
		if t.path == full {
			f.activate(i, mut cx)
			return
		}
	}
	d := read_doc(full) or {
		kit.push_toast(mut cx.Cx, .error, 'Cannot open ${os.file_name(path)}: ${err.msg()}')
		return
	}
	if f.tabs.len > 0 && f.cur().blank() {
		f.load(mut f.cur(), full, d, mut cx)
		f.activate(f.active, mut cx)
	} else {
		f.add_tab(full, d, mut cx)
	}
	f.prefs_with(mut cx.Cx, full, add_recent)
}

// load puts a document read from disk into a tab; its views start at the top.
fn (mut f Folio) load(mut t Tab, path string, d Disk, mut cx core.ViewCx[Folio]) {
	t.path, t.saved, t.text, t.crlf, t.mtime = path, d.text, d.text, d.crlf, d.mtime
	t.outside = false
	code.set_text(mut cx.Cx, t.editor, d.text)
	t.scroll.y = 0
	t.sync.editor_top, t.sync.section = -1, -1
}

fn (mut f Folio) new_doc(mut cx core.ViewCx[Folio]) {
	f.add_tab('', Disk{}, mut cx)
	f.set_layout(.raw, mut cx) // a new document is for writing
}

fn (mut f Folio) focus_editor(mut cx core.ViewCx[Folio]) {
	if f.tabs.len > 0 {
		cx.peek(f.cur().editor.model()).focus.focus(mut cx.Cx)
	}
}

fn (mut f Folio) save(mut cx core.ViewCx[Folio]) bool {
	return f.tabs.len > 0 && f.save_tab(mut f.cur(), mut cx)
}

// save_tab writes a document; a new one asks for a path first. It reports success.
fn (mut f Folio) save_tab(mut t Tab, mut cx core.ViewCx[Folio]) bool {
	if t.path == '' {
		return f.save_as_tab(mut t, mut cx)
	}
	t.mtime = write_doc(t.path, t.text, t.crlf) or {
		kit.push_toast(mut cx.Cx, .error, 'Cannot save: ${err.msg()}')
		return false
	}
	t.saved = t.text
	t.outside = false
	return true
}

fn (mut f Folio) save_as(mut cx core.ViewCx[Folio]) bool {
	return f.tabs.len > 0 && f.save_as_tab(mut f.cur(), mut cx)
}

fn (mut f Folio) save_as_tab(mut t Tab, mut cx core.ViewCx[Folio]) bool {
	paths := cx.pick_files(platform.FileDialog{
		title:        'Save Markdown'
		save:         true
		filters:      md_filters
		default_name: t.name()
		dir:          f.here()
	})
	if paths.len == 0 {
		return false
	}
	mut path := paths[0]
	if os.file_ext(path) == '' {
		path += '.md'
	}
	t.path = path
	if !f.save_tab(mut t, mut cx) {
		return false
	}
	f.prefs_with(mut cx.Cx, path, add_recent)
	f.activate(f.active, mut cx) // the title and the session
	return true
}

// check_outside notices changes on disk: a tab with nothing unsaved reloads, else its
// banner asks.
fn (mut f Folio) check_outside(mut cx core.ViewCx[Folio]) {
	for mut t in f.tabs {
		if t.path == '' || t.outside || !os.is_file(t.path) {
			continue
		}
		if os.file_last_mod_unix(t.path) == t.mtime {
			continue
		}
		if t.dirty() {
			t.outside = true
			continue
		}
		f.reload(mut t, mut cx)
	}
}

// reload reads a tab's file again, keeping its scroll positions.
fn (mut f Folio) reload(mut t Tab, mut cx core.ViewCx[Folio]) {
	d := read_doc(t.path) or { return }
	keep := t.scroll.y
	keep_ed := cx.peek(t.editor.model()).scroll
	f.load(mut t, t.path, d, mut cx)
	t.scroll.y = keep
	mut e := cx.peek(t.editor.model())
	e.scroll = keep_ed
	t.sync.editor_top = e.top_line()
}

// close_requested asks before closing a window with unsaved changes, and saves settings.
fn (mut f Folio) close_requested(mut cx core.ViewCx[Folio]) {
	f.prefs.flush(mut cx.Cx) or {}
	f.windows.flush(mut cx.Cx) or {}
	dirty := f.tabs.filter(it.dirty()).map(it.id)
	if dirty.len > 0 {
		cx.prevent_close()
		f.ask_save(dirty, true, mut cx)
	}
}
