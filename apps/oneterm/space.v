// A Space's element: the terminal or the empty placeholder, its ring and context menu.
module main

import quadra.core
import quadra.kit
import quadra.kit.work
import quadra.ui

fn (o &OneTerm) space_el(tab &TermTab, s &Space, t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	content := if s.view.id != 0 {
		core.Element(cx.child_view(s.view))
	} else {
		o.empty_space(s, t, mut cx)
	}
	items := if o.cfg.mouse.show_context_menu {
		o.space_menu(tab, s, mut cx)
	} else {
		[]kit.MenuItem{}
	}
	mut box := kit.context_menu(content, items).test_id('space-${s.id}').size_full().on_mouse_down(core.listener_with[OneTerm, string, core.MouseDownEvent](cx.entity,
		s.id, fn (mut o OneTerm, id string, _ &core.MouseDownEvent, mut cx core.ViewCx[OneTerm]) {
		o.activate(id, mut cx)
	}))
	if s.channel != '' {
		box.child(channel_badge(s.channel, t))
	}
	if bar := progress_bar(s.progress, t) {
		box.child(bar)
	}
	if tab.spaces.len > 1 && is_recording(s) {
		box.child(ui.div().absolute().top(4).right(8).test_id('recording-${s.id}').child(ui.text('●').size(12).color(t.p.error)))
	}
	if s.ssh_closed {
		box.child(ui.div().absolute().bottom(0).left(0).right(0).flex_col().test_id('ssh-closed-${s.id}').child(kit.alert(.warning,
			'SSH connection closed. Input is disabled.', '')))
	}
	if tab.spaces.len == 1 {
		return box // a lone Space has no chrome
	}
	if s.id == tab.active {
		ring := if s.channel != '' { channel_color(s.channel, t) } else { t.p.accent }
		box.border(2).border_color(ring)
	} else {
		box.border(1).border_color(t.p.line).p(1)
	}
	return box
}

// split_rows are the four Split items, disabled once the session is gone.
fn split_rows(dead bool) []kit.MenuItem {
	return [
		kit.MenuItem{
			label:    'Split Right'
			disabled: dead
			action:   Do{.split_right, ''}
		},
		kit.MenuItem{
			label:    'Split Left'
			disabled: dead
			action:   Do{.split_left, ''}
		},
		kit.MenuItem{
			label:    'Split Up'
			disabled: dead
			action:   Do{.split_up, ''}
		},
		kit.MenuItem{
			label:    'Split Down'
			disabled: dead
			action:   Do{.split_down, ''}
		},
	]
}

fn duplicate_menu(tab &TermTab, dead bool) kit.MenuItem {
	if dead {
		return kit.MenuItem{
			label:    'Duplicate Session'
			disabled: true
		}
	}
	mut rows := [item('In New Tab', .duplicate_to, 'new')]
	for e in tab.spaces.filter(it.view.id == 0) {
		rows << item('Into Space #${e.number}', .duplicate_to, 'space:${e.id}')
	}
	rows << kit.separator()
	rows << item('Split Right', .duplicate_to, 'right')
	rows << item('Split Down', .duplicate_to, 'down')
	return kit.submenu('Duplicate Session', rows)
}

// space_menu is the context menu of a Space, in the original's order. Close Space needs a sibling.
fn (o &OneTerm) space_menu(tab &TermTab, s &Space, mut cx core.ViewCx[OneTerm]) []kit.MenuItem {
	if s.view.id == 0 {
		mut rows := [item('New Terminal Here', .new_here, '')]
		rows << split_rows(false)
		rows << kit.separator()
		rows << item('Close Terminal Tab', .close_panel, '')
		if tab.spaces.len > 1 {
			rows << item('Close Space', .close_space, '')
		}
		return rows
	}
	dead := !alive(s, &cx.Cx) || s.ssh_closed
	mut rows := [item('New Terminal', .new_here, ''), duplicate_menu(tab, dead),
		kit.separator()]
	rows << split_rows(dead)
	rows << kit.separator()
	rows << kit.MenuItem{
		label:    'Copy'
		disabled: !cx.peek(s.view.model()).screen.sel.active
		action:   Do{.copy, ''}
	}
	rows << item('Paste', .paste, '')
	rows << item('Select All', .select_all, '')
	rows << item('Clear', .clear, '')
	rows << kit.separator()
	rows << log_menu(s)
	rows << kit.separator()
	rows << item('Close Terminal Tab', .close_panel, '')
	if tab.spaces.len > 1 {
		rows << item('Close Space', .close_space, '')
	}
	return rows
}

// empty_space is a Space without a terminal: it accepts a dragged terminal tab.
fn (o &OneTerm) empty_space(s &Space, t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	return ui.div().id('empty-${s.id}').test_id('empty-${s.id}').size_full().center().flex_col().gap(8).track_focus(s.focus).child(ui.div().test_id('empty-icon-${s.id}').child(kit.icon(icon_terminal_logo).size(24).color(brand_accent))).child(ui.text('Space #${s.number}').color(t.p.text_2)).child(ui.text('Right-click → New Terminal Here').color(t.p.text_3)).child(ui.text('or split, or drag a terminal tab here').color(t.p.text_3)).on_drop(work.tab_drag_kind, core.listener_with[OneTerm, string, core.DropEvent](cx.entity,
		s.id, fn (mut o OneTerm, id string, e &core.DropEvent, mut cx core.ViewCx[OneTerm]) {
		o.move_into_space(work.tab_drag_id(e.payload), id, mut cx)
	})).drag_over(ui.Patch{
		background: t.p.accent_tint
	})
}
