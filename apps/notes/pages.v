// The notes page: folders and note list on the left, the editor on the right.
module main

import time
import apps.notes.store
import quadra.app
import quadra.core
import quadra.kit
import quadra.ui

fn time_of(unix i64) time.Time {
	return time.unix(unix).local()
}

fn (mut s Shell) toolbar(_ &Notes, mut cx core.ViewCx[Shell]) core.Element {
	t := kit.theme(cx.Cx)
	tabs := [app.t(cx.Cx, 'nav.notes'), app.t(cx.Cx, 'nav.table'),
		app.t(cx.Cx, 'nav.settings')]
	return ui.div().flex_row().items_center().gap(8).px(10).h(40).shrink_0().bg(t.p.panel).border_b(1).border_color(t.p.line).child(kit.button('new-note').label(app.t(cx.Cx,
		'note.new')).icon(kit.icon_plus).primary().size(.sm).on_click(cx.listener(fn (mut s Shell, _ &core.ClickEvent, mut cx core.ViewCx[Shell]) {
		s.new_note(mut cx)
	}))).child(ui.div().w(260).child(cx.child_view(s.search))).child(ui.div().grow()).child(kit.tabs('pages',
		tabs, int(s.router.current())).pill().on_change(cx.listener(fn (mut s Shell, i &int, mut cx core.ViewCx[Shell]) {
		s.show(unsafe { Page(*i) }, mut cx)
	})))
}

fn (mut s Shell) status_bar(n &Notes, mut cx core.ViewCx[Shell]) core.Element {
	t := kit.theme(cx.Cx)
	state := if n.saving > 0 {
		kit.status_label(.pending, app.t(cx.Cx, 'note.saving'))
	} else if n.dirty.len > 0 {
		kit.status_label(.pending, app.t(cx.Cx, 'note.unsaved'))
	} else {
		kit.status_label(.succeeded, app.t(cx.Cx, 'note.saved'))
	}
	mut right := [core.Element(state)]
	if n.error != '' {
		right << ui.text(n.error).color(t.p.error)
	}
	return ui.div().test_id('status').flex_col().child(kit.statusbar([
		kit.status_item(kit.icon_file, app.tn(cx.Cx, 'note.count', n.lib.notes.len)),
	], right))
}

fn (mut s Shell) notes_page(n &Notes, mut cx core.ViewCx[Shell]) core.Element {
	if !n.loaded {
		return ui.div().size_full().flex_row().items_center().justify_center().gap(8).child(kit.spinner()).child(kit.label(app.t(cx.Cx,
			'loading')))
	}
	left :=
		ui.div().flex_row().size_full().child(s.folders(n, mut cx)).child(s.note_list(n, mut cx))
	return kit.split('main-split', left, s.editor_pane(n, mut cx), cx.peek(s.prefs.model).split).on_resize(cx.listener(fn (mut s Shell, r &f32, mut cx core.ViewCx[Shell]) {
		ratio := *r
		s.prefs.update(mut cx.Cx, fn [ratio] (mut p Prefs, mut _ core.ViewCx[Prefs]) {
			p.split = ratio
		})
	}))
}

fn (mut s Shell) folders(n &Notes, mut cx core.ViewCx[Shell]) core.Element {
	t := kit.theme(cx.Cx)
	mut col :=
		ui.div().flex_col().w(180).shrink_0().h_full().py(6).bg(t.p.panel).border_r(1).border_color(t.p.line)
	col.child(s.folder_item('', app.t(cx.Cx, 'nav.all'), kit.icon_layers, n.lib.notes.len, mut cx))
	col.child(kit.section_label(app.t(cx.Cx, 'nav.folders'), t).pl(10).pt(12).pb(4))
	for f in n.lib.folders {
		count := n.lib.notes.filter(it.folder == f).len
		col.child(s.folder_item(f, f, kit.icon_folder, count, mut cx))
	}
	return col
}

// folder_item is a sidebar row (specs/15): hover one step, the current folder two.
fn (mut s Shell) folder_item(id string, label string, glyph string, count int, mut cx core.ViewCx[Shell]) core.Element {
	t := kit.theme(cx.Cx)
	p := t.p
	on := s.folder == id
	return ui.div().test_id('folder-${id}').flex_row().items_center().gap(6).pl(10).pr(8).h(t.row_h).cursor(.pointer).bg(if on {
		p.press
	} else {
		p.panel
	}).text_color(p.text).hover(ui.Patch{
		background: if on { p.press } else { p.raised_hover }
	}).role(.listitem).label(label).selected(on).child(kit.icon(glyph).size(16).color(p.text_2)).child(ui.text(label)).child(ui.div().grow()).child(ui.text(count.str()).family(t.mono_family).size(t.font_size(.sm)).color(p.text_2)).on_click(core.listener_with[Shell, string, core.ClickEvent](cx.entity,
		id, fn (mut s Shell, id string, _ &core.ClickEvent, mut _ core.ViewCx[Shell]) {
		s.folder = id
	}))
}

fn (mut s Shell) note_list(n &Notes, mut cx core.ViewCx[Shell]) core.Element {
	t := kit.theme(cx.Cx)
	notes := n.visible(s.folder)
	if notes.len == 0 {
		msg := if n.query.trim_space() != '' { 'search.none' } else { 'note.empty' }
		return ui.div().grow().p(16).child(kit.label(app.t(cx.Cx, msg)))
	}
	ent := cx.entity
	selected := s.selected
	untitled := app.t(cx.Cx, 'note.untitled')
	loc := app.locale(cx.Cx)
	now := time.now()
	folders := n.lib.folders
	m := s.notes
	labels := [app.t(cx.Cx, 'note.open_window'), app.t(cx.Cx, 'note.delete')]
	move_label := app.t(cx.Cx, 'note.move_to')
	list := ui.uniform_list_with('note-list', notes.len, 60, NoteRows{notes, ent, selected, untitled, loc, now, folders, m, labels, move_label, unsafe { t }},
		note_row)
	return ui.div().flex_col().grow().h_full().child(list)
}

// NoteRows is what the visible rows of the note list are built from.
struct NoteRows {
	notes      []store.Note
	ent        u32
	selected   string
	untitled   string
	loc        string
	now        time.Time
	folders    []string
	m          core.Model[Notes]
	labels     []string
	move_label string
	t          &kit.Theme = unsafe { nil }
}

fn note_row(d NoteRows, i int) core.Element {
	notes, ent, selected, untitled, loc, now, folders, m, labels, move_label, t := d.notes, d.ent, d.selected, d.untitled, d.loc, d.now, d.folders, d.m, d.labels, d.move_label, d.t
	note := notes[i]
	id := note.id
	on := id == selected
	p := t.p
	// Selected two steps up, hover one (specs/15); the title in text, the rest muted.
	content := ui.div().flex_col().justify_center().gap(2).grow().px(12).child(ui.div().flex_row().items_center().gap(8).child(ui.text(title_of(note,
		untitled))).child(ui.div().grow()).child(ui.text(app.format_relative(loc,
		time_of(note.modified), now)).size(t.font_size(.sm)).color(p.text_2))).child(ui.text(preview(note)).size(t.font_size(.sm)).color(p.text_2))
	row := ui.div().test_id('note-${i}').flex_row().h(60).w_full().border_b(1).border_color(p.line).cursor(.pointer).bg(if on {
		p.press
	} else {
		p.canvas
	}).hover(ui.Patch{
		background: if on { p.press } else { p.panel_hover }
	}).role(.listitem).label(title_of(note, untitled)).selected(on).child(content).on_click(core.listener_with[Shell, string, core.ClickEvent](ent,
		id, fn (mut s Shell, id string, _ &core.ClickEvent, mut cx core.ViewCx[Shell]) {
		s.open_note(id, mut cx)
	}))
	return kit.context_menu(row, note_menu(m, id, note.folder, folders, labels, move_label))
}

// note_menu is the context menu of a list row: open in a window, move, delete.
fn note_menu(m core.Model[Notes], id string, folder string, folders []string, labels []string, move_label string) []kit.MenuItem {
	mut items := [
		kit.MenuItem{
			label:  labels[0]
			glyph:  kit.icon_external_link
			action: NoteAction{m, id, .open, ''}
		},
		kit.separator(),
	]
	for f in folders {
		if f == folder {
			continue
		}
		items << kit.MenuItem{
			label:  move_label.replace('{folder}', f)
			glyph:  kit.icon_folder
			action: NoteAction{m, id, .move, f}
		}
	}
	items << kit.separator()
	items << kit.MenuItem{
		label:  labels[1]
		glyph:  kit.icon_trash_2
		danger: true
		action: NoteAction{m, id, .delete, ''}
	}
	return items
}

fn (mut s Shell) editor_pane(n &Notes, mut cx core.ViewCx[Shell]) core.Element {
	t := kit.theme(cx.Cx)
	if s.selected == '' {
		return ui.div().size_full().flex_row().items_center().justify_center().child(ui.text(app.t(cx.Cx,
			'note.empty')).color(t.p.text_3))
	}
	note := n.find(s.selected) or { store.Note{} }
	// A breadcrumb-like meta strip above the editor plane.
	meta := ui.div().flex_row().items_center().gap(8).px(24).h(t.panel_header_h + 4).shrink_0().border_b(1).border_color(t.p.line).text_size(t.font_size(.sm)).child(kit.icon(kit.icon_folder).size(14).color(t.p.text_3)).child(ui.text(note.folder).color(t.p.text_2)).child(ui.text('·').color(t.p.text_3)).child(ui.text(app.format_relative(app.locale(cx.Cx),
		time_of(note.modified), time.now())).color(t.p.text_3))
	return ui.div().flex_col().size_full().bg(t.p.canvas).child(meta).child(ui.div().flex_col().grow().child(cx.child_view(s.editor)))
}

enum NoteDo {
	open
	move
	delete
}

// NoteAction is what a note's context menu item does (a kit.MenuAction: no closure per row).
struct NoteAction {
	m      core.Model[Notes]
	id     string
	do     NoteDo
	folder string
}

pub fn (a NoteAction) run(mut cx core.Cx) {
	match a.do {
		.open { open_note_window(mut cx, a.m, a.id) or { kit.push_toast(mut cx, .error, err.msg()) } }
		.move { move_to(mut cx, a.m, a.id, a.folder) }
		.delete { remove(mut cx, a.m, a.id) } // the shell drops the selection when it sees the note gone
	}
}
