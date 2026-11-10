// Terminal: the view of a session (screen, parser, PTY). Output arrives through a
// core.Inbox; the grid element paints the visible rows (grid.v), selection is in select.v.
module term

import encoding.base64
import time
import quadra.core
import quadra.kit
import quadra.prim
import quadra.platform
import quadra.ui

pub type SpawnFn = fn (opts platform.PtyOptions) !platform.Pty

@[heap]
pub struct Terminal {
pub mut:
	id          string
	screen      &Screen = unsafe { nil }
	parser      Parser
	pty         ?platform.Pty
	exited      ?int
	offset      int // lines scrolled back into the history (0 = following the output)
	focus       core.FocusHandle
	bounds      prim.Rect // the grid, from the last prepaint
	cell        prim.Size
	inbox       &core.Inbox = unsafe { nil }
	b           Behavior
	stats       Stats
	zoom        f32 // px added to the font size (not saved)
	find        Finder
	hover       Link   // the link under the mouse
	pending_url string // the link the confirmation dialog is asking about
	shapes      ShapeCache
	shapes_bold ShapeCache
	contrast    [64]ContrastSlot
	cell_px     [2]int      // device px of a cell (width, height)
	shape_row   []ShapeQuad // this row's coalesced shape quads
	open_a      []int       // indices into shape_row: the previous column's rects
	open_b      []int
	last_shape  int = -2 // column of the last shape cell
	feature_sig string // what font_features was last told
	hover_cell  Pos
	held        int = -1 // the mouse button a program is being told about
	last_cell   Pos
	ringing     bool                     // the bell badge shows
	preedit     string                   // IME text being composed
	eat_text    bool                     // a CSI u key was sent: drop the text event that follows it
	gfx         map[u32]&core.ImageEntry // uploaded images
	gfx_free    []&core.ImageEntry       // tiles of dead images, reused for same-size ones
	sync_timer  bool                     // the update-timeout timer is running
	toast_at    [10]i64                  // OSC 9 rate limit
	toast_n     u64
	hooks       Hooks        // what the app adds (specs/17, App hooks)
	classes     [][]u8       // per visible row: one class byte per column, from hooks.rows
	tints       []CellTint   // class byte -> style, sized by the app
	key_held    map[int]bool // keys whose press went to the program: they get a release
	quiet_exit  bool         // no exit line: the app reports the end itself (a dropped SSH link)
}

@[params]
pub struct TerminalOptions {
pub:
	id               string = 'term'
	cols             int    = 80
	rows             int    = 24
	scrollback       int    = 10000
	product          string = 'Quadra' // what XTVERSION and TN report
	allow_osc52      bool // programs may write the clipboard
	allow_osc52_read bool // programs may read it
	behavior         Behavior
	quiet_exit       bool // no "process exited" line
}

pub fn new_terminal(mut cx core.Cx, opts TerminalOptions) core.View[Terminal] {
	mut s := new_screen(opts.cols, opts.rows)
	s.scrollback.max = opts.scrollback
	s.allow_osc52 = opts.allow_osc52
	s.allow_osc52_read = opts.allow_osc52_read
	s.product_name = opts.product
	return cx.new_view(Terminal{
		b:          opts.behavior
		id:         opts.id
		screen:     s
		focus:      cx.focus_handle()
		quiet_exit: opts.quiet_exit
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

// set_hooks attaches an app's extensions (specs/17, App hooks); call it before the first paint.
pub fn (mut t Terminal) set_hooks(h Hooks, tints []CellTint) {
	t.hooks = h
	t.tints = tints
}

// feed parses output as if the program wrote it (tests, local echo).
pub fn (mut t Terminal) feed(data []u8) {
	before := t.screen.scrollback.len()
	t.parser.feed(mut t.screen, data)
	if t.offset > 0 { // reading the history: stay on the same lines
		t.offset = clamp(t.offset + t.screen.scrollback.len() - before, 0,
			t.screen.scrollback.len())
	}
	t.flush_replies()
}

fn (mut t Terminal) flush_replies() {
	if t.screen.replies.len > 0 {
		t.send(t.screen.replies)
		t.screen.replies.clear()
	}
}

fn (mut t Terminal) receive(c &core.Chunk, mut cx core.ViewCx[Terminal]) {
	title, cwd := t.screen.title, t.screen.cwd
	sw := time.new_stopwatch()
	t.feed(c.data)
	if t.hooks.output != unsafe { nil } {
		t.hooks.output(t.hooks.holder, mut t, c.data)
	}
	t.stats.parse_us += sw.elapsed().microseconds()
	t.stats.bytes += u64(c.data.len)
	t.stats.chunks++
	t.screen.answer_queries(kit.theme_of(cx.app).code.terminal)
	t.flush_replies()
	t.drain_events(mut cx)
	if t.screen.title != title || t.screen.cwd != cwd {
		cx.emit(TerminalInfo{})
	}
	if clip := t.screen.clipboard {
		cx.clipboard_write(clip)
		t.screen.clipboard = none
	}
	if t.screen.clipboard_query {
		t.screen.clipboard_query = false
		if s := cx.clipboard_read() {
			t.send('\x1b]52;c;${base64.encode_str(s)}\x07'.bytes())
		}
	}
	if t.screen.bell {
		t.screen.bell = false
		t.ringing = t.ringing || t.b.bell
	}
	if t.find.query != '' && !t.find.due {
		t.find.due = true
		cx.set_timeout(150, fn (mut t Terminal, mut cx core.ViewCx[Terminal]) {
			t.find.due = false
			t.search_refresh()
			cx.emit(TerminalInfo{})
			cx.notify()
		})
	}
	t.hold_sync(mut cx)
	if c.exited {
		t.exited = c.exit_code
		t.pty = none
		if !t.quiet_exit {
			t.feed('\r\n\x1b[2m[process exited with code ${c.exit_code}]\x1b[0m\r\n'.bytes())
		}
		cx.emit(TerminalInfo{})
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
	root := ui.div().id(t.id).test_id(t.id).size_full().cursor(if t.hover.url != '' {
		platform.Cursor.pointer
	} else {
		platform.Cursor.ibeam
	}).track_focus(t.focus).on_key_down(cx.listener(fn (mut t Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[Terminal]) {
		t.key(e, mut cx)
	})).on_key_up(cx.listener(fn (mut t Terminal, e &core.KeyUpEvent, mut _ core.ViewCx[Terminal]) {
		t.key_up(e)
	})).on_text_input(cx.listener(fn (mut t Terminal, e &core.TextInputEvent, mut cx core.ViewCx[Terminal]) {
		if t.eat_text {
			t.eat_text = false
			cx.stop_propagation()
		} else if e.text.len > 0 && e.text[0] >= 0x20 {
			t.type_bytes(e.text.bytes())
			cx.stop_propagation()
		}
	})).on_scroll(cx.listener(fn (mut t Terminal, e &core.ScrollEvent, mut cx core.ViewCx[Terminal]) {
		t.wheel(e, mut cx)
	})).on_mouse_down(cx.listener(fn (mut t Terminal, e &core.MouseDownEvent, mut cx core.ViewCx[Terminal]) {
		t.mouse_down(e, mut cx)
	})).on_mouse_up(cx.listener(fn (mut t Terminal, e &core.MouseUpEvent, mut cx core.ViewCx[Terminal]) {
		t.mouse_up(e, mut cx)
	})).on_mouse_move(cx.listener(fn (mut t Terminal, e &core.MouseMoveEvent, mut cx core.ViewCx[Terminal]) {
		t.mouse_move(e, mut cx)
	})).on_focus_change(cx.listener(fn (mut t Terminal, e &core.FocusChangeEvent, mut cx core.ViewCx[Terminal]) {
		t.focus_report(e.focused)
	})).on_ime(cx.listener(fn (mut t Terminal, e &core.ImeInputEvent, mut cx core.ViewCx[Terminal]) {
		t.ime(e, mut cx)
	})).child(Grid{
		t: t
	})
	if t.ringing {
		root.child(ui.div().absolute().top(6).right(14).test_id('${t.id}-bell').child(kit.icon(kit.icon_bell).size(14).color(kit.theme(cx).p.warn)))
	}
	if t.hooks.overlay != unsafe { nil } {
		if el := t.hooks.overlay(t.hooks.holder, mut t, mut cx) {
			root.child(el)
		}
	}
	return root
}

// ime shows the composition at the cursor; the alternate screen takes committed text only.
fn (mut t Terminal) ime(e &core.ImeInputEvent, mut cx core.ViewCx[Terminal]) {
	match e.kind {
		.preedit {
			t.preedit = if t.screen.alt_active { '' } else { e.text }
		}
		.commit {
			t.preedit = ''
			t.type_bytes(e.text.bytes())
		}
		else {
			t.preedit = ''
		}
	}

	cx.notify()
}

// type_bytes sends typed input and returns the view to the live screen.
fn (mut t Terminal) type_bytes(b []u8) {
	t.offset = 0
	t.screen.sel.clear()
	t.ringing = false
	t.send(b)
}
