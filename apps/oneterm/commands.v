// Commands: one enum behind the keys, the menus and the tab context menu (specs/22).
module main

import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.ui

pub enum Cmd {
	new_default
	new_shell
	new_here
	close_panel
	close_space
	split_right
	split_left
	split_up
	split_down
	rename_tab
	duplicate_tab
	close_tab
	close_others
	close_right
	duplicate_space
	clear
	copy
	paste
	select_all
	dark
	light
	theme
	find
	open_settings
	about
	quit
	lock
	toggle_dock
	toggle_fps
	toggle_rail_collapsed
	join_channel
	leave_channel
	close_channel
	join_all
	leave_all
	new_admin
	log_start
	log_stop
	quick_connect
	new_saved_session
	open_saved
	open_selected
	delete_selected
	properties_selected
	sftp
	toggle_gutter
	duplicate_to
}

// Do is the action behind every command; arg is a tab id or a shell kind.
pub struct Do {
pub:
	cmd Cmd
	arg string
}

// run makes Do a menu action: the menu dispatches it into the window.
pub fn (d Do) run(mut cx core.Cx) {
	cx.dispatch(d)
}

fn (mut o OneTerm) with_actions(root &ui.Div, mut cx core.ViewCx[OneTerm]) &ui.Div {
	return root.on_action(cx.listener(fn (mut o OneTerm, d &Do, mut cx core.ViewCx[OneTerm]) {
		o.run(*d, mut cx)
	}))
}

pub fn (mut o OneTerm) run(d Do, mut cx core.ViewCx[OneTerm]) {
	match d.cmd {
		.new_default { o.new_tab('', '', mut cx) }
		.new_shell { o.new_tab(d.arg, '', mut cx) }
		.new_here { o.new_here(mut cx) }
		.close_panel { o.close_tab(o.active, mut cx) }
		.close_space { o.close_space(mut cx) }
		.split_right { o.split(.row, false, mut cx) }
		.split_left { o.split(.row, true, mut cx) }
		.split_up { o.split(.col, true, mut cx) }
		.split_down { o.split(.col, false, mut cx) }
		.rename_tab { o.ask_rename(d.arg, mut cx) }
		.duplicate_tab { o.duplicate(d.arg, 'new', mut cx) }
		.close_tab { o.close_tab(d.arg, mut cx) }
		.close_others { o.close_others(d.arg, mut cx) }
		.close_right { o.close_right(d.arg, mut cx) }
		.duplicate_space { o.duplicate(o.active, 'new', mut cx) }
		.duplicate_to { o.duplicate(o.active, d.arg, mut cx) }
		.clear { o.clear(mut cx) }
		.copy { o.with_terminal(mut cx, .copy) }
		.paste { o.with_terminal(mut cx, .paste) }
		.select_all { o.with_terminal(mut cx, .select_all) }
		.dark { o.use_theme(o.last_dark, mut cx) }
		.light { o.use_theme(o.last_light, mut cx) }
		.theme { o.use_theme(d.arg, mut cx) }
		.find { o.toggle_find(mut cx) }
		.open_settings { o.open_settings(mut cx) }
		.about { open_about(cx.self_model(), mut cx.Cx) }
		.quit { o.quit_app(mut cx) }
		.lock { o.lock_now(mut cx) }
		.toggle_dock { o.toggle_dock(mut cx) }
		.toggle_fps { o.toggle_fps(mut cx) }
		.toggle_rail_collapsed { o.toggle_rail_collapsed(mut cx) }
		.join_channel { o.join_channel(d.arg, mut cx) }
		.leave_channel { o.leave_channel(mut cx) }
		.close_channel { o.close_channel(mut cx) }
		.join_all { o.join_all(d.arg, mut cx) }
		.leave_all { o.leave_all(mut cx) }
		.new_admin { o.launch_admin(d.arg, mut cx) }
		.log_start { o.log_start(mut cx) }
		.log_stop { o.log_stop(mut cx) }
		.quick_connect, .new_saved_session, .open_saved { o.session_cmd(d, mut cx) }
		.open_selected, .delete_selected, .properties_selected { o.session_cmd(d, mut cx) }
		.sftp { o.sftp_cmd(d.arg, mut cx) }
		.toggle_gutter { o.toggle_gutter(mut cx) }
	}

	cx.notify()
}

enum TermOp {
	copy
	paste
	select_all
}

// active_view is the active Space's terminal model (id 0 when the Space is empty).
fn (o &OneTerm) active_view() core.Model[term.Terminal] {
	tab := o.current()
	for s in tab.spaces {
		if s.id == tab.active {
			return s.view.model()
		}
	}
	return core.Model[term.Terminal]{}
}

fn (mut o OneTerm) active_terminal(mut cx core.ViewCx[OneTerm]) ?&term.Terminal {
	tab := o.current()
	for s in tab.spaces {
		if s.id == tab.active && s.view.id != 0 {
			return cx.peek(s.view.model())
		}
	}
	return none
}

// clear erases the screen and the scrollback in the terminal (specs/22, Deviations).
fn (mut o OneTerm) clear(mut cx core.ViewCx[OneTerm]) {
	mut t := o.active_terminal(mut cx) or { return }
	t.feed('\x1b[H\x1b[2J\x1b[3J'.bytes())
}

fn (mut o OneTerm) with_terminal(mut cx core.ViewCx[OneTerm], op TermOp) {
	mut t := o.active_terminal(mut cx) or { return }
	match op {
		.copy {
			text := t.selected_text()
			if text != '' {
				cx.clipboard_write(text)
			}
		}
		.paste {
			t.paste_clipboard(mut cx.Cx)
		}
		.select_all {
			t.select_all()
		}
	}
}

fn (mut o OneTerm) use_theme(name string, mut cx core.ViewCx[OneTerm]) {
	o.ui.theme_name = name
	if t := theme_info(o.themes, name) {
		if t.light {
			o.last_light = name
		} else {
			o.last_dark = name
		}
	}
	o.apply_and_persist(mut cx.Cx)
}

fn log_menu(s &Space) kit.MenuItem {
	running := is_recording(s)
	return kit.submenu('Log', [
		kit.MenuItem{
			label:    'Start'
			disabled: running || s.view.id == 0
			action:   Do{.log_start, ''}
		},
		kit.MenuItem{
			label:    'Stop'
			disabled: !running
			action:   Do{.log_stop, ''}
		},
	])
}

fn (mut o OneTerm) log_start(mut cx core.ViewCx[OneTerm]) {
	mut s := o.current().active_space() or { return }
	if s.ext == unsafe { nil } {
		return
	}
	path := s.ext.start_log(o.log_config()) or {
		s.ext.log.take_error()
		kit.notify(mut cx.Cx, .error, 'Terminal logging', err.msg())
		return
	}
	kit.notify(mut cx.Cx, .success, 'Terminal logging', 'Logging to ${path}')
}

fn (mut o OneTerm) log_stop(mut cx core.ViewCx[OneTerm]) {
	mut s := o.current().active_space() or { return }
	if s.ext == unsafe { nil } {
		return
	}
	s.ext.log.stop() or {
		kit.notify(mut cx.Cx, .error, 'Terminal logging', err.msg())
		return
	}
	kit.notify(mut cx.Cx, .success, 'Terminal logging', 'Terminal logging stopped.')
}

// tick_logs surfaces a log failure of any terminal.
fn (mut o OneTerm) tick_logs(mut cx core.ViewCx[OneTerm]) {
	for tab in o.tabs {
		for s in tab.spaces {
			if s.ext != unsafe { nil } {
				msg := s.ext.log.take_error()
				if msg != '' {
					toast(mut cx, .error, 'Terminal logging', msg)
				}
			}
		}
	}
}

// launch_admin asks for an elevated window with this shell, off the UI thread (the consent
// prompt pumps messages); one request at a time.
fn (mut o OneTerm) launch_admin(shell string, mut cx core.ViewCx[OneTerm]) {
	if !should_start_launch(o.launching) {
		return
	}
	o.launching = true
	core.spawn_task[OneTerm, string](mut cx, fn [shell] (_ &core.TaskCx) !string {
		started, code := request_elevation(shell)
		return launch_outcome(started, code)
	}, fn (mut o OneTerm, r &core.TaskResult[string], mut cx core.ViewCx[OneTerm]) {
		o.launching = false
		if r.ok && r.value != '' {
			kit.notify(mut cx.Cx, .error, 'Run as administrator', r.value)
		}
	})
}
