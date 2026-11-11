// The terminal view in the harness on a FakePty (specs/17, Tests).
module main

import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.testing
import quadra.ui

struct Host {
mut:
	term core.View[term.Terminal]
	fake &term.FakePty = unsafe { nil }
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	if h.term.id == 0 {
		h.term = term.new_terminal(mut cx.Cx, id: 't')
		h.fake = &term.FakePty{
			prompt: '$ '
			answer: fn (line string) string {
				return if line.starts_with('echo ') { line[5..] + '\r\n' } else { '' }
			}
		}
		term.start(mut cx.Cx, h.term, h.fake.spawner(), platform.PtyOptions{}) or { panic(err) }
	}
	return ui.div().size_full().child(cx.child_view(h.term))
}

fn open() !(&testing.Harness, &Host) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Host{}, width: 640, height: 400)!
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	return h, host
}

fn screen_of(h &testing.Harness, host &Host) &term.Terminal {
	mut cx := core.Cx{
		app: h.app
	}
	return cx.peek(host.term.model())
}

fn test_typing_echoes_and_programs_answer() {
	mut h, host := open()!
	t := screen_of(h, host)
	assert t.screen.cols > 40 && t.screen.rows > 10 // fitted to the window
	assert host.fake.cols == t.screen.cols // and the PTY was resized
	h.click('t')!
	h.type_text('echo hi')
	h.press('enter')!
	assert t.screen.text() == '$ echo hi\nhi\n$'
	assert host.fake.written.bytestr() == 'echo hi\r'
	h.press('up')!
	h.press('ctrl-c')!
	assert host.fake.written.bytestr().ends_with('\x1b[A\x03')
}

fn test_history_scrolls_and_typing_returns() {
	mut h, host := open()!
	mut fake := host.fake
	for i in 0 .. 100 {
		fake.output('line ${i}\r\n')
	}
	h.frame()
	t := screen_of(h, host)
	assert t.screen.scrollback.len() > 50
	r := h.find('t') or { panic('') }
	h.scroll_at(r.x + 50, r.y + 50, 120)
	assert t.offset > 0
	before := t.offset
	fake.output('more\r\n') // reading history: the view stays on the same lines
	h.frame()
	assert t.offset == before + 1
	h.click('t')!
	h.type_text('x')
	assert t.offset == 0
}

fn test_select_copy_and_paste() {
	mut h, host := open()!
	mut fake := host.fake
	fake.output('hello world\r\n')
	h.frame()
	t := screen_of(h, host)
	// Double-click the second word of the first row (after the prompt).
	r := h.find('t') or { panic('') }
	x := r.x + t.cell.width * 9.5
	y := r.y + t.cell.height * 0.5
	h.click_at(x, y)
	h.click_at(x, y)
	assert t.selected_text() == 'world'
	h.press('ctrl-shift-c')!
	assert h.app.plat.clipboard_read() or { '' } == 'world'
	// Paste is bracketed once the program asks for it.
	fake.output('\x1b[?2004h')
	h.frame()
	h.app.plat.clipboard_write('a\nb')
	h.press('ctrl-shift-v')!
	assert fake.written.bytestr().ends_with('\x1b[200~a\nb\x1b[201~')
	// Triple-click selects the row.
	h.advance(1000)
	h.click_at(x, y)
	h.click_at(x, y)
	h.click_at(x, y)
	assert t.selected_text() == '$ hello world'
}

fn test_exit_and_resize() {
	mut h, host := open()!
	mut fake := host.fake
	h.resize(400, 300)
	t := screen_of(h, host)
	assert fake.cols == t.screen.cols && t.screen.cols < 60
	fake.exit(3)
	h.frame()
	assert t.exited or { -1 } == 3
	assert t.screen.text().contains('[process exited with code 3]')
}

fn test_color_queries_and_sixel_through_the_view() {
	mut h, host := open()!
	mut fake := host.fake
	t := screen_of(h, host)
	h.click('t')!
	fake.written.clear()
	fake.echo = false
	fake.output('\x1b]11;?\x07\x1bPq#0;2;100;0;0#0!40~-!40~\x1b\\\r\n')
	h.frame()
	assert fake.written.bytestr().starts_with('\x1b]11;rgb:')
	assert t.screen.graphics.len == 1
	h.frame()
	assert t.gfx.len == 1 // painted, so uploaded to the atlas
	fake.output('\x1b[2J\x1b[3J\r\n') // cleared rows free the image and its tile
	for _ in 0 .. 70 {
		fake.output('x\r\n') // one chunk per frame: the sweep runs every 64 chunks
		h.frame()
	}
	assert t.screen.graphics.len == 0 && t.gfx.len == 0
}
