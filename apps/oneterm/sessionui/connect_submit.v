// Connect in the connect dialog: validate, collect credentials, start an attempt (specs/26).
module sessionui

import quadra.core
import quadra.kit
import store
import vault

// connect runs the Rust connect logic: a password to save needs the vault unlocked first.
fn (mut d ConnectDialog) connect(mut cx core.ViewCx[ConnectDialog]) {
	if d.connecting {
		return
	}
	mut p := d.panel
	auth := cx.peek(d.auth.model())
	if auth.wants_save() && d.mode == .saved {
		if auth.typed(cx.Cx) == '' {
			warn(mut cx.Cx, 'Enter the password to save, or untick Save password.')
			return
		}
		match p.vault.phase() {
			.unlocked {
				d.attempt_now(mut cx)
			}
			.locked, .uninitialised {
				m := d.me.model()
				p.ensure_unlocked(mut cx.Cx, fn [m] (mut cx core.Cx) {
					cx.update(m, fn (mut d ConnectDialog, mut cx core.ViewCx[ConnectDialog]) {
						d.attempt_now(mut cx)
					})
				})
			}
			.unavailable {
				mut reason := p.vault.unavailable_reason()
				if reason == '' {
					reason = 'no master password store.'
				}
				warn(mut cx.Cx,
					'Saved passwords are unavailable: ${reason} Untick Save password to connect.')
			}
		}

		return
	}
	d.attempt_now(mut cx)
}

fn (mut d ConnectDialog) attempt_now(mut cx core.ViewCx[ConnectDialog]) {
	if d.connecting {
		return
	}
	d.error = ''
	match d.mode {
		.saved { d.connect_saved(mut cx) }
		else { d.connect_quick(mut cx) }
	}
}

// connect_saved is on_connect_click: username, hops, credentials, store, then the attempt.
fn (mut d ConnectDialog) connect_saved(mut cx core.ViewCx[ConnectDialog]) {
	mut p := d.panel
	mut session := d.session
	mut username := session.username or { '' }
	mut host := session.host
	mut port := session.port
	if d.has_username {
		raw := cx.peek(d.username.model()).value().trim_space()
		if raw == '' {
			warn(mut cx.Cx, 'Username is required.')
			return
		}
		parsed := parse_user_host_port(raw) or {
			warn(mut cx.Cx, err.msg())
			return
		}
		username = parsed.user
		host = parsed.host or { session.host }
		port = parsed.port or { session.port }
	}
	hops := take_hops(d.hops, mut cx.Cx) or {
		warn(mut cx.Cx, err.msg())
		cx.notify()
		return
	}
	mut auth := cx.peek(d.auth.model())
	cred := auth.take_credential(mut cx.Cx) or {
		warn(mut cx.Cx, err.msg())
		cx.touch(d.auth.model())
		return
	}
	sid := d.session_id or { 0 }
	if (d.save_username || auth.wants_save()) && d.has_username {
		session.username = username
		session.host = host
		session.port = port
		p.sessions.update(sid, session)
		p.refresh(mut cx.Cx)
	}
	mut seal := ?PendingSeal(none)
	if auth.wants_save() && cred.password != '' {
		seal = PendingSeal{sid, vault.SealContext{host, u16(port), username}, cred.password}
	}
	d.start(ConnectRequest{
		session_id:       d.session_id
		label:            session.label
		host:             host
		port:             port
		username:         username
		credential:       cred
		hops:             hops
		forwards:         session.forwards
		agent_forwarding: session.agent_forwarding
		logging:          session.logging
	}, none, seal, mut cx)
}

// connect_quick is the Quick Connect / Duplicate path.
fn (mut d ConnectDialog) connect_quick(mut cx core.ViewCx[ConnectDialog]) {
	if d.chain_error != '' {
		warn(mut cx.Cx, d.chain_error)
		return
	}
	hops := take_hops(d.hops, mut cx.Cx) or {
		warn(mut cx.Cx, err.msg())
		cx.notify()
		return
	}
	target := resolve_target(cx.peek(d.username.model()).value().trim_space(),
		cx.peek(d.host.model()).value().trim_space(), cx.peek(d.port.model()).value().trim_space()) or {
		warn(mut cx.Cx, err.msg())
		return
	}
	mut auth := cx.peek(d.auth.model())
	cred := auth.take_credential(mut cx.Cx) or {
		warn(mut cx.Cx, err.msg())
		cx.touch(d.auth.model())
		return
	}
	mut save := ?store.Session(none)
	if d.save_session {
		save = store.Session{
			label:       target.label()
			host:        target.host
			port:        target.port
			username:    target.username
			auth_method: auth.method
			key_path:    if auth.method == .private_key { auth.key_value(cx.Cx) } else { none }
			jump_host:   d.picked
		}
	}
	dup := d.duplicate
	d.start(ConnectRequest{
		label:             target.label()
		host:              target.host
		port:              target.port
		username:          target.username
		credential:        cred
		hops:              hops
		forwards:          dup.forwards
		agent_forwarding:  dup.agent_forwarding
		shell_integration: dup.shell_integration
		initial_cwd:       dup.initial_cwd
		tag:               dup.tag
	}, save, none, mut cx)
}

// start registers the attempt with the panel, emits the request and shows Connecting.
fn (mut d ConnectDialog) start(req ConnectRequest, save ?store.Session, seal ?PendingSeal, mut cx core.ViewCx[ConnectDialog]) {
	mut p := d.panel
	d.connecting = true
	d.attempt = p.begin_attempt(mut cx.Cx, Attempt{
		request:      req
		modal:        d.modal
		dialog:       d.me
		has_dialog:   true
		save_session: save
		seal:         seal
	})
	cx.notify()
	cx.touch_global[kit.Overlays]()
}
