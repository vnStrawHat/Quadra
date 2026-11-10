// Tab operations of the menus and key bindings: close many, duplicate, split, rename (specs/22).
module main

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.kit.work
import quadra.ui
import sessionui

// close_many closes the given tabs, asking first when there is more than one.
fn (mut o OneTerm) close_many(ids []string, mut cx core.ViewCx[OneTerm]) {
	if ids.len == 0 {
		return
	}
	if ids.len == 1 {
		o.close_tab(ids[0], mut cx)
		return
	}
	kit.confirm(mut cx.Cx, 'Close Tabs', 'Close ${ids.len} terminal tabs? Their sessions end.',
		'Close', core.listener_with[OneTerm, []string, bool](cx.entity, ids, fn (mut o OneTerm, ids []string, yes &bool, mut cx core.ViewCx[OneTerm]) {
		if *yes {
			for id in ids {
				o.close_tab(id, mut cx)
			}
		}
	}))
}

fn (mut o OneTerm) close_others(id string, mut cx core.ViewCx[OneTerm]) {
	o.close_many(o.tabs.filter(it.id != id).map(it.id), mut cx)
}

fn (mut o OneTerm) close_right(id string, mut cx core.ViewCx[OneTerm]) {
	i := o.tabs.map(it.id).index(id)
	if i >= 0 {
		o.close_many(o.tabs[i + 1..].map(it.id), mut cx)
	}
}

// duplicate re-opens the tab's active session at `dest`: 'new' (a tab), 'space:<id>' (an empty
// Space), 'right' or 'down' (a split). An SSH session goes through its duplicate dialog.
fn (mut o OneTerm) duplicate(id string, dest string, mut cx core.ViewCx[OneTerm]) {
	for tab in o.tabs {
		if tab.id != id {
			continue
		}
		for s in tab.spaces {
			if s.id != tab.active || s.view.id == 0 {
				continue
			}
			if s.ssh_closed || !alive(s, &cx.Cx) {
				return
			}
			cwd := if is_restricted() { '' } else { cx.peek(s.view.model()).screen.cwd }
			if s.remote {
				o.duplicate_ssh(s, cwd, dest, mut cx)
			} else {
				o.duplicate_local(tab, s, cwd, dest, mut cx)
			}
			cx.notify()
			return
		}
	}
}

fn (mut o OneTerm) duplicate_local(tab_ &TermTab, s &Space, cwd string, dest string, mut cx core.ViewCx[OneTerm]) {
	if dest == 'new' {
		o.new_tab(s.shell.kind, cwd, mut cx)
		return
	}
	mut tab := unsafe { tab_ }
	mut target := if dest.starts_with('space:') {
		o.empty_space_of(tab, dest.after('space:')) or { return }
	} else {
		axis, before := if dest == 'right' { work.Axis.row, false } else { work.Axis.col, false }
		o.split_space(mut tab, s.id, axis, before, mut cx)
	}
	tab.active = target.id
	o.start(mut target, s.shell.kind, cwd, mut cx)
	o.focus_active(mut cx)
}

fn (o &OneTerm) empty_space_of(tab &TermTab, id string) ?&Space {
	for s in tab.spaces {
		if s.id == id && s.view.id == 0 {
			return s
		}
	}
	return none
}

// duplicate_ssh opens the Duplicate SSH Session dialog; the tag brings the destination back.
fn (mut o OneTerm) duplicate_ssh(s &Space, cwd string, dest string, mut cx core.ViewCx[OneTerm]) {
	mut spec := s.duplicate
	spec = sessionui.DuplicateSpec{
		...spec
		initial_cwd: cwd
		tag:         '${s.id}|${dest}'
	}
	mut panel := cx.peek(o.sess_view.model())
	panel.open_duplicate(mut cx.Cx, spec)
}

// split_space adds an empty Space next to `id` and makes it active.
fn (mut o OneTerm) split_space(mut tab TermTab, id string, axis work.Axis, before bool, mut cx core.ViewCx[OneTerm]) &Space {
	s := o.new_space(mut cx)
	tab.root = tab.root.split(id, axis, s.id, before)
	tab.spaces << s
	tab.active = s.id
	return s
}

// split adds an empty Space next to the active one and makes it active.
fn (mut o OneTerm) split(axis work.Axis, before bool, mut cx core.ViewCx[OneTerm]) {
	mut tab := o.current()
	o.split_space(mut tab, tab.active, axis, before, mut cx)
	o.sync_active(mut cx)
	o.focus_active(mut cx)
	cx.notify()
}

// close_space removes the active Space; the last one closes its tab.
fn (mut o OneTerm) close_space(mut cx core.ViewCx[OneTerm]) {
	mut tab := o.current()
	if tab.spaces.len == 1 {
		return
	}
	i := tab.spaces.map(it.id).index(tab.active)
	stop(tab.spaces[i], mut cx)
	tab.root = tab.root.remove(tab.active)
	tab.spaces.delete(i)
	tab.active = tab.root.groups()[0]
	o.sync_active(mut cx)
	o.focus_active(mut cx)
	cx.notify()
}

fn (mut o OneTerm) activate(id string, mut cx core.ViewCx[OneTerm]) {
	mut tab := o.current()
	if tab.active != id {
		tab.active = id
		o.sync_active(mut cx)
		cx.notify()
	}
}

// new_here starts the default shell in the active Space when it is empty, else opens a tab.
fn (mut o OneTerm) new_here(mut cx core.ViewCx[OneTerm]) {
	mut tab := o.current()
	for mut s in tab.spaces {
		if s.id == tab.active && s.view.id == 0 {
			o.start(mut s, '', '', mut cx)
			o.sync_active(mut cx)
			o.focus_active(mut cx)
			cx.notify()
			return
		}
	}
	o.new_tab('', '', mut cx)
	cx.notify()
}

// ask_rename opens the rename dialog (440 px). An empty title keeps it open with a warning;
// Cancel never saves.
fn (mut o OneTerm) ask_rename(id string, mut cx core.ViewCx[OneTerm]) {
	mut tab := o.tabs[o.tabs.map(it.id).index(id)]
	field := input.new_input(mut cx.Cx,
		id:          'rename-input'
		value:       o.title_of(tab, &cx.Cx)
		placeholder: 'Tab title'
	)
	mut modal := &ModalRef{}
	r := Rename{tab, modal}
	input.on_submit(mut cx.Cx, field, core.listener_with[OneTerm, Rename, string](cx.entity, r, fn (mut o OneTerm, r Rename, v &string, mut cx core.ViewCx[OneTerm]) {
		o.finish_rename(r, *v, mut cx)
	}))
	save := core.listener_with[OneTerm, RenameField, core.ClickEvent](cx.entity, RenameField{r, field}, fn (mut o OneTerm, f RenameField, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
		o.finish_rename(f.rename, cx.peek(f.field.model()).value(), mut cx)
	})
	cancel := core.handler_with(ModalClose{modal}, about_close)
	th := kit.theme(cx)
	size, fg := th.font_size(.sm), th.p.text
	modal.id = kit.open_modal(mut cx.Cx, kit.ModalOptions{
		title:  'Rename Tab'
		width:  440
		body:   fn [field, size, fg] () core.Element {
			return ui.div().flex_col().gap(4).w_full().child(ui.text('Tab title').size(size).color(fg)).child(core.view_element(field.any()))
		}
		footer: fn [save, cancel] () core.Element {
			return ui.div().flex_row().justify_end().gap(8).child(kit.button('rename-cancel').label('Cancel').outline().on_click(cancel)).child(kit.button('rename-save').label('Save').primary().on_click(save))
		}
	})
	cx.peek(field.model()).focus.focus(mut cx.Cx)
}

fn (mut o OneTerm) finish_rename(r Rename, text string, mut cx core.ViewCx[OneTerm]) {
	name := text.trim_space()
	if name == '' {
		kit.notify(mut cx.Cx, .warning, 'Tab title cannot be empty.', '')
		return
	}
	mut tab := r.tab
	tab.name = name
	kit.close_modal(mut cx.Cx, r.modal.id)
	cx.notify()
}

@[heap]
struct ModalRef {
mut:
	id int
}

struct Rename {
	tab   &TermTab  = unsafe { nil }
	modal &ModalRef = unsafe { nil }
}

struct RenameField {
	rename Rename
	field  core.View[input.TextInput]
}
