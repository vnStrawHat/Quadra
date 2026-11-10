// The connect attempts: what the app reports back (specs/26).
module sessionui

import quadra.core
import quadra.kit
import store
import vault

// Attempt is one connection in flight, or waiting on the host-key prompt.
struct Attempt {
mut:
	id           u64
	request      ConnectRequest
	modal        int  // the dialog (or alert) carrying it; 0 once none does
	alert        bool // the host-key prompt, not a connect dialog
	dialog       core.View[ConnectDialog]
	has_dialog   bool
	save_session ?store.Session // Save to SSH Sessions: added after a successful connect
	seal         ?PendingSeal   // a typed password to seal after the server accepts it
}

// PendingSeal is a password to store once the connection is up (never before).
struct PendingSeal {
	session_id u64
	context    vault.SealContext
	password   string
}

// begin registers an attempt and tells the app to connect.
fn (mut p SessionPanel) begin_attempt(mut cx core.Cx, a_ Attempt) u64 {
	mut a := a_
	p.next_attempt++
	id := p.next_attempt
	a.id = id
	a.request = ConnectRequest{
		...a.request
		attempt: id
	}
	p.attempts[id] = a
	cx.app.invoke(p.on_connect, &a.request, cx.win, true)
	return id
}

const rejected_note = 'The server rejected the saved password. Type the current one.'

// connect_succeeded closes the dialog, runs the saves that waited for success and toasts.
pub fn (mut p SessionPanel) connect_succeeded(mut cx core.Cx, attempt u64) {
	a := p.attempts[attempt] or { return }
	p.attempts.delete(attempt)
	if a.has_dialog {
		kit.close_modal(mut cx, a.modal)
	}
	if sid := a.request.session_id {
		p.rejected.delete(sid)
	}
	if seal := a.seal {
		p.save_connected_password(mut cx, seal)
	}
	if session := a.save_session {
		p.sessions.add(session)
		p.refresh(mut cx)
	}
	success(mut cx, 'Connected to "${a.request.label}".')
}

// save_connected_password seals the password the server just accepted and stores it.
fn (mut p SessionPanel) save_connected_password(mut cx core.Cx, seal PendingSeal) {
	sealed := p.vault.seal(.ssh_password, seal.context, seal.password) or {
		warn(mut cx, 'Connected, but the password was not saved: ${err.msg()}')
		return
	}
	p.sessions.set_sealed_password(seal.session_id, seal.context.host, int(seal.context.port),
		seal.context.username, sealed.raw()) or {
		warn(mut cx, 'Connected, but the password was not saved: ${err.msg()}')
		return
	}
	p.refresh(mut cx)
	success(mut cx, 'Password saved.')
}

// connect_failed reports a failure the dialog shows inline (and as a toast). `authentication`
// says it was the server rejecting credentials; a saved password then turns into an input.
pub fn (mut p SessionPanel) connect_failed(mut cx core.Cx, attempt u64, message string, authentication bool) {
	a := p.attempts[attempt] or { return }
	p.attempts.delete(attempt)
	if a.has_dialog {
		mut d := cx.peek(a.dialog.model())
		d.connecting = false
		d.error = message
		d.attempt = 0
		if authentication {
			d.reveal_rejected(message, mut cx)
		}
		cx.touch(a.dialog.model())
		cx.touch_global[kit.Overlays]()
	} else if authentication {
		p.remember_rejection_toast(mut cx, a)
	}
	kit.push_toast(mut cx, .error, message)
}

// remember_rejection_toast: the dialog is gone (a host-key approval closed it).
fn (mut p SessionPanel) remember_rejection_toast(mut cx core.Cx, a Attempt) {
	sid := a.request.session_id or { return }
	s := p.sessions.get(sid) or { return }
	if sealed := s.saved_password() {
		p.rejected[sid] = sealed
	}
	warn(mut cx,
		'The saved password for "${s.label}" was rejected. Open the session to type the current one.')
}

fn (mut d ConnectDialog) reveal_rejected(message string, mut cx core.Cx) {
	if hop := hop_for_error(d.hops, message) {
		mut f := cx.peek(hop.form.model())
		if f.uses_saved() {
			f.reveal_input("The server rejected the saved password. Type the current one; update it in that jump host's session.",
				false)
			cx.touch(hop.form.model())
		}
		return
	}
	mut f := cx.peek(d.auth.model())
	if f.uses_saved() {
		f.reveal_input(rejected_note, true)
		cx.touch(d.auth.model())
		if sid := d.session_id {
			mut p := d.panel
			if sealed := d.session.saved_password() {
				p.rejected[sid] = sealed
			}
		}
	}
}

// host_key_unknown closes the dialog and asks; Trust and Connect re-emits the request.
pub fn (mut p SessionPanel) host_key_unknown(mut cx core.Cx, attempt u64, host string, port int, algorithm string, fingerprint string) {
	mut a := p.attempts[attempt] or { return }
	mut via := ''
	if a.request.hops.any(it.host == host && it.port == port) {
		via = '  (jump host for ${a.request.label})'
	}
	if a.has_dialog {
		kit.close_modal(mut cx, a.modal)
		a.has_dialog = false
	}
	a.alert = true
	a.modal = p.open_danger(mut cx, 'Unknown SSH Host Key', host_key_lines(host, port, via,
		algorithm, fingerprint), 'Trust and Connect', 1, attempt)
	a.request = accepted(a.request, host, port, fingerprint)
	p.attempts[attempt] = a
}

// accepted marks the hop with this address, else the target, as accepting the fingerprint.
fn accepted(r ConnectRequest, host string, port int, fingerprint string) ConnectRequest {
	mut hops := r.hops.clone()
	mut on_hop := false
	for i, h in hops {
		if h.host == host && h.port == port {
			hops[i] = HopRequest{
				...h
				accept_fingerprint: fingerprint
			}
			on_hop = true
			break
		}
	}
	return ConnectRequest{
		...r
		hops:               hops
		accept_fingerprint: if on_hop { r.accept_fingerprint } else { fingerprint }
	}
}

fn (mut p SessionPanel) trust_host_key(mut cx core.Cx, attempt u64) {
	mut a := p.attempts[attempt] or { return }
	a.modal = 0
	a.alert = false
	p.attempts[attempt] = a
	cx.app.invoke(p.on_connect, &a.request, cx.win, true)
}
