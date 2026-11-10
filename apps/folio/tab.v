// Tabs (specs/21, Tabs): every open document has its own editor, preview scroll, outline
// and sync state. Opening a file that is open shows its tab; an untouched new document is
// replaced by the next file opened. The open files are the session, restored at start.
module main

import os
import quadra.core
import quadra.kit.code
import quadra.kit.md
import quadra.ui

@[heap]
pub struct Tab {
pub mut:
	id      int // stable for the tab's life (test ids, listeners)
	editor  core.View[code.Editor]
	path    string // '' while a new document has no file
	saved   string // the text as on disk
	text    string // the text as edited
	crlf    bool
	mtime   i64
	outside bool // changed on disk while it has unsaved changes here
	pos     &md.Positions   = &md.Positions{}
	memo    &md.Memo        = &md.Memo{}
	scroll  &ui.ScrollState = &ui.ScrollState{}
	sync    &SyncState      = &SyncState{}
	heads   []md.Heading // the outline at the last render
}

pub fn (t &Tab) dirty() bool {
	return t.text != t.saved
}

fn (t &Tab) name() string {
	return if t.path == '' { 'Untitled.md' } else { os.file_name(t.path) }
}

// blank: a new document nobody typed in; opening a file takes its place.
fn (t &Tab) blank() bool {
	return t.path == '' && t.text == ''
}

// cur is the active tab; callers check that a tab is open.
fn (f &Folio) cur() &Tab {
	return f.tabs[f.active]
}

fn (f &Folio) find(id int) ?&Tab {
	for t in f.tabs {
		if t.id == id {
			return t
		}
	}
	return none
}

fn (f &Folio) index_of(id int) int {
	for i, t in f.tabs {
		if t.id == id {
			return i
		}
	}
	return -1
}

// add_tab opens a document in a new tab after the active one and shows it.
fn (mut f Folio) add_tab(path string, d Disk, mut cx core.ViewCx[Folio]) &Tab {
	f.next_id++
	id := f.next_id
	ed := code.new_editor(mut cx.Cx, id: 'editor-${id}', path: 'doc.md', text: d.text, wrap: true)
	code.on_change(mut cx.Cx, ed, core.listener_with[Folio, int, code.Doc](cx.entity, id, fn (mut f Folio, id int, d &code.Doc, mut _ core.ViewCx[Folio]) {
		if mut t := f.find(id) {
			t.text = d.text()
		}
	}))
	code.on_link(mut cx.Cx, ed, cx.listener(fn (mut f Folio, url &string, mut cx core.ViewCx[Folio]) {
		f.follow_link(*url, mut cx) // ctrl-click on a URL in the editor
	}))
	t := &Tab{
		id:     id
		editor: ed
		path:   path
		saved:  d.text
		text:   d.text
		crlf:   d.crlf
		mtime:  d.mtime
	}
	at := if f.tabs.len == 0 { 0 } else { f.active + 1 }
	f.tabs.insert(at, t)
	f.activate(at, mut cx)
	return t
}

// activate shows tab i: the window title follows, the session is saved.
fn (mut f Folio) activate(i int, mut cx core.ViewCx[Folio]) {
	if i < 0 || i >= f.tabs.len {
		return
	}
	f.active = i
	mut t := f.cur()
	t.sync.editor_top = -1 // the preview finds the editor's line again
	cx.win.pw.set_title('${t.name()} - Folio')
	f.save_session(mut cx)
	f.focus_editor(mut cx)
}

// cycle moves to the next (1) or previous (-1) tab, wrapping around.
fn (mut f Folio) cycle(step int, mut cx core.ViewCx[Folio]) {
	if f.tabs.len > 1 {
		f.activate((f.active + step + f.tabs.len) % f.tabs.len, mut cx)
	}
}

// close_tab closes a tab, asking first when it has unsaved changes.
fn (mut f Folio) close_tab(id int, mut cx core.ViewCx[Folio]) {
	t := f.find(id) or { return }
	if t.dirty() {
		f.ask_save([t.id], false, mut cx)
		return
	}
	f.drop_tab(id, mut cx)
}

// drop_tab removes a tab without asking; the one to its right (else left) becomes active.
fn (mut f Folio) drop_tab(id int, mut cx core.ViewCx[Folio]) {
	i := f.index_of(id)
	if i < 0 {
		return
	}
	cx.release(f.tabs[i].editor.model())
	f.tabs.delete(i)
	if f.tabs.len == 0 {
		f.active = -1
		cx.win.pw.set_title('Folio')
		f.save_session(mut cx)
		return
	}
	f.activate(if i < f.active || f.active >= f.tabs.len { f.active - 1 } else { f.active }, mut cx)
}

// move_tab puts a dragged tab at index to.
fn (mut f Folio) move_tab(id int, to int, mut cx core.ViewCx[Folio]) {
	i := f.index_of(id)
	if i < 0 || to < 0 || to >= f.tabs.len || i == to {
		return
	}
	active := f.cur().id
	t := f.tabs[i]
	f.tabs.delete(i)
	f.tabs.insert(to, t)
	f.active = f.index_of(active)
	f.save_session(mut cx)
}

// save_session keeps the open files (and which one is shown) for the next start.
fn (mut f Folio) save_session(mut cx core.ViewCx[Folio]) {
	shown := if f.tabs.len > 0 { f.cur().path } else { '' }
	f.prefs_with(mut cx.Cx, Session{f.tabs.filter(it.path != '').map(it.path), shown}, fn (mut p Prefs, s Session, mut _ core.ViewCx[Prefs]) {
		p.session = s.paths
		p.shown = s.shown
	})
}

struct Session {
	paths []string
	shown string
}

// restore opens the last session's files that still exist.
fn (mut f Folio) restore(mut cx core.ViewCx[Folio]) {
	p := f.prefs.get(cx.Cx)
	paths, shown := p.session.clone(), p.shown
	for path in paths {
		if os.is_file(path) {
			f.open_path(path, mut cx)
		}
	}
	for i, t in f.tabs {
		if t.path == shown {
			f.activate(i, mut cx)
		}
	}
}
