// Tabs and Spaces: the operations of specs/22. Every terminal is made and started here.
module main

import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.kit.work
import quadra.platform
import quadra.prim
import termlog
import ssh
import os

fn (mut o OneTerm) new_space(mut cx core.ViewCx[OneTerm]) &Space {
	n := o.next
	o.next++
	return &Space{
		id:     's${n}'
		number: n
		focus:  cx.focus_handle()
	}
}

// new_tab opens a tab with one Space running the shell `kind` ('' = the default).
fn (mut o OneTerm) new_tab(kind string, cwd string, mut cx core.ViewCx[OneTerm]) {
	mut s := o.new_space(mut cx)
	mut tab := &TermTab{
		id:     't${s.number}'
		root:   work.leaf(s.id)
		spaces: [s]
		active: s.id
	}
	o.tabs << tab
	o.active = tab.id
	o.start(mut s, kind, cwd, mut cx)
	o.focus_active(mut cx)
}

// start runs a shell in a Space.
fn (mut o OneTerm) start(mut s Space, kind string, cwd string, mut cx core.ViewCx[OneTerm]) {
	shell_cfg := o.shell_config(kind) or {
		kit.notify(mut cx.Cx, .error, 'Shell', err.msg())
		return
	}
	spec := resolve_shell(shell_cfg, kind, if is_restricted() { '' } else { cwd })
	s.shell = spec
	o.open_terminal(mut s, Spawn{ make: o.spawn, app: cx.app }, platform.PtyOptions{
		command: spec.command
		env:     spec.env
		cwd:     spec.cwd
	}, o.cfg.logging.local, termlog.local_log_identity(spec.name, u32(os.getpid())), mut cx) or {
		mut t := cx.peek(s.view.model())
		t.feed('\x1b[31mCould not start ${spec.command}: ${err}\x1b[0m\r\n'.bytes())
	}
}

// open_terminal gives a Space its terminal and runs `make` behind the broadcast decorator.
fn (mut o OneTerm) open_terminal(mut s Space, make Spawn, opts platform.PtyOptions, logging bool, identity string, mut cx core.ViewCx[OneTerm]) ! {
	s.view = term.new_terminal(mut cx.Cx,
		id:               'term-${s.id}'
		scrollback:       o.cfg.scroll.scrollback_history
		allow_osc52:      true
		allow_osc52_read: o.cfg.security.allow_clipboard_read
		product:          'OneTerm'
		behavior:         o.behavior()
		quiet_exit:       s.remote
	)
	cx.subscribe[term.Terminal, term.TerminalInfo](s.view.model(), fn (mut o OneTerm, m core.Model[term.Terminal], e &term.TerminalInfo, mut cx core.ViewCx[OneTerm]) {
		o.on_terminal_info(m, e, mut cx)
	})
	o.attach_ext(mut s, logging, identity, mut cx)
	term.start_with(mut cx.Cx, s.view, TabSpawn{make, unsafe { &OneTerm(&o) }, cx.app, unsafe { &Space(&s) }},
		spawn_tab, opts)!
}

// Spawn is how a Space's program starts: the platform's shell (or a test's spawner), or an
// SSH session's shell.
struct Spawn {
	make term.SpawnFn = unsafe { nil } // tests: a fake
	sess &ssh.Session = unsafe { nil }
	app  &core.App    = unsafe { nil }
}

fn (s Spawn) run(opts platform.PtyOptions) !platform.Pty {
	if s.sess != unsafe { nil } {
		mut sess := s.sess
		return sess.attach(opts)!
	}
	if s.make != unsafe { nil } {
		return s.make(opts)!
	}
	mut app := s.app
	return app.plat.spawn_pty(opts)!
}

// TabSpawn is what spawn_tab needs: data, not a closure per terminal (specs/14).
struct TabSpawn {
	make  Spawn
	one   &OneTerm  = unsafe { nil }
	app   &core.App = unsafe { nil }
	space &Space    = unsafe { nil }
}

// spawn_tab runs the program behind the broadcast decorator; its output also goes to the
// Space's log (LogTap).
fn spawn_tab(w TabSpawn, opts platform.PtyOptions) !platform.Pty {
	tap := &LogTap{w.space.ext.log, opts.ctx, opts.on_data, opts.on_exit}
	mut b := &BroadcastPty{
		one:   w.one
		space: w.space
		app:   w.app
		inner: w.make.run(platform.PtyOptions{
			...opts
			ctx:     prim.hold(tap)
			on_data: tap_data
			on_exit: tap_exit
		})!
	}
	mut sp := w.space
	sp.pty = b
	sp.ext.pty = b
	return b
}

struct LogTap {
	log     &termlog.TerminalLogController = unsafe { nil }
	ctx     prim.Holder
	on_data fn (ctx prim.Holder, data []u8) = unsafe { nil }
	on_exit fn (ctx prim.Holder, code int)  = unsafe { nil }
}

fn tap_data(ctx prim.Holder, data []u8) {
	t := prim.held[LogTap](ctx)
	mut log := t.log
	log.process(data) // the printable output goes to the log file
	t.on_data(t.ctx, data)
}

fn tap_exit(ctx prim.Holder, code int) {
	t := prim.held[LogTap](ctx)
	t.on_exit(t.ctx, code)
}

// focus_active gives the keyboard to the active Space of the active tab.
fn (mut o OneTerm) focus_active(mut cx core.ViewCx[OneTerm]) {
	tab := o.current()
	for s in tab.spaces {
		if s.id == tab.active {
			if s.view.id != 0 {
				cx.peek(s.view.model()).focus.focus(mut cx.Cx)
			} else {
				s.focus.focus(mut cx.Cx)
			}
		}
	}
}

fn (mut o OneTerm) select_tab(id string, mut cx core.ViewCx[OneTerm]) {
	o.active = id
	o.sync_active(mut cx)
	o.focus_active(mut cx)
	cx.notify()
}

// shell_config is the shell settings of a new terminal; an elevated window ignores
// terminal.json and takes a fixed path.
fn (o &OneTerm) shell_config(kind string) !ShellConfig {
	if !is_restricted() {
		return o.cfg.shell
	}
	return trusted_shell(if kind != '' { kind } else { 'cmd' })!
}

// move_into_space moves a dragged tab's active terminal into an empty Space; the emptied tab
// closes without ending the shell.
fn (mut o OneTerm) move_into_space(tab_id string, target string, mut cx core.ViewCx[OneTerm]) {
	mut tab := o.current()
	mut found := tab.spaces.filter(it.id == target && it.view.id == 0)
	src_i := o.tabs.map(it.id).index(tab_id)
	if found.len == 0 || src_i < 0 {
		return
	}
	mut dst := found[0]
	mut src_tab := o.tabs[src_i]
	mut src := src_tab.active_space() or { return }
	if src.view.id == 0 || src.id == dst.id {
		return
	}
	dst.view, dst.shell, dst.channel = src.view, src.shell, src.channel
	dst.remote, dst.ext, dst.status, dst.progress = src.remote, src.ext, src.status, src.progress
	dst.session, dst.duplicate, dst.ssh_closed = src.session, src.duplicate, src.ssh_closed
	if src.pty != unsafe { nil } {
		dst.pty = src.pty
		dst.pty.space = dst
	}
	src.view, src.pty, src.channel = core.View[term.Terminal]{}, unsafe { nil }, ''
	src.ext, src.session = unsafe { nil }, unsafe { nil }
	src.remote = false
	if src_tab.spaces.len == 1 {
		o.tabs.delete(src_i)
		o.active = tab.id
	} else {
		i := src_tab.spaces.map(it.id).index(src.id)
		src_tab.root = src_tab.root.remove(src.id)
		src_tab.spaces.delete(i)
		src_tab.active = src_tab.root.groups()[0]
	}
	tab.active = target
	o.sync_active(mut cx)
	o.focus_active(mut cx)
	cx.notify()
}

fn (mut o OneTerm) move_tab(id string, to int) {
	from := o.tabs.map(it.id).index(id)
	if from < 0 || to < 0 || to >= o.tabs.len {
		return
	}
	tab := o.tabs[from]
	o.tabs.delete(from)
	o.tabs.insert(to, tab)
}

fn stop(s &Space, mut cx core.ViewCx[OneTerm]) {
	if s.view.id != 0 {
		mut t := cx.peek(s.view.model())
		t.close()
	}
}

// close_tab ends a tab's shells; closing the last tab resets it to one empty tab.
fn (mut o OneTerm) close_tab(id string, mut cx core.ViewCx[OneTerm]) {
	i := o.tabs.map(it.id).index(id)
	if i < 0 {
		return
	}
	for s in o.tabs[i].spaces {
		stop(s, mut cx)
	}
	if o.tabs.len == 1 {
		mut s := o.new_space(mut cx)
		o.tabs[0] = &TermTab{
			id:     't${s.number}'
			root:   work.leaf(s.id)
			spaces: [s]
			active: s.id
		}
		o.active = o.tabs[0].id
	} else {
		o.tabs.delete(i)
		if o.active == id {
			o.active = o.tabs[if i < o.tabs.len { i } else { i - 1 }].id
		}
	}
	o.sync_active(mut cx)
	o.focus_active(mut cx)
	cx.notify()
}
