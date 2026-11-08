// The main window: toolbar, then the notes page (folders + list | editor), the table of all
// notes or the settings, chosen by a router (alt-left / alt-right go back and forward).
module main

import apps.notes.store
import quadra.app
import quadra.core
import quadra.kit
import quadra.kit.data
import quadra.kit.input
import quadra.platform
import quadra.ui

pub enum Page {
	notes
	table
	settings
}

pub struct Shell {
pub mut:
	notes    core.Model[Notes]
	prefs    app.Settings[Prefs]
	windows  app.Settings[app.WindowStates]
	router   app.Router[Page] = app.new_router[Page](Page.notes)
	folder   string // '' = all notes
	selected string
	editor   core.View[input.TextInput]
	search   core.View[input.TextInput]
	table    core.View[data.Table]
	shown    string // id of the note loaded in the editor
	table_v  int    // library version shown by the table
	version  int    // bumped whenever the notes model changes
}

@[markused]
pub fn (mut s Shell) render(mut cx core.ViewCx[Shell]) core.Element {
	if s.editor.id == 0 {
		s.setup(mut cx)
	}
	n := cx.read(s.notes)
	t := kit.theme(cx.Cx)
	page := match s.router.current() {
		.notes { s.notes_page(n, mut cx) }
		.table { s.table_page(mut cx) }
		.settings { s.settings_page(mut cx) }
	}

	root := ui.div().flex_col().size_full().bg(t.colors.background).child(kit.titlebar(app.t(cx.Cx,
		'app.title')).menu(app_menu(mut cx))).child(s.toolbar(n, mut cx)).child(ui.div().flex_col().grow().overflow_hidden().child(page)).child(s.status_bar(n, mut cx))
	return s.with_actions(root, mut cx)
}

fn (mut s Shell) setup(mut cx core.ViewCx[Shell]) {
	s.editor = input.textarea(mut cx.Cx,
		id:          'editor'
		placeholder: app.t(cx.Cx, 'note.placeholder')
		rows:        12
		fill:        true
	)
	input.on_change(mut cx.Cx, s.editor, cx.listener(fn (mut s Shell, body &string, mut cx core.ViewCx[Shell]) {
		if s.selected != '' {
			edit(mut cx.Cx, s.notes, s.selected, *body)
		}
	}))
	s.search = input.new_input(mut cx.Cx,
		id:          'search'
		placeholder: app.t(cx.Cx, 'search.placeholder')
		prefix:      kit.icon_search
	)
	input.on_change(mut cx.Cx, s.search, cx.listener(fn (mut s Shell, q &string, mut cx core.ViewCx[Shell]) {
		search(mut cx.Cx, s.notes, *q)
	}))
	s.table = data.new_table(mut cx.Cx,
		id:      'all-notes'
		columns: table_columns(cx.Cx)
		rows:    0
		cell:    fn (_ int, _ int) string {
			return ''
		}
	)
	data.on_select(mut cx.Cx, s.table, cx.listener(fn (mut s Shell, rows &[]int, mut cx core.ViewCx[Shell]) {
		if rows.len == 1 {
			n := cx.peek(s.notes)
			if rows[0] < n.lib.notes.len {
				s.open_note(n.lib.notes[rows[0]].id, mut cx)
			}
		}
	}))
	core.observe(mut cx, s.notes, fn (mut s Shell, m core.Model[Notes], mut cx core.ViewCx[Shell]) {
		s.version++
		s.sync_editor(mut cx)
		if s.router.current() == .table {
			s.refresh_table(mut cx) // keep the shown table live (imports, other windows)
		}
	})
	cx.on_close_requested(cx.listener(fn (mut s Shell, _ &core.CloseRequest, mut cx core.ViewCx[Shell]) {
		s.close_requested(mut cx)
	}))
	cx.on_files_dropped(cx.listener(fn (mut s Shell, e &platform.FileDropEvent, mut cx core.ViewCx[Shell]) {
		import_paths(mut cx.Cx, s.notes, e.paths, s.folder)
		kit.push_toast(mut cx.Cx, .success, app.tn(cx.Cx, 'note.imported', e.paths.len))
	}))
	cx.on_instance_args(cx.listener(fn (mut s Shell, e &platform.InstanceArgs, mut cx core.ViewCx[Shell]) {
		// A second launch with files: import them; without: just come to the front.
		paths := e.args.filter(it.ends_with('.md') || it.ends_with('.txt'))
		if paths.len > 0 {
			import_paths(mut cx.Cx, s.notes, paths, s.folder)
		}
	}))
}

// relabel refreshes the texts that inputs keep (placeholders) after a language switch.
pub fn (mut s Shell) relabel(mut cx core.ViewCx[Shell]) {
	input.set_placeholder(mut cx.Cx, s.editor, app.t(cx.Cx, 'note.placeholder'))
	input.set_placeholder(mut cx.Cx, s.search, app.t(cx.Cx, 'search.placeholder'))
	s.table_v = -1 // column titles too, next time the table is shown
}

// open_note selects a note and loads it into the editor.
pub fn (mut s Shell) open_note(id string, mut cx core.ViewCx[Shell]) {
	s.selected = id
	s.router.replace(.notes)
	n := cx.peek(s.notes)
	body := if note := n.find(id) { note.body } else { '' }
	s.shown = id
	input.set_value(mut cx.Cx, s.editor, body)
}

// sync_editor reloads the editor when its note changed elsewhere (another window).
fn (mut s Shell) sync_editor(mut cx core.ViewCx[Shell]) {
	if s.selected == '' {
		return
	}
	n := cx.peek(s.notes)
	note := n.find(s.selected) or {
		s.selected, s.shown = '', ''
		input.set_value(mut cx.Cx, s.editor, '')
		return
	}
	ed := cx.peek(s.editor.model())
	if ed.value() != note.body && !ed.focus.is_focused_in(cx.app) {
		input.set_value(mut cx.Cx, s.editor, note.body)
	}
}

pub fn (mut s Shell) new_note(mut cx core.ViewCx[Shell]) {
	id := create(mut cx.Cx, s.notes, s.folder)
	s.open_note(id, mut cx)
	ed := cx.peek(s.editor.model())
	ed.focus.focus(mut cx.Cx)
}

pub fn (mut s Shell) delete_selected(mut cx core.ViewCx[Shell]) {
	if s.selected == '' {
		return
	}
	remove(mut cx.Cx, s.notes, s.selected)
	s.selected, s.shown = '', ''
	input.set_value(mut cx.Cx, s.editor, '')
	kit.push_toast(mut cx.Cx, .info, app.t(cx.Cx, 'note.deleted'))
}

pub fn (mut s Shell) go(p Page) {
	if s.router.current() != p {
		s.router.push(p)
	}
}

// close_requested asks before closing when notes are unsaved; it also saves the window
// placement and pending settings.
fn (mut s Shell) close_requested(mut cx core.ViewCx[Shell]) {
	app.remember_placement(mut cx.Cx, s.windows, 'main')
	s.windows.flush(mut cx.Cx) or {}
	s.prefs.flush(mut cx.Cx) or {}
	if cx.peek(s.notes).dirty.len == 0 {
		return
	}
	cx.prevent_close()
	m := s.notes
	kit.confirm(mut cx.Cx, app.t(cx.Cx, 'close.title'), app.t(cx.Cx, 'close.message'), app.t(cx.Cx,
		'close.save'), cx.listener(fn [m] (mut s Shell, yes &bool, mut cx core.ViewCx[Shell]) {
		if *yes {
			save_all(mut cx.Cx, m) or {
				kit.push_toast(mut cx.Cx, .error, err.msg())
				return
			}
			cx.close_window()
		}
	}))
}

fn table_columns(cx &core.Cx) []data.Column {
	return [
		data.Column{
			title: app.t(cx, 'table.title')
			width: 300
		},
		data.Column{
			title: app.t(cx, 'table.folder')
			width: 140
		},
		data.Column{
			title: app.t(cx, 'table.modified')
			width: 180
		},
		data.Column{
			title:   app.t(cx, 'table.size')
			width:   110
			numeric: true
		},
	]
}

// table_cell formats one cell of the all-notes table from a snapshot of the library.
fn table_cell(notes []store.Note, untitled string, locale string) data.CellFn {
	return fn [notes, untitled, locale] (row int, col int) string {
		n := notes[row]
		return match col {
			0 {
				title_of(n, untitled)
			}
			1 {
				n.folder
			}
			2 {
				app.format_date(locale, time_of(n.modified)) + ' ' +
					app.format_time(locale, time_of(n.modified))
			}
			else {
				n.body.len.str()
			}
		}
	}
}
