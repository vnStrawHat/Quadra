// Connecting (specs/22, Remote): a ConnectRequest from the Session panel becomes an engine Config,
// runs on a worker and lands as a remote Space. Nothing here logs or stores a secret.
module main

import os
import sync
import ssh
import sessionui
import store
import quadra.core

// WarnBox collects `Config.on_warning` messages raised on worker threads.
@[heap]
struct WarnBox {
mut:
	mu   &sync.Mutex = sync.new_mutex()
	msgs []string
}

fn (mut b WarnBox) push(msg string) {
	b.mu.@lock()
	b.msgs << msg
	b.mu.unlock()
}

fn (mut b WarnBox) take() []string {
	b.mu.@lock()
	defer {
		b.mu.unlock()
	}
	out := b.msgs.clone()
	b.msgs.clear()
	return out
}

enum ConnectKind {
	connected
	host_unknown
	failed
	cancelled
}

// ConnectOutcome is what a worker hands back; `meta` holds no secret.
struct ConnectOutcome {
	attempt        u64
	kind           ConnectKind
	session        &ssh.Session = unsafe { nil }
	meta           sessionui.DuplicateSpec
	logging        store.LoggingOverride
	message        string
	authentication bool
	host           string
	port           int
	algorithm      string
	fingerprint    string
}

fn auth_of(c sessionui.Credential) ssh.AuthMethod {
	match c.method {
		.password {
			if c.password == '' {
				return ssh.AuthMethod(ssh.NoAuth{}) // an empty password is "auth none"
			}
			return ssh.AuthMethod(ssh.PasswordAuth{
				password: ssh.secret(c.password)
			})
		}
		.private_key {
			mut key := ssh.KeyAuth{
				key_path: c.key_path
			}
			if c.passphrase != '' {
				key.passphrase = ssh.secret(c.passphrase)
			}
			return ssh.AuthMethod(key)
		}
		.agent {
			return ssh.AuthMethod(ssh.AgentAuth{})
		}
	}
}

fn forward_of(f store.PortForward) ssh.PortForward {
	return match f.kind {
		.local {
			ssh.PortForward(ssh.LocalForward{f.bind, f.bind_port, f.target_host, f.target_port})
		}
		.remote {
			ssh.PortForward(ssh.RemoteForward{f.bind, f.bind_port, f.target_host, f.target_port})
		}
		.dynamic {
			ssh.PortForward(ssh.DynamicForward{f.bind, f.bind_port})
		}
	}
}

fn (o &OneTerm) known_hosts_file() string {
	return if o.known_hosts != '' {
		o.known_hosts
	} else {
		os.join_path(os.home_dir(), '.ssh', 'known_hosts')
	}
}

// config_of builds the engine Config. Jump hops carry their address as label, so a hop's error
// starts with `jump host <user>@<host>:<port>:`.
fn (o &OneTerm) config_of(req &sessionui.ConnectRequest, cancel &ssh.Cancel, mut box WarnBox) ssh.Config {
	mut hops := []ssh.Hop{cap: req.hops.len}
	for h in req.hops {
		hops << ssh.Hop{
			host:     h.host
			port:     h.port
			username: h.username
			auth:     auth_of(h.credential)
			policy:   ssh.accept_new(h.accept_fingerprint)
		}
	}
	return ssh.Config{
		target:            ssh.Hop{
			host:     req.host
			port:     req.port
			username: req.username
			auth:     auth_of(req.credential)
			policy:   ssh.accept_new(req.accept_fingerprint)
		}
		jump_hops:         hops
		known_hosts_path:  o.known_hosts_file()
		agent_forwarding:  req.agent_forwarding
		shell_integration: req.shell_integration
		port_forwards:     req.forwards.map(forward_of(it))
		keepalive:         ssh.Keepalive{
			enabled: o.cfg.ssh.keepalive_enabled
			seconds: o.cfg.ssh.keepalive_interval_secs
			max:     o.cfg.ssh.keepalive_max
		}
		cancel:            cancel
		on_warning:        fn [mut box] (msg string) {
			box.push(msg)
		}
	}
}

// failure_message names the failure once, as the original's connect_failure_message does;
// a hop's failure keeps its `jump host` prefix first so the panel can blame the right row.
fn failure_message(err IError) (string, bool) {
	if err is ssh.ConnectError {
		text := if err.message.starts_with('jump host ') { err.message } else { err.msg() }
		return text, err.phase == .authentication
	}
	if err is ssh.HostKeyChanged {
		return err.msg(), false
	}
	return 'SSH connect failed: ${err.msg()}', false
}

fn wipe_auth(mut a ssh.AuthMethod) {
	match mut a {
		ssh.PasswordAuth {
			a.password.wipe()
		}
		ssh.KeyAuth {
			if mut p := a.passphrase {
				p.wipe()
			}
		}
		else {}
	}
}

// run_connect is the worker: typed errors become plain data, spawn_task keeps only strings.
fn run_connect(mut cfg ssh.Config, base ConnectOutcome) ConnectOutcome {
	session := ssh.connect(cfg) or {
		wipe_config(mut cfg)
		if err is ssh.HostKeyUnknown {
			return ConnectOutcome{
				...base
				kind:        .host_unknown
				host:        err.host
				port:        err.port
				algorithm:   err.algorithm
				fingerprint: err.fingerprint
			}
		}
		if err is ssh.Cancelled {
			return ConnectOutcome{
				...base
				kind: .cancelled
			}
		}
		text, auth := failure_message(err)
		return ConnectOutcome{
			...base
			kind:           .failed
			message:        text
			authentication: auth
		}
	}
	wipe_config(mut cfg)
	return ConnectOutcome{
		...base
		kind:    .connected
		session: session
	}
}

fn wipe_config(mut cfg ssh.Config) {
	wipe_auth(mut cfg.target.auth)
	for mut h in cfg.jump_hops {
		wipe_auth(mut h.auth)
	}
}

// on_connect starts an attempt; Trust and Connect re-enters here with the same attempt id.
fn (mut o OneTerm) on_connect(req &sessionui.ConnectRequest, mut cx core.ViewCx[OneTerm]) {
	if is_restricted() {
		return
	}
	mut cancel := ssh.new_cancel()
	o.attempts[req.attempt] = cancel
	mut cfg := o.config_of(req, cancel, mut o.warn_box)
	base := ConnectOutcome{
		attempt: req.attempt
		meta:    meta_of(req)
		logging: req.logging
	}
	core.spawn_task[OneTerm, ConnectOutcome](mut cx, fn [mut cfg, base] (_ &core.TaskCx) !ConnectOutcome {
		return run_connect(mut cfg, base)
	}, fn (mut o OneTerm, r &core.TaskResult[ConnectOutcome], mut cx core.ViewCx[OneTerm]) {
		o.finish_connect(r.value, mut cx)
	})
}

// on_cancel: closing the dialog ends the attempt; a late result finds no attempt and is dropped.
fn (mut o OneTerm) on_cancel(attempt u64) {
	if mut c := o.attempts[attempt] {
		c.cancel()
		o.attempts.delete(attempt)
	}
}

fn close_session(mut s ssh.Session) {
	s.close()
}

fn (mut o OneTerm) drain_warnings(mut cx core.ViewCx[OneTerm]) {
	for msg in o.warn_box.take() {
		toast(mut cx, .warning, 'SSH', msg)
	}
}

// finish_connect is the done half of an attempt, on the UI thread.
fn (mut o OneTerm) finish_connect(out ConnectOutcome, mut cx core.ViewCx[OneTerm]) {
	if cx.win == unsafe { nil } && cx.app.windows.len > 0 {
		cx.win = cx.app.windows[0] // a task callback has no window; dialogs need one
	}
	o.drain_warnings(mut cx)
	if out.attempt !in o.attempts {
		if out.session != unsafe { nil } { // cancelled: the connection is dropped silently
			mut s := unsafe { out.session }
			spawn close_session(mut s)
		}
		return
	}
	o.attempts.delete(out.attempt)
	mut panel := cx.peek(o.sess_view.model())
	match out.kind {
		.connected {
			o.place_session(out, mut cx)
			panel.connect_succeeded(mut cx.Cx, out.attempt)
		}
		.host_unknown {
			panel.host_key_unknown(mut cx.Cx, out.attempt, out.host, out.port, out.algorithm,
				out.fingerprint)
		}
		.failed {
			panel.connect_failed(mut cx.Cx, out.attempt, out.message, out.authentication)
		}
		.cancelled {
			panel.connect_failed(mut cx.Cx, out.attempt, 'SSH connect failed: cancelled', false)
		}
	}
}
