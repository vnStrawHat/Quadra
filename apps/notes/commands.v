// Keyboard shortcuts and the command palette (ctrl-p).
module main

import quadra.app
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub struct NewNote {}

pub struct DeleteNote {}

pub struct FocusSearch {}

pub struct OpenPalette {}

pub struct ShowPage {
	page Page
}

pub fn bind_keys(mut a core.App) ! {
	a.bind('ctrl-n', NewNote{})!
	a.bind('ctrl-shift-backspace', DeleteNote{})!
	a.bind('ctrl-f', FocusSearch{})!
	a.bind('ctrl-p', OpenPalette{})!
	a.bind('ctrl-1', ShowPage{.notes})!
	a.bind('ctrl-2', ShowPage{.table})!
	a.bind('ctrl-,', ShowPage{.settings})!
	app.bind_navigation(mut a)!
}

// with_actions attaches the shell's action handlers to its root element.
fn (mut s Shell) with_actions(root &ui.Div, mut cx core.ViewCx[Shell]) &ui.Div {
	return root.on_action(cx.listener(fn (mut s Shell, _ &NewNote, mut cx core.ViewCx[Shell]) {
		s.new_note(mut cx)
	})).on_action(cx.listener(fn (mut s Shell, _ &DeleteNote, mut cx core.ViewCx[Shell]) {
		s.delete_selected(mut cx)
	})).on_action(cx.listener(fn (mut s Shell, _ &FocusSearch, mut cx core.ViewCx[Shell]) {
		s.go(.notes)
		cx.peek(s.search.model()).focus.focus(mut cx.Cx)
	})).on_action(cx.listener(fn (mut s Shell, e &ShowPage, mut cx core.ViewCx[Shell]) {
		s.show(e.page, mut cx)
	})).on_action(cx.listener(fn (mut s Shell, _ &app.NavigateBack, mut cx core.ViewCx[Shell]) {
		if s.router.back() {
			s.refresh_table(mut cx)
		}
	})).on_action(cx.listener(fn (mut s Shell, _ &app.NavigateForward, mut cx core.ViewCx[Shell]) {
		if s.router.forward() {
			s.refresh_table(mut cx)
		}
	})).on_action(cx.listener(fn (mut s Shell, _ &OpenPalette, mut cx core.ViewCx[Shell]) {
		input.open_command_palette(mut cx.Cx, s.commands(mut cx))
	}))
}

// show switches page, refreshing the table when it is shown.
fn (mut s Shell) show(p Page, mut cx core.ViewCx[Shell]) {
	s.go(p)
	if p == .table {
		s.refresh_table(mut cx)
	}
}

// commands lists the palette entries; each one updates the shell through its view handle.
fn (mut s Shell) commands(mut cx core.ViewCx[Shell]) []input.Command {
	me := cx.handle()
	m := s.notes
	prefs := s.prefs
	mut cmds := [
		input.Command{app.t(cx.Cx, 'note.new'), 'ctrl-n', fn [me] (mut cx core.Cx) {
			cx.update_view(me, fn (mut s Shell, mut cx core.ViewCx[Shell]) {
				s.new_note(mut cx)
			})
		}},
		input.Command{app.t(cx.Cx, 'cmd.search'), 'ctrl-f', fn [me] (mut cx core.Cx) {
			cx.update_view(me, fn (mut s Shell, mut cx core.ViewCx[Shell]) {
				s.go(.notes)
				cx.peek(s.search.model()).focus.focus(mut cx.Cx)
			})
		}},
		input.Command{app.t(cx.Cx, 'cmd.theme'), '', fn [prefs] (mut cx core.Cx) {
			dark := kit.theme_of(cx.app).mode == .dark
			update_prefs(mut cx, prefs, fn [dark] (mut p Prefs, mut _ core.ViewCx[Prefs]) {
				p.theme = if dark { 'light' } else { 'dark' }
			})
		}},
	]
	for p in [Page.notes, .table, .settings] {
		key := ['nav.notes', 'nav.table', 'nav.settings'][int(p)]
		cmds << input.Command{app.t(cx.Cx, key), '', fn [me, p] (mut cx core.Cx) {
			cx.update_view(me, fn [p] (mut s Shell, mut cx core.ViewCx[Shell]) {
				s.show(p, mut cx)
			})
		}}
	}
	if s.selected != '' {
		id := s.selected
		cmds << input.Command{app.t(cx.Cx, 'note.open_window'), '', fn [m, id] (mut cx core.Cx) {
			open_note_window(mut cx, m, id) or { kit.push_toast(mut cx, .error, err.msg()) }
		}}
		cmds << input.Command{app.t(cx.Cx, 'note.delete'), 'ctrl-shift-backspace', fn [me] (mut cx core.Cx) {
			cx.update_view(me, fn (mut s Shell, mut cx core.ViewCx[Shell]) {
				s.delete_selected(mut cx)
			})
		}}
	}
	return cmds
}

// MenuCmd is a titlebar menu entry: it dispatches the action its keys are bound to, so the
// menu needs no closures.
struct MenuCmd {
	act int
}

fn (m MenuCmd) run(mut cx core.Cx) {
	match m.act {
		0 { cx.dispatch(NewNote{}) }
		1 { cx.dispatch(FocusSearch{}) }
		2 { cx.dispatch(OpenPalette{}) }
		3 { cx.dispatch(ShowPage{.notes}) }
		4 { cx.dispatch(ShowPage{.table}) }
		else { cx.dispatch(ShowPage{.settings}) }
	}
}

// app_menu is the titlebar's hamburger menu: the main commands with their keys.
fn app_menu(mut cx core.ViewCx[Shell]) []kit.MenuItem {
	mut items := []kit.MenuItem{}
	for i, e in [['note.new', 'ctrl-n'], ['cmd.search', 'ctrl-f'],
		['cmd.palette', 'ctrl-p'], ['-', ''], ['nav.notes', 'ctrl-1'],
		['nav.table', 'ctrl-2'], ['nav.settings', 'ctrl-,']] {
		if e[0] == '-' {
			items << kit.separator()
			continue
		}
		items << kit.MenuItem{
			label:    app.t(cx.Cx, e[0])
			shortcut: e[1]
			action:   MenuCmd{if i > 3 { i - 1 } else { i }}
		}
	}
	return items
}
