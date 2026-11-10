// The SSH wiring (specs/22, Remote) on fakes: the dock, the connect path with a closed local
// port, typed-error mapping, the gutter, the context menu. Data goes under .tmp/.
module main

import os
import net
import time
import sessionui
import ssh
import store
import quadra.core
import quadra.kit.term
import quadra.testing

fn wrig(name string, prep fn (mut s store.SessionStore)) !(&testing.Harness, &OneTerm, core.View[OneTerm]) {
	dir := '.tmp/sshwire_fake_${name}'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	assert one.known_hosts == os.join_path(dir, 'known_hosts') // never the real ~/.ssh
	assert one.local_home == dir
	prep(mut one.sessions)
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1200, height: 800)!
	h.frame()
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return h, cx.peek(v.model()), v
}

fn run_do(mut h testing.Harness, v core.View[OneTerm], d Do) {
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	cx.update_view(v, fn [d] (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.run(d, mut cx)
	})
	h.frame()
}

fn wait_for(mut h testing.Harness, ok fn () bool) {
	for _ in 0 .. 500 {
		h.plat.run_posted()
		h.frame()
		if ok() {
			return
		}
		time.sleep(20 * time.millisecond)
	}
}

fn test_the_dock_mounts_both_panels_and_expanding_hides_the_session_section() {
	mut h, mut o, v := wrig('dock', fn (mut _ store.SessionStore) {})!
	run_do(mut h, v, Do{.toggle_dock, ''})
	assert o.dock_open
	assert h.find('session-panel') != none
	assert h.find('sftp-panel') != none
	assert h.has_text('No SFTP connection.')
	assert h.has_text('SFTP Browser')
	h.click('session-add')!
	h.frame()
	assert h.has_text('New SSH Session')
	h.press('escape')!
	h.frame()
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	_ = cx
	h.click('sftp-expand')!
	h.frame()
	assert o.sftp_expanded // through the panel's on_expanded
	assert h.find('session-panel') == none
	assert h.find('sftp-panel') != none
}

fn test_the_plus_menu_and_keys_reach_the_session_panel() {
	mut h, mut o, v := wrig('menu', fn (mut s store.SessionStore) {
		s.add(store.Session{ label: 'db', host: '127.0.0.1', username: 'dba' })
	})!
	assert o.new_tab_items().any(it.label == 'Quick Connect...')
	assert o.new_tab_items().any(it.label == 'New Saved Session...')
	run_do(mut h, v, Do{.quick_connect, ''})
	assert h.has_text('SSH Quick Connect')
	h.press('escape')!
	h.frame()
	run_do(mut h, v, Do{.new_saved_session, ''})
	assert h.has_text('New SSH Session')
	h.press('escape')!
	h.frame()
	id := o.saved_sessions()[0].id
	run_do(mut h, v, Do{.open_saved, id})
	assert h.has_text('Connect to db (dba@127.0.0.1:22)')
}

fn test_a_refused_connection_reports_in_the_dialog() {
	// a port just closed on 127.0.0.1: the whole path through the worker and back
	mut l := net.listen_tcp(.ip, '127.0.0.1:0')!
	port := l.addr()!.port()!
	l.close()!
	mut h, mut o, v := wrig('refused', fn [port] (mut s store.SessionStore) {
		s.add(store.Session{
			label:       'closed'
			host:        '127.0.0.1'
			port:        int(port)
			username:    'nobody'
			auth_method: .agent
		})
	})!
	run_do(mut h, v, Do{.open_saved, o.saved_sessions()[0].id})
	h.click('connect-button')!
	assert o.attempts.len == 1
	wait_for(mut h, fn [o] () bool {
		return o.attempts.len == 0
	})
	assert o.attempts.len == 0
	h.frame()
	assert h.has_text('Connect to closed (nobody@127.0.0.1:${port})') // the dialog stays open
	assert h.texts().any(it.contains('SSH connect failed'))
	assert o.tabs.len == 1 // no tab appeared
}

fn test_cancel_ends_the_attempt_and_drops_the_late_result() {
	mut h, mut o, _ := wrig('cancel', fn (mut _ store.SessionStore) {})!
	mut c := ssh.new_cancel()
	o.attempts[7] = c
	o.on_cancel(7)
	assert c.is_cancelled()
	assert 7 !in o.attempts
	h.frame()
}

fn test_empty_password_is_auth_none_and_keys_keep_their_passphrase_out() {
	none_auth := auth_of(sessionui.Credential{ method: .password })
	assert none_auth is ssh.NoAuth
	assert auth_of(sessionui.Credential{ method: .password, password: 'pw' }) is ssh.PasswordAuth
	req := sessionui.ConnectRequest{
		label:      'x'
		host:       'h'
		port:       22
		username:   'u'
		credential: sessionui.Credential{
			method:     .private_key
			key_path:   'k'
			passphrase: 'secret-passphrase'
		}
	}
	meta := meta_of(req)
	assert meta.key_path == 'k'
	assert !'${meta}'.contains('secret-passphrase')
}

fn test_failure_messages_name_themselves_once() {
	auth, is_auth := failure_message(IError(ssh.ConnectError{
		phase:   .authentication
		message: 'rejected'
	}))
	assert auth == 'SSH authentication failed: rejected'
	assert is_auth
	hop, _ := failure_message(IError(ssh.ConnectError{
		phase:   .authentication
		message: 'jump host a@b:22: rejected'
	}))
	assert hop == 'jump host a@b:22: rejected' // the panel blames the row from this prefix
	other, _ := failure_message(error('boom'))
	assert other == 'SSH connect failed: boom'
}

fn test_the_gutter_labels_each_row_and_keeps_its_stamps() {
	mut h, mut o, v := wrig('gutter', fn (mut _ store.SessionStore) {})!
	s := o.current().active_space() or { panic('no space') }
	mut t := core.Cx{
		app: unsafe { h.app }
	}.peek(s.view.model())
	t.feed('one\r\ntwo\r\n'.bytes())
	mut g := Gutter{}
	g.stamp(t, 'one\r\ntwo\r\n'.bytes(), 3661)
	clock, num := g.label(g.absolute(t), 0, t.screen.rows, 1)
	assert clock == '[01:01:01]'
	assert num.trim_space() == '2'
	run_do(mut h, v, Do{.toggle_gutter, ''})
	h.frame()
	h.frame() // the first frame learns the cell width
	assert o.cfg.layout.show_gutter
	assert h.find('${t.id}-gutter') != none
	assert t.b.padding.left > o.cfg.layout.padding.left
	run_do(mut h, v, Do{.toggle_gutter, ''})
	h.frame()
	assert t.b.padding.left == o.cfg.layout.padding.left
}

@[heap]
struct Rows {
mut:
	labels []string
}

fn test_close_space_is_a_noop_alone_and_the_menu_says_so() {
	mut h, mut o, v := wrig('close', fn (mut _ store.SessionStore) {})!
	run_do(mut h, v, Do{.close_space, ''})
	assert o.tabs.len == 1 && o.current().spaces.len == 1 && o.current().spaces[0].view.id != 0
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	mut rows := &Rows{}
	cx.update_view(v, fn [mut rows] (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		tab := o.current()
		rows.labels = o.space_menu(tab, tab.spaces[0], mut cx).map(it.label)
	})
	assert 'Duplicate Session' in rows.labels && 'Close Space' !in rows.labels
}

fn test_duplicating_a_local_terminal_into_a_split_starts_it_there() {
	mut h, mut o, v := wrig('dup', fn (mut _ store.SessionStore) {})!
	run_do(mut h, v, Do{.duplicate_to, 'right'})
	tab := o.current()
	assert tab.spaces.len == 2
	assert tab.spaces.all(it.view.id != 0)
	h.frame()
}

fn test_the_session_dialog_inputs_are_one_control_height() {
	mut h, _, v := wrig('heights', fn (mut _ store.SessionStore) {})!
	run_do(mut h, v, Do{.toggle_dock, ''})
	h.click('session-add')!
	h.frame()
	for id in ['session-label', 'session-host', 'session-port', 'session-username',
		'session-auth-password'] {
		r := h.find(id) or { panic('no ${id}') }
		assert r.height == 32, id // the original's medium input, as the buttons
	}
	h.press('escape')!
	h.frame()
	run_do(mut h, v, Do{.quick_connect, ''})
	for id in ['quick-host', 'quick-port', 'quick-username', 'connect-auth-password'] {
		r := h.find(id) or { panic('no ${id}') }
		assert r.height == 32, id
	}
}

// The expanded browser covers the terminals and the dock (specs/22, a departure the user
// chose); collapsing brings the dock back at the width it had.
fn test_the_expanded_sftp_browser_covers_the_workspace() {
	mut h, _, v := wrig('zoom', fn (mut _ store.SessionStore) {})!
	run_do(mut h, v, Do{.toggle_dock, ''})
	dock := h.find('right-dock') or { panic('no dock') }
	term_area := h.find('terminal-area') or { panic('no terminal area') }
	h.click('sftp-expand')!
	h.frame()
	full := h.find('right-dock') or { panic('no dock') }
	assert full.x == term_area.x && full.y == term_area.y
	assert full.x + full.width == dock.x + dock.width && full.height == dock.height
	assert h.find('terminal-area') == none && h.find('dock-handle') == none
	panel := h.find('sftp-panel') or { panic('no browser') }
	assert panel.x <= term_area.x + 1 && panel.width >= term_area.width + dock.width - 2
	h.click('sftp-expand')!
	h.frame()
	back := h.find('right-dock') or { panic('no dock') }
	assert back == dock
	assert h.find('terminal-area') or { panic('no terminal area') } == term_area
}
