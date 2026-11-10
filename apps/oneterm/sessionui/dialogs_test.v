module sessionui

import os
import quadra.core
import quadra.kit
import quadra.testing
import quadra.ui
import store
import vault

@[heap]
pub struct DRig {
mut:
	sessions &store.SessionStore = unsafe { nil }
	vault    &vault.VaultState   = unsafe { nil }
	panel    core.View[SessionPanel]
	requests []ConnectRequest
	cancels  []u64
}

@[markused]
pub fn (mut r DRig) render(mut cx core.ViewCx[DRig]) core.Element {
	if r.panel.id == 0 {
		return ui.div()
	}
	return ui.div().size_full().flex_col().child(cx.child_view(r.panel))
}

fn drig(name string, seed fn (mut s store.SessionStore)) !(&testing.Harness, &DRig) {
	dir := os.join_path('.tmp/oneterm-tests/sessionui', name)
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	kit.set_theme(mut h.app, kit.dark())
	mut sessions := store.open(dir, false)
	seed(mut sessions)
	st := vault.new_state(dir, false)
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	v := h.open(DRig{ sessions: sessions, vault: st },
		width:  1100
		height: 1500
	)!
	cx.update_view(v, fn (mut r DRig, mut cx core.ViewCx[DRig]) {
		r.panel = new_panel(mut cx.Cx, PanelOptions{
			store:      r.sessions
			vault:      r.vault
			on_connect: cx.listener(fn (mut r DRig, q &ConnectRequest, mut _ core.ViewCx[DRig]) {
				r.requests << *q
			})
			on_cancel:  cx.listener(fn (mut r DRig, a &u64, mut _ core.ViewCx[DRig]) {
				r.cancels << *a
			})
		})
	})
	h.frame()
	return h, cx.peek(v.model())
}

fn dcx(h &testing.Harness) core.Cx {
	return core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
}

fn pan(h &testing.Harness, r &DRig) &SessionPanel {
	return dcx(h).peek(r.panel.model())
}

fn root_session() store.Session {
	return store.Session{
		label:    'Prod'
		host:     '10.0.0.1'
		username: 'root'
	}
}

fn type_into(mut h testing.Harness, id string, s string) ! {
	h.click(id)!
	h.type_text(s)
	h.frame()
}

fn test_new_session_dialog_saves_a_valid_session() {
	mut h, r := drig('new', fn (mut _ store.SessionStore) {})!
	mut cx := dcx(h)
	mut p0 := pan(h, r)
	p0.open_new_session(mut cx)
	h.frame()
	assert h.has_text('New SSH Session') && h.has_text('Label') && h.has_text('Advanced')
	h.click('dialog-save')!
	h.frame()
	assert h.has_text('Label and Host are required.') && r.sessions.sessions().len == 0
	type_into(mut h, 'session-label', 'Lab')!
	type_into(mut h, 'session-host', 'lab.test')!
	type_into(mut h, 'session-username', 'dev')!
	h.click('session-group')!
	h.frame()
	type_into(mut h, 'session-group-search', 'Staging')!
	assert h.has_text('Create "Staging"')
	h.press('enter')!
	h.frame()
	h.press('ctrl-a')!
	h.click('dialog-save')!
	h.frame()
	assert r.sessions.sessions().len == 1
	s := r.sessions.sessions()[0].session
	assert s.label == 'Lab' && s.host == 'lab.test' && s.username or { '' } == 'dev'
	assert s.group or { '' } == 'Staging' && s.port == 22
	assert h.has_text('SSH session saved.') && h.has_text('Lab')
}

fn test_a_bad_forward_opens_advanced_and_is_refused() {
	mut h, r := drig('forward', fn (mut _ store.SessionStore) {})!
	mut cx := dcx(h)
	mut p0 := pan(h, r)
	p0.open_new_session(mut cx)
	h.frame()
	h.click('session-advanced')!
	h.frame()
	h.click('forward-add')!
	h.frame()
	type_into(mut h, 'session-label', 'A')!
	type_into(mut h, 'session-host', 'h')!
	h.click('session-advanced')!
	h.frame()
	assert h.find('forward-bind-1') == none
	h.click('dialog-save')!
	h.frame()
	assert h.find('forward-bind-1') != none, 'advanced reopened'
	assert h.texts().any(it.contains('listening port')) && r.sessions.sessions().len == 0
}

fn test_connect_emits_the_request_and_reports_results() {
	mut h, r := drig('connect', fn (mut s store.SessionStore) {
		s.add(root_session())
	})!
	id := r.sessions.sessions()[0].id
	row := 'session-row-session:${id}'
	h.click(row)!
	h.click(row)! // the second click makes the double click
	h.frame()
	assert h.has_text('Connect to Prod (root@10.0.0.1:22)') && h.has_text('ssh://root@10.0.0.1:22')
	type_into(mut h, 'connect-auth-password', 'hunter2')!
	h.press('enter')!
	h.frame()
	assert r.requests.len == 1 && r.requests[0].credential.password == 'hunter2'
	assert r.requests[0].host == '10.0.0.1' && r.requests[0].username == 'root'
	assert h.has_text('Connecting')
	attempt := r.requests[0].attempt
	mut cx := dcx(h)
	mut p1 := pan(h, r)
	p1.connect_failed(mut cx, attempt, 'SSH connect failed: timed out', false)
	h.frame()
	assert h.has_text('SSH connect failed: timed out') && h.has_text('Connect')
	h.press('enter')!
	h.frame()
	assert r.requests.len == 2
	p1.connect_succeeded(mut cx, r.requests[1].attempt)
	h.frame()
	assert h.has_text('Connected to "Prod".') && !h.has_text('Connecting')
	assert h.find('connect-button') == none
}

fn test_cancelling_a_running_attempt_tells_the_app() {
	mut h, r := drig('cancel', fn (mut s store.SessionStore) {
		s.add(root_session())
	})!
	mut cx := dcx(h)
	mut p := pan(h, r)
	p.open_connect(mut cx, r.sessions.sessions()[0].id)
	h.frame()
	h.press('enter')!
	h.frame()
	assert r.requests.len == 1
	h.click('dialog-cancel')!
	h.frame()
	h.advance(1)
	assert r.cancels == [r.requests[0].attempt]
}

fn test_unknown_host_key_prompts_and_retries_with_the_fingerprint() {
	mut h, r := drig('hostkey', fn (mut s store.SessionStore) {
		s.add(root_session())
	})!
	mut cx := dcx(h)
	mut p := pan(h, r)
	p.open_connect(mut cx, r.sessions.sessions()[0].id)
	h.frame()
	h.press('enter')!
	h.frame()
	attempt := r.requests[0].attempt
	p.host_key_unknown(mut cx, attempt, '10.0.0.1', 22, 'ssh-ed25519', 'SHA256:abc')
	h.frame()
	assert h.has_text('Unknown SSH Host Key')
	assert h.texts().any(it.contains('\nSHA-256 fingerprint: SHA256:abc\n')) // one description
	assert h.find('connect-button') == none
	h.click('danger-ok')!
	h.frame()
	assert r.requests.len == 2
	assert r.requests[1].attempt == attempt && r.requests[1].accept_fingerprint == 'SHA256:abc'
}

fn test_quick_connect_saves_only_after_a_successful_connect() {
	mut h, r := drig('quick', fn (mut _ store.SessionStore) {})!
	mut cx := dcx(h)
	mut p := pan(h, r)
	p.open_quick_connect(mut cx)
	h.frame()
	assert h.has_text('SSH Quick Connect')
	type_into(mut h, 'quick-host', 'box.test')!
	type_into(mut h, 'quick-username', 'root')!
	h.click('quick-save-session')!
	h.frame()
	h.press('enter')!
	h.frame()
	assert r.sessions.sessions().len == 0, 'not saved before success'
	assert r.requests.len == 1 && r.requests[0].label == 'root@box.test:22'
	p.connect_succeeded(mut cx, r.requests[0].attempt)
	h.frame()
	assert r.sessions.sessions().len == 1
	assert r.sessions.sessions()[0].session.label == 'root@box.test:22'
}

// The whole save-password path: set up the master password, connect, seal after success.
fn test_saving_a_typed_password_seals_it_after_success() {
	mut h, r := drig('seal', fn (mut s store.SessionStore) {
		s.add(root_session())
	})!
	mut cx := dcx(h)
	mut p := pan(h, r)
	id := r.sessions.sessions()[0].id
	p.open_connect(mut cx, id)
	h.frame()
	type_into(mut h, 'connect-auth-password', 'hunter2')!
	h.click('connect-auth-save')!
	h.frame()
	h.press('enter')!
	h.frame()
	assert h.has_text('Create a master password')
	type_into(mut h, 'vault-input-0', 'correct horse')!
	type_into(mut h, 'vault-input-1', 'correct horse')!
	h.click('vault-confirm')!
	h.frame()
	h.wait_tasks()!
	h.frame()
	assert r.vault.phase() == .unlocked
	assert r.requests.len == 1, 'connected after the set-up'
	p.connect_succeeded(mut cx, r.requests[0].attempt)
	h.frame()
	sealed := r.sessions.get(id) or { panic('gone') }.sealed_password or { '' }
	assert sealed.starts_with('otv1:')
	assert !sealed.contains('hunter2')
	// The next connect uses the saved password; the dialog opens with no typing.
	p.open_connect(mut cx, id)
	h.frame()
	assert h.has_text('Using saved password')
	h.press('enter')!
	h.frame()
	assert r.requests.len == 2 && r.requests[1].credential.password == 'hunter2'
}
