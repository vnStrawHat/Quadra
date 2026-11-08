// Terminal: the view of a session (screen, parser, PTY). Output arrives through a
// core.Inbox; the grid element paints the visible rows (grid.v), selection is in select.v.
module term

import quadra.core
import quadra.prim
import quadra.platform
import quadra.ui

pub type SpawnFn = fn (opts platform.PtyOptions) !platform.Pty

@[heap]
pub struct Terminal {
pub mut:
	id     string
	screen &Screen = unsafe { nil }
	parser Parser
	pty    ?platform.Pty
	exited ?int
	offset int // lines scrolled back into the history (0 = following the output)
	sel    Selection
	focus  core.FocusHandle
	bounds prim.Rect // the grid, from the last prepaint
	cell   prim.Size
	inbox  &core.Inbox = unsafe { nil }
}

@[params]
pub struct TerminalOptions {
pub:
	id          string = 'term'
	cols        int    = 80
	rows        int    = 24
	scrollback  int    = 10000
	allow_osc52 bool // programs may write the clipboard
}

pub fn new_terminal(mut cx core.Cx, opts TerminalOptions) core.View[Terminal] {
	mut s := new_screen(opts.cols, opts.rows)
	s.scrollback.max = opts.scrollback
	s.allow_osc52 = opts.allow_osc52
	return cx.new_view(Terminal{
		id:     opts.id
		screen: s
		focus:  cx.focus_handle()
	})
}

// start runs a program on a PTY from make (the platform spawn_pty, or a FakePty spawner).
pub fn start(mut cx core.Cx, v core.View[Terminal], make SpawnFn, opts platform.PtyOptions) ! {
	mut t := cx.peek(v.model())
	if t.inbox == unsafe { nil } {
		cx.update_view(v, fn (mut t Terminal, mut cx core.ViewCx[Terminal]) {
			t.inbox = core.new_inbox(mut cx, fn (mut t Terminal, c &core.Chunk, mut cx core.ViewCx[Terminal]) {
				t.receive(c, mut cx)
			})
		})
	}
	mut inbox := t.inbox
	pty := make(platform.PtyOptions{
		...opts
		cols:    t.screen.cols
		rows:    t.screen.rows
		on_data: fn [mut inbox] (data []u8) {
			inbox.push(data)
		}
		on_exit: fn [mut inbox] (code int) {
			inbox.finish(code)
		}
	})!
	t.pty = pty
	t.exited = none
}

// feed parses output as if the program wrote it (tests, local echo).
pub fn (mut t Terminal) feed(data []u8) {
	before := t.screen.scrollback.len()
	t.parser.feed(mut t.screen, data)
	if t.offset > 0 { // reading the history: stay on the same lines
		t.offset = clamp(t.offset + t.screen.scrollback.len() - before, 0,
			t.screen.scrollback.len())
	}
	if t.screen.replies.len > 0 {
		t.send(t.screen.replies)
		t.screen.replies.clear()
	}
}

fn (mut t Terminal) receive(c &core.Chunk, mut cx core.ViewCx[Terminal]) {
	t.feed(c.data)
	if clip := t.screen.clipboard {
		cx.clipboard_write(clip)
		t.screen.clipboard = none
	}
	if c.exited {
		t.exited = c.exit_code
		t.pty = none
		t.feed('\r\n\x1b[2m[process exited with code ${c.exit_code}]\x1b[0m\r\n'.bytes())
	}
}

// send writes bytes to the program.
pub fn (mut t Terminal) send(data []u8) {
	if mut p := t.pty {
		p.write(data) or {}
	}
}

// close ends the program (the view stays, showing the last screen).
pub fn (mut t Terminal) close() {
	if mut p := t.pty {
		p.close()
	}
}

@[markused]
pub fn (mut t Terminal) render(mut cx core.ViewCx[Terminal]) core.Element {
	return ui.div().id(t.id).test_id(t.id).size_full().cursor(.ibeam).track_focus(t.focus).on_key_down(cx.listener(fn (mut t Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[Terminal]) {
		t.key(e, mut cx)
	})).on_text_input(cx.listener(fn (mut t Terminal, e &core.TextInputEvent, mut cx core.ViewCx[Terminal]) {
		if e.text.len > 0 && e.text[0] >= 0x20 {
			t.type_bytes(e.text.bytes())
			cx.stop_propagation()
		}
	})).on_scroll(cx.listener(fn (mut t Terminal, e &core.ScrollEvent, mut cx core.ViewCx[Terminal]) {
		t.wheel(e, mut cx)
	})).on_mouse_down(cx.listener(fn (mut t Terminal, e &core.MouseDownEvent, mut cx core.ViewCx[Terminal]) {
		t.mouse_down(e, mut cx)
	})).on_mouse_move(cx.listener(fn (mut t Terminal, e &core.MouseMoveEvent, mut cx core.ViewCx[Terminal]) {
		t.mouse_move(e, mut cx)
	})).child(Grid{
		t: t
	})
}

// type_bytes sends typed input and returns the view to the live screen.
fn (mut t Terminal) type_bytes(b []u8) {
	t.offset = 0
	t.sel.clear()
	t.send(b)
}

fn (mut t Terminal) key(e &core.KeyDownEvent, mut cx core.ViewCx[Terminal]) {
	cs := platform.Mods.ctrl | .shift
	if e.mods == cs && e.key == .c {
		t.copy(mut cx)
		cx.stop_propagation()
		return
	}
	if e.mods == cs && e.key == .v {
		if s := cx.clipboard_read() {
			t.type_bytes(paste_bytes(s, t.screen.bracketed))
		}
		cx.stop_propagation()
		return
	}
	if e.mods.has(.shift) && (e.key == .page_up || e.key == .page_down) {
		page := if e.key == .page_up { t.screen.rows - 1 } else { 1 - t.screen.rows }
		t.offset = clamp(t.offset + page, 0, t.screen.scrollback.len())
		cx.stop_propagation()
		return
	}
	if b := encode_key(e.key, e.mods, t.screen.app_cursor) {
		t.type_bytes(b)
		cx.stop_propagation()
	}
}

fn (mut t Terminal) wheel(e &core.ScrollEvent, mut cx core.ViewCx[Terminal]) {
	if t.screen.alt_active || t.cell.height <= 0 {
		return
	}
	lines := int(e.dy / t.cell.height) + if e.dy > 0 {
		1
	} else if e.dy < 0 {
		-1
	} else {
		0
	}
	next := clamp(t.offset + lines, 0, t.screen.scrollback.len())
	if next != t.offset {
		t.offset = next
		cx.stop_propagation()
	}
}
