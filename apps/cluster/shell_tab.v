// The Shell tab: a terminal on a fake container shell for the selected pod. With --local,
// the first shell is the real system shell on ConPTY instead (manual checks).
module main

import os
import apps.cluster.fake
import quadra.core
import quadra.kit.term
import quadra.platform

fn container_shell(r fake.Resource) core.Bound[string, string] {
	return core.bind_fn(r, container_reply)
}

fn container_reply(r fake.Resource, line string) string {
	cmd := line.trim_space()
	return match cmd.all_before(' ') {
		'' { '' }
		'ls' { 'app  bin  dev  etc  home  lib  proc  tmp  usr  var\r\n' }
		'hostname' { '${r.name}\r\n' }
		'env' { 'HOSTNAME=${r.name}\r\nKUBERNETES_SERVICE_HOST=10.96.0.1\r\nNAMESPACE=${r.namespace}\r\nIMAGE=${r.image}\r\n' }
		'ps' { '  PID USER     TIME  COMMAND\r\n    1 app       0:12 /app/server --port 8080\r\n   42 root      0:00 sh\r\n' }
		'logs' { 'logs are on the Logs tab\r\n' }
		else { 'sh: ${cmd.all_before(' ')}: not found\r\n' }
	}
}

struct AppRef {
	app &core.App
}

// open_shell connects the terminal to the selected pod (once per pod).
fn (mut c Cluster) open_shell(r fake.Resource, mut cx core.ViewCx[Cluster]) {
	if c.shell_pod == r.name {
		return
	}
	if c.term.id == 0 {
		c.term = term.new_terminal(mut cx.Cx, id: 'shell')
	}
	if c.shell_pod == '' && '--local' in os.args {
		c.shell_pod = r.name
		term.start_with(mut cx.Cx, c.term, AppRef{cx.app}, fn (a AppRef, o platform.PtyOptions) !platform.Pty {
			mut app := a.app
			return app.plat.spawn_pty(o)
		}, platform.PtyOptions{}) or {}
		return
	}
	if !isnil(c.shell) {
		c.shell.closed = true // the old session goes quiet; the screen is reset below
	}
	c.shell_pod = r.name
	c.shell = &term.FakePty{
		reply: container_shell(r)
	}
	mut t := cx.peek(c.term.model())
	t.feed('\x1bc'.bytes())
	term.start_with(mut cx.Cx, c.term, term.FakeRef{c.shell}, term.spawn_fake, platform.PtyOptions{}) or {}
	c.shell.prompt = '\x1b[32mroot@${r.name}\x1b[0m:/# '
	c.shell.output('\x1b[2mkubectl exec -it ${r.name} -n ${r.namespace} -- sh\x1b[0m\r\n' +
		c.shell.prompt)
}
