// The Terminal page: the terminal view on a fake shell (no process), with colors, styles
// and wide characters to show what the grid paints.
module pages

import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.ui

pub struct TerminalPage {
mut:
	term core.View[term.Terminal]
	fake &term.FakePty = unsafe { nil }
}

const term_banner = '\x1b[1mQuadra terminal\x1b[0m  \x1b[2mfake shell: try help, ls, colors, styles\x1b[0m\r\n\r\n'

fn fake_shell(line string) string {
	return match line.trim_space() {
		'' {
			''
		}
		'help' {
			'commands: \x1b[36mls\x1b[0m \x1b[36mcolors\x1b[0m \x1b[36mstyles\x1b[0m \x1b[36mclear\x1b[0m\r\n'
		}
		'ls' {
			'\x1b[1;34mapps\x1b[0m  \x1b[1;34mquadra\x1b[0m  \x1b[1;34mspecs\x1b[0m  \x1b[32mbuild.sh\x1b[0m  README.md  v.mod\r\n'
		}
		'colors' {
			mut s := ''
			for i in 0 .. 16 {
				s += '\x1b[48;5;${i}m  \x1b[0m'
			}
			s += '\r\n'
			for i in 0 .. 36 {
				s += '\x1b[48;5;${16 + i * 6}m \x1b[0m'
			}
			s += '\r\n'
			for i in 0 .. 48 {
				r, g := 255 * i / 47, 128
				s += '\x1b[48;2;${r};${g};${255 - r}m \x1b[0m'
			}
			s + '\r\n'
		}
		'styles' {
			'\x1b[1mbold\x1b[0m \x1b[2mfaint\x1b[0m \x1b[3mitalic\x1b[0m \x1b[4munderline\x1b[0m \x1b[4:3mcurly\x1b[0m \x1b[9mstrike\x1b[0m \x1b[7minverse\x1b[0m  漢字 ok\r\n'
		}
		'clear' {
			'\x1b[2J\x1b[H'
		}
		else {
			'\x1b[31m${line}: command not found\x1b[0m\r\n'
		}
	}
}

@[markused]
pub fn (mut p TerminalPage) render(mut cx core.ViewCx[TerminalPage]) core.Element {
	t := kit.theme(cx)
	if p.term.id == 0 {
		p.term = term.new_terminal(mut cx.Cx, id: 'demo-term')
		p.fake = &term.FakePty{
			answer: fake_shell
		}
		term.start(mut cx.Cx, p.term, p.fake.spawner(), platform.PtyOptions{}) or {}
		p.fake.prompt = '\x1b[32mquadra\x1b[0m:\x1b[34m~\x1b[0m$ '
		p.fake.output(term_banner + p.fake.prompt)
		for cmd in ['ls', 'styles', 'colors'] {
			p.fake.write('${cmd}\r'.bytes()) or {}
		}
	}
	box := ui.div().h(360).bg(t.code.terminal.background).p(8).child(cx.child_view(p.term))
	return page('Terminal',
		'A VT terminal: one element paints the visible rows, output arrives through an inbox, and the PTY is ConPTY on Windows or a fake in tests. Select with the mouse, ctrl-shift-c copies, ctrl-shift-v pastes.', [
		demo('Fake shell', framed(t, box),
			'term.start(mut cx, v.term, cx.app.plat.spawn_pty, platform.PtyOptions{})  // or FakePty.spawner()'),
	])
}
