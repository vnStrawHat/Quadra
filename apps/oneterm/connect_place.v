// Landing a connected session (specs/22, Remote): where it goes, how its terminal starts.
module main

import sessionui
import termlog
import quadra.core
import quadra.kit
import quadra.kit.work
import quadra.platform

const duplicate_gone = 'The duplicate destination is no longer available.'

// place_session opens a tab for a new connection, or the duplicate's destination, and starts
// the terminal on it. A destination that vanished releases the session.
fn (mut o OneTerm) place_session(out ConnectOutcome, mut cx core.ViewCx[OneTerm]) {
	mut sess := unsafe { out.session }
	mut target := o.landing_space(out.meta.tag, mut cx) or {
		spawn close_session(mut sess)
		kit.notify(mut cx.Cx, .warning, 'SSH', duplicate_gone)
		return
	}
	target.remote = true
	target.session = sess
	target.ssh_closed = false
	target.duplicate = out.meta
	target.shell = ShellSpec{
		kind: 'ssh'
		name: out.meta.label
	}
	identity := termlog.ssh_log_identity(out.meta.username, out.meta.host, u16(out.meta.port))
	logging := out.logging.resolve(o.cfg.logging.ssh)
	o.open_terminal(mut target, Spawn{ sess: sess }, platform.PtyOptions{}, logging, identity, mut
		cx) or {
		spawn close_session(mut sess)
		kit.notify(mut cx.Cx, .error, 'SSH',
			'Connected, but the terminal could not be started: ${err.msg()}')
		return
	}
	if out.meta.initial_cwd != '' {
		mut t := cx.peek(target.view.model())
		t.send(remote_cd_command(out.meta.initial_cwd).bytes())
	}
	o.focus_active(mut cx)
	o.sync_active(mut cx)
	cx.notify()
}

// remote_cd_command is the original's `cd -- '<path>'`, quoting single quotes.
fn remote_cd_command(cwd string) string {
	escaped := cwd.replace("'", '\'"\'"\'')
	return "cd -- '${escaped}'\r"
}

// landing_space resolves a tag ('' = a new connection, see duplicate_ssh for the others).
fn (mut o OneTerm) landing_space(tag string, mut cx core.ViewCx[OneTerm]) ?&Space {
	if tag == '' {
		if empty := o.single_empty_space() {
			return empty
		}
		return o.open_empty_tab(mut cx)
	}
	src_id, dest := tag.before('|'), tag.after('|')
	if dest == 'new' {
		return o.open_empty_tab(mut cx)
	}
	mut src_tab := o.tab_with_space(src_id)?
	if dest.starts_with('space:') {
		id := dest.after('space:')
		for mut s in src_tab.spaces {
			if s.id == id && s.view.id == 0 {
				src_tab.active = s.id
				return s
			}
		}
		return none
	}
	axis, before := if dest == 'right' { work.Axis.row, false } else { work.Axis.col, false }
	return o.split_space(mut src_tab, src_id, axis, before, mut cx)
}

// single_empty_space is the only Space of the active tab, when it holds no terminal.
fn (o &OneTerm) single_empty_space() ?&Space {
	tab := o.current()
	if tab.spaces.len == 1 && tab.spaces[0].view.id == 0 {
		return tab.spaces[0]
	}
	return none
}

fn (mut o OneTerm) open_empty_tab(mut cx core.ViewCx[OneTerm]) &Space {
	mut s := o.new_space(mut cx)
	o.tabs << &TermTab{
		id:     't${s.number}'
		root:   work.leaf(s.id)
		spaces: [s]
		active: s.id
	}
	o.active = o.tabs.last().id
	return s
}

fn (o &OneTerm) tab_with_space(id string) ?&TermTab {
	for tab in o.tabs {
		for s in tab.spaces {
			if s.id == id {
				return tab
			}
		}
	}
	return none
}

// meta_of keeps what a duplicate needs and drops every secret.
fn meta_of(req &sessionui.ConnectRequest) sessionui.DuplicateSpec {
	return sessionui.DuplicateSpec{
		label:             req.label
		host:              req.host
		port:              req.port
		username:          req.username
		method:            req.credential.method
		key_path:          req.credential.key_path
		hops:              req.hops.map(sessionui.HopSpec{
			label:    it.label
			host:     it.host
			port:     it.port
			username: it.username
			method:   it.credential.method
			key_path: it.credential.key_path
		})
		forwards:          req.forwards
		agent_forwarding:  req.agent_forwarding
		shell_integration: req.shell_integration
		initial_cwd:       req.initial_cwd
		tag:               req.tag
	}
}
