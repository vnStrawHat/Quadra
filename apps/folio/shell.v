// The window (specs/21, Window): titlebar, the outline panel on the left, then the tab bar,
// the toolbar and the editor and/or preview of the active tab, and the status bar.
module main

import os
import quadra.app
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.kit.md
import quadra.kit.work
import quadra.platform
import quadra.ui

pub struct Folio {
pub mut:
	prefs   app.Settings[Prefs]
	windows app.Settings[app.WindowStates]
	tabs    []&Tab
	active  int = -1
	next_id int
	pending []string // files to open at the first render
	ready   bool
	fresh   bool                       // a new window: no session to restore
	filter  core.View[input.TextInput] // the outline's search field
	query   string
}

// dirty: some tab has unsaved changes.
pub fn (f &Folio) dirty() bool {
	return f.tabs.any(it.dirty())
}

@[markused]
pub fn (mut f Folio) render(mut cx core.ViewCx[Folio]) core.Element {
	if !f.ready {
		f.setup(mut cx)
	}
	t := kit.theme(cx.Cx)
	p := f.prefs.get(cx.Cx)
	lay := layout_of(p.layout)
	mut body := ui.div().flex_row().grow().overflow_hidden()
	if f.tabs.len > 0 {
		mut tab := f.cur()
		tab.heads = md.outline(md.parse(tab.text))
		if p.outline {
			body.child(f.outline_panel(p, t, mut cx))
		}
		mut center := ui.div().flex_col().grow().h_full().min_w(0)
		center.child(f.tab_bar(mut cx)).child(f.toolbar(lay, t, mut cx))
		if tab.outside {
			center.child(f.outside_banner(t, mut cx))
		}
		body.child(center.child(f.main_area(lay, p, t, mut cx)))
	} else {
		body.child(f.empty_state(p, t, mut cx))
	}
	root := ui.div().flex_col().size_full().bg(t.p.canvas).text_color(t.p.text).font(t.font_family).text_size(t.font_size(.md)).child(f.titlebar(mut cx)).child(body).child(f.status_bar(lay,
		p, mut cx))
	return f.with_actions(root, mut cx)
}

fn (mut f Folio) setup(mut cx core.ViewCx[Folio]) {
	f.ready = true
	f.filter = input.new_input(mut cx.Cx,
		id:          'outline-filter'
		placeholder: 'Search outline…'
		bare:        true
	)
	input.on_change(mut cx.Cx, f.filter, cx.listener(fn (mut f Folio, q &string, mut _ core.ViewCx[Folio]) {
		f.query = (*q).to_lower()
	}))
	cx.on_close_requested(cx.listener(fn (mut f Folio, _ &core.CloseRequest, mut cx core.ViewCx[Folio]) {
		f.close_requested(mut cx)
	}))
	cx.on_files_dropped(cx.listener(fn (mut f Folio, e &platform.FileDropEvent, mut cx core.ViewCx[Folio]) {
		for path in e.paths {
			f.open_path(path, mut cx)
		}
	}))
	cx.set_interval(2000, fn (mut f Folio, mut cx core.ViewCx[Folio]) {
		f.check_outside(mut cx)
	})
	if f.pending.len > 0 {
		for path in f.pending {
			f.open_path(path, mut cx)
		}
		f.pending = []
	} else if !f.fresh {
		f.restore(mut cx)
	}
}

// titlebar: the app menu and the folder of the shown file, as a project name.
fn (mut f Folio) titlebar(mut cx core.ViewCx[Folio]) core.Element {
	folder := if f.tabs.len > 0 && f.cur().path != '' {
		os.file_name(os.dir(f.cur().path))
	} else {
		''
	}
	return kit.titlebar(folder).menu(f.menu_items(mut cx)).controls([
		ui.div().grow(),
	])
}

// tab_bar lists the open documents: a dot marks unsaved changes, the x closes (middle
// click too), dragging reorders, + starts a new document.
fn (mut f Folio) tab_bar(mut cx core.ViewCx[Folio]) core.Element {
	tabs := f.tabs.map(work.Tab{
		id:    it.id.str()
		title: it.name()
		glyph: kit.icon_file_text
		dirty: it.dirty()
	})
	add := kit.icon_button('tab-new', kit.icon_plus).ghost().size(.sm).tooltip_keys('New document',
		'ctrl-n').on_click(cx.listener(fn (mut f Folio, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
		f.new_doc(mut cx)
	}))
	return work.tab_strip('tabs', tabs, f.cur().id.str()).actions([
		core.Element(add),
	]).on_select(cx.listener(fn (mut f Folio, id &string, mut cx core.ViewCx[Folio]) {
		f.activate(f.index_of(id.int()), mut cx)
	})).on_close(cx.listener(fn (mut f Folio, id &string, mut cx core.ViewCx[Folio]) {
		f.close_tab(id.int(), mut cx)
	})).on_move(cx.listener(fn (mut f Folio, m &work.TabMove, mut cx core.ViewCx[Folio]) {
		f.move_tab(m.id.int(), m.to, mut cx)
	}))
}

// status_bar: the outline toggle on the left; the cursor, the word count and the language
// on the right.
fn (mut f Folio) status_bar(lay Layout, p &Prefs, mut cx core.ViewCx[Folio]) core.Element {
	toggle := kit.status_button('outline-toggle', kit.icon_list_tree, p.outline).tooltip_keys('Outline',
		'ctrl-shift-o').on_click(cx.listener(fn (mut f Folio, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
		f.toggle_outline(mut cx)
	}))
	if f.tabs.len == 0 {
		return kit.statusbar([core.Element(toggle)], [])
	}
	tab := f.cur()
	ed := cx.read_view(tab.editor)
	c := ed.doc.primary()
	line := ed.doc.buf.line_of(c.head)
	col := c.head - ed.doc.buf.line_start(line) + 1
	n := md.words(tab.text)
	mut right := []core.Element{}
	if lay != .preview {
		right << ui.text('${line + 1}:${col}')
	}
	right << ui.text('${n} words, ${reading_minutes(n)} min')
	right << ui.text('Markdown')
	return kit.statusbar([core.Element(toggle)], right)
}

fn (mut f Folio) outside_banner(t &kit.Theme, mut cx core.ViewCx[Folio]) core.Element {
	reload := kit.button('outside-reload').label('Reload').size(.sm).on_click(cx.listener(fn (mut f Folio, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
		f.reload(mut f.cur(), mut cx)
	}))
	keep := kit.button('outside-keep').label('Keep mine').ghost().size(.sm).on_click(cx.listener(fn (mut f Folio, _ &core.ClickEvent, mut _ core.ViewCx[Folio]) {
		mut tab := f.cur()
		tab.outside = false
		tab.mtime = os.file_last_mod_unix(tab.path)
	}))
	msg :=
		ui.text('${f.cur().name()} changed on disk, and you have unsaved changes here.').color(t.p.text)
	return ui.div().test_id('outside-banner').flex_row().items_center().gap(10).px(12).h(36).shrink_0().bg(t.p.warn.alpha(0.14)).border_b(1).border_color(t.p.line).child(kit.icon(kit.icon_triangle_alert).size(15).color(t.p.warn)).child(msg).child(ui.div().grow()).child(keep).child(reload)
}

// prefs_with changes the settings with data, without a closure (specs/14).
fn (f &Folio) prefs_with[D](mut cx core.Cx, data D, fun fn (mut p Prefs, d D, mut cx core.ViewCx[Prefs])) {
	app.update_with(mut cx, f.prefs, data, fun)
}

fn add_recent(mut p Prefs, path string, mut _ core.ViewCx[Prefs]) {
	p.recent = with_recent(p.recent, path)
}
