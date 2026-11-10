// SftpPanel: the SFTP Browser of the right dock (specs/27). One panel for the whole app; it
// shows the active session's state.
module sftpui

import os
import ssh
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.platform

// EditSettings mirrors the app's SFTP settings for the Edit workflow.
pub struct EditSettings {
pub:
	mode    string = 'os_default' // or 'custom'
	program string
	args    []string
	limit   i64 = 1048576 // bytes; 0 = no limit
}

pub type LaunchFn = fn (program string, args []string, path string) !

@[params]
pub struct PanelOptions {
pub:
	edit_dir    string // working copies of Edit, under the app's config folder
	state_path  string // the app's docks.json; '' keeps nothing
	local_dir   string // first folder of the Local pane; '' = home
	editor      EditSettings
	launch      ?LaunchFn // runs a custom editor; tests inject one
	on_expanded ?core.Listener[bool]
}

@[heap]
pub struct SftpPanel {
pub mut:
	expanded bool
	shortcut map[string]string // action key -> shown shortcut
mut:
	view        core.View[SftpPanel]
	states      map[u64]&Backend
	active      &Backend = unsafe { nil } // nil: no SFTP connection
	term_cwd    string
	table       core.View[FileTable]
	path        core.View[input.TextInput]
	bus         &Bus = &Bus{}
	timer       platform.TimerId
	ticking     bool
	drop_bound  bool
	win         &core.Window = unsafe { nil } // for overlays opened from tasks and timers
	edits       map[u64]&EditSession
	next_edit   u64
	settings    EditSettings
	launch      ?LaunchFn
	edit_dir    string
	edit_lock   ?EditLock
	state_path  string
	on_expanded ?core.AnyListener
	form_modal  &ModalRef = unsafe { nil }
	local       LocalState
	split       f32 = 0.5
}

// new_panel creates the panel view; mount it with core.view_element(v.any()).
pub fn new_panel(mut cx core.Cx, opts PanelOptions) core.View[SftpPanel] {
	kit.install(mut cx.app) or {}
	input.install(mut cx.app) or {}
	mut l := ?core.AnyListener(none)
	if e := opts.on_expanded {
		l = e.any
	}
	v := cx.new_view(SftpPanel{
		settings:    opts.editor
		launch:      opts.launch
		edit_dir:    opts.edit_dir
		state_path:  opts.state_path
		on_expanded: l
		local:       LocalState{
			start: if opts.local_dir != '' { opts.local_dir } else { os.home_dir() }
		}
	})
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.view = cx.handle()
		p.bus.inbox = core.new_inbox[SftpPanel](mut cx, on_bus)
		p.table = new_table(mut cx.Cx,
			id:        'sftp-table'
			cols:      remote_columns()
			drag_kind: 'sftp-remote'
			drop_kind: 'sftp-local'
			on_event:  cx.listener(on_remote_table)
		)
		p.path = input.new_input(mut cx.Cx, id: 'sftp-path', bare: true, placeholder: 'Path')
		input.on_submit(mut cx.Cx, p.path, cx.listener(on_path_submit))
		input.on_change(mut cx.Cx, p.path, cx.listener(on_path_change))
		p.init_local(mut cx)
		p.load_state(mut cx)
		p.sweep_edit_cache()
	})
	return v
}

// set_session shows this session's browser (a new session opens at its home folder); none
// shows "No SFTP connection.".
pub fn set_session(mut cx core.Cx, v core.View[SftpPanel], s ?&ssh.Sftp) {
	if sess := s {
		set_client(mut cx, v, Client(&SftpClient{
			s: sess
		}), u64(voidptr(sess)))
	} else {
		clear_client(mut cx, v)
	}
}

// set_client is set_session for any Client; key identifies the session across calls.
pub fn set_client(mut cx core.Cx, v core.View[SftpPanel], c Client, key u64) {
	cx.update_view_with(v, ClientKey{c, key}, fn (mut p SftpPanel, k ClientKey, mut cx core.ViewCx[SftpPanel]) {
		p.attach(k.c, k.key, mut cx)
	})
}

struct ClientKey {
	c   Client
	key u64
}

pub fn clear_client(mut cx core.Cx, v core.View[SftpPanel]) {
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.active = unsafe { nil }
		p.reap(mut cx)
		cx.notify()
	})
}

// follow_cwd is the active terminal's working folder from OSC 7 ('' = unknown): the sync
// button jumps to it, and the Follow toggle goes there whenever it changes.
pub fn follow_cwd(mut cx core.Cx, v core.View[SftpPanel], path string) {
	cx.update_view_with(v, path, fn (mut p SftpPanel, path string, mut cx core.ViewCx[SftpPanel]) {
		p.term_cwd = path
		p.maybe_follow(mut cx)
		cx.notify()
	})
}

pub fn set_edit_settings(mut cx core.Cx, v core.View[SftpPanel], s EditSettings) {
	cx.update_view_with(v, s, fn (mut p SftpPanel, s EditSettings, mut cx core.ViewCx[SftpPanel]) {
		p.settings = s
	})
}

// set_shortcuts fills the menus' shortcut column from the app's keymap (action key -> keys).
pub fn set_shortcuts(mut cx core.Cx, v core.View[SftpPanel], m map[string]string) {
	cx.update_view_with(v, Shortcuts{m}, fn (mut p SftpPanel, s Shortcuts, mut cx core.ViewCx[SftpPanel]) {
		p.shortcut = s.m.clone()
	})
}

struct Shortcuts {
	m map[string]string
}

// run_action runs a browser action, for the app's key bindings.
pub fn run_action(mut cx core.Cx, v core.View[SftpPanel], a SftpAction) {
	cx.update_view_with(v, a, fn (mut p SftpPanel, a SftpAction, mut cx core.ViewCx[SftpPanel]) {
		p.run(a, mut cx)
	})
}

pub fn set_expanded(mut cx core.Cx, v core.View[SftpPanel], on bool) {
	cx.update_view_with(v, on, fn (mut p SftpPanel, on bool, mut cx core.ViewCx[SftpPanel]) {
		p.set_expanded(on, mut cx)
	})
}

// dispose removes every working copy and stops the timer (app exit).
pub fn dispose(mut cx core.Cx, v core.View[SftpPanel]) {
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		for id, _ in p.edits {
			p.end_edit(id)
		}
		if p.ticking {
			cx.cancel_timer(p.timer)
			p.ticking = false
		}
	})
}

fn (mut p SftpPanel) attach(c Client, key u64, mut cx core.ViewCx[SftpPanel]) {
	mut st := p.states[key] or {
		fresh := &Backend{
			key:    key
			client: c
		}
		p.states[key] = fresh
		fresh
	}
	st.client = c
	p.active = st
	p.ensure_tick(mut cx)
	if st.cwd == '' {
		p.load_dir('.', mut cx)
	} else {
		p.sync_table(mut cx)
		p.sync_path(mut cx)
		cx.notify()
	}
}

fn (mut p SftpPanel) set_expanded(on bool, mut cx core.ViewCx[SftpPanel]) {
	if p.expanded == on {
		return
	}
	p.expanded = on
	if on {
		p.local_ensure_loaded(mut cx)
	}
	p.schedule_save(mut cx)
	if l := p.on_expanded {
		cx.app.invoke(l, &on, cx.win, true)
	}
	cx.notify()
}

// ensure_tick starts the 500 ms tick; it ends itself with no backend and no edit session.
fn (mut p SftpPanel) ensure_tick(mut cx core.ViewCx[SftpPanel]) {
	if p.ticking {
		return
	}
	p.ticking = true
	p.timer = cx.set_interval(500, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.on_tick(mut cx)
	})
}

fn (mut p SftpPanel) on_tick(mut cx core.ViewCx[SftpPanel]) {
	p.sync_path(mut cx)
	p.poll_edits(mut cx)
	p.reap(mut cx)
	if p.active == unsafe { nil } && p.edits.len == 0 {
		cx.cancel_timer(p.timer)
		p.ticking = false
	}
}

// reap forgets closed sessions and ends their edit sessions.
fn (mut p SftpPanel) reap(mut cx core.ViewCx[SftpPanel]) {
	mut dead := []u64{}
	for key, _ in p.states {
		mut st := p.states[key] or { continue }
		if !st.client.alive() {
			dead << key
		}
	}
	for key in dead {
		p.end_edits_of(key)
		if p.active != unsafe { nil } && p.active.key == key {
			p.active = unsafe { nil }
			cx.notify()
		}
		p.states.delete(key)
	}
}

// sync_path mirrors the cwd into the path box unless the user is typing in it.
fn (mut p SftpPanel) sync_path(mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	mut w := cx.peek(p.path.model())
	if !w.focus.is_focused(cx.Cx) && w.value() != p.active.cwd {
		input.set_value(mut cx.Cx, p.path, p.active.cwd)
	}
	p.local_sync_path(mut cx)
}

fn on_bus(mut p SftpPanel, _ &core.Chunk, mut cx core.ViewCx[SftpPanel]) {
	for m in p.bus.take() {
		if mut st := p.states[m.key] {
			st.apply(m)
		}
	}
	cx.notify()
}

// bind gives a window to a context that has none: task completions and timers run without one,
// and overlays (notices, modals) belong to a window.
fn (p &SftpPanel) bind(mut cx core.ViewCx[SftpPanel]) {
	if cx.win == unsafe { nil } {
		cx.win = p.win
	}
}

fn (mut p SftpPanel) notice(kind kit.ToastKind, msg string, mut cx core.ViewCx[SftpPanel]) {
	p.bind(mut cx)
	kit.notify(mut cx.Cx, kind, msg, '', timeout_ms: 5000)
}
