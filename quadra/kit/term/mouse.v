// Mouse: selection gestures, reports to programs that asked for the mouse, and the wheel.
module term

import quadra.core
import quadra.kit
import quadra.platform

// cell_at is the cell under a point (pos_at gives the nearest boundary between cells).
fn (t &Terminal) cell_at(x f32, y f32) Pos {
	p := t.pos_at(x - t.cell.width / 2, y)
	return Pos{p.line, clamp(p.col, 0, t.screen.cols - 1)}
}

// reporting says whether the program gets this mouse event; shift keeps it for the terminal.
fn (t &Terminal) reporting(mods platform.Mods) bool {
	return t.screen.mouse != .off && !mods.has(.shift)
}

fn mods_code(m platform.Mods) int {
	return (if m.has(.shift) {
		4
	} else {
		0
	}) + (if m.has(.alt) {
		8
	} else {
		0
	}) + (if m.has(.ctrl) {
		16
	} else {
		0
	})
}

// report sends one mouse event in the program's encoding. X10 reports presses only, with
// no modifiers and always in the six-byte form.
fn (mut t Terminal) report(code int, p Pos, press bool) {
	col, row := p.col + 1, p.line - t.top_line() + 1
	if t.screen.mouse == .x10 {
		if press && code < 32 {
			t.send([u8(0x1b), `[`, `M`, u8(32 + (code & 3)), mouse_byte(col),
				mouse_byte(row)])
		}
		return
	}
	match t.screen.mouse_enc {
		.sgr {
			t.send('\x1b[<${code};${col};${row}${if press { 'M' } else { 'm' }}'.bytes())
		}
		.urxvt {
			b := if press { code } else { (code & ~3) | 3 }
			t.send('\x1b[${b + 32};${col};${row}M'.bytes())
		}
		.utf8 {
			b := if press { code } else { (code & ~3) | 3 }
			mut out := [u8(0x1b), `[`, `M`, u8(32 + b)]
			out << mouse_utf8(col)
			out << mouse_utf8(row)
			t.send(out)
		}
		.normal {
			if col < 224 && row < 224 {
				b := if press { code } else { (code & ~3) | 3 }
				t.send([u8(0x1b), `[`, `M`, u8(32 + b), mouse_byte(col),
					mouse_byte(row)])
			}
		}
	}
}

@[inline]
fn mouse_byte(v int) u8 {
	return u8(if v + 32 > 255 {
		255
	} else {
		v + 32
	})
}

fn mouse_utf8(v int) []u8 {
	c := if v + 32 > 0x7ff { 0x7ff } else { v + 32 }
	return rune(c).bytes()
}

// focus_report tells a program that asked (mode 1004) that focus changed.
fn (mut t Terminal) focus_report(focused bool) {
	if t.screen.focus_reports {
		t.send(if focused { '\x1b[I'.bytes() } else { '\x1b[O'.bytes() })
	}
}

fn (mut t Terminal) mouse_down(e &core.MouseDownEvent, mut cx core.ViewCx[Terminal]) {
	t.focus.focus(mut cx.Cx)
	cell := t.cell_at(e.x, e.y)
	if t.reporting(e.mods) {
		code := match e.button {
			.left { 0 }
			.middle { 1 }
			.right { 2 }
			else { -1 }
		}

		if code >= 0 {
			t.held = code + mods_code(e.mods)
			t.last_cell = cell
			t.report(t.held, cell, true)
			cx.capture_mouse()
			cx.stop_propagation()
		}
		return
	}
	if e.button == .middle && t.b.middle_paste {
		t.paste_clipboard(mut cx.Cx)
		cx.stop_propagation()
		return
	}
	if e.button != .left {
		return
	}
	if e.mods.has(.ctrl) {
		if l := t.link_at(cell) {
			t.open_link(l, mut cx)
			cx.stop_propagation()
			return
		}
	}
	p := t.pos_at(e.x, e.y)
	alt := e.mods.has(.alt)
	t.screen.sel.block, t.screen.sel.expand = alt, .none
	if alt {
		if !(e.mods.has(.shift) && t.screen.sel.active) {
			t.screen.sel.anchor = p
		}
		t.screen.sel.head = p
	} else if e.clicks == 2 {
		t.select_expand(.word, cell)
	} else if e.clicks >= 3 {
		t.select_expand(.lines, cell)
	} else {
		if !(e.mods.has(.shift) && t.screen.sel.active) {
			t.screen.sel.anchor = p
		}
		t.screen.sel.head = p
	}

	t.screen.sel.active, t.screen.sel.dragging = true, true
	cx.capture_mouse()
	cx.stop_propagation()
}

fn (mut t Terminal) mouse_up(e &core.MouseUpEvent, mut cx core.ViewCx[Terminal]) {
	if t.held >= 0 {
		t.report(t.held, t.cell_at(e.x, e.y), false)
		t.held = -1
		return
	}
	if t.screen.sel.dragging {
		t.end_drag(mut cx)
	}
}

fn (mut t Terminal) end_drag(mut cx core.ViewCx[Terminal]) {
	t.screen.sel.dragging = false
	if t.screen.sel.anchor == t.screen.sel.head {
		t.screen.sel.active = false
	} else if t.b.copy_on_select {
		t.copy(mut cx)
	}
	cx.notify()
}

fn (mut t Terminal) mouse_move(e &core.MouseMoveEvent, mut cx core.ViewCx[Terminal]) {
	cell := t.cell_at(e.x, e.y)
	moving := t.screen.mouse == .any && !e.mods.has(.shift)
	if t.held >= 0 || moving {
		if t.screen.mouse != .click && cell != t.last_cell {
			t.last_cell = cell
			base := if t.held >= 0 { t.held } else { 3 + mods_code(e.mods) }
			t.report(32 + base, cell, true)
		}
		return
	}
	if e.pressed {
		if t.screen.sel.dragging {
			if t.screen.sel.expand == .none {
				t.screen.sel.head = t.pos_at(e.x, e.y)
			} else {
				t.drag_expand(cell)
			}
			cx.notify()
		}
		return
	}
	if t.screen.sel.dragging { // the button went up outside the window
		t.end_drag(mut cx)
	}
	if cell != t.hover_cell {
		t.hover_cell = cell
		l := t.link_at(cell) or { Link{} }
		if l != t.hover {
			t.hover = l
			cx.notify()
		}
	}
}

// wheel_steps is how many lines a wheel movement is: at least one.
fn (t &Terminal) wheel_steps(dy f32) int {
	n := int((if dy < 0 { -dy } else { dy }) / t.cell.height * t.b.wheel + 0.5)
	return if n < 1 { 1 } else { n }
}

fn (mut t Terminal) wheel(e &core.ScrollEvent, mut cx core.ViewCx[Terminal]) {
	if t.cell.height <= 0 || e.dy == 0 {
		return
	}
	steps := t.wheel_steps(e.dy)
	if t.reporting(e.mods) {
		cell := t.cell_at(e.x, e.y)
		for _ in 0 .. steps {
			t.report((if e.dy > 0 { 64 } else { 65 }) + mods_code(e.mods), cell, true)
		}
		cx.stop_propagation()
		return
	}
	if t.screen.alt_active {
		if t.b.alternate_scroll && t.screen.alt_scroll {
			lead := if t.screen.app_cursor { '\x1bO' } else { '\x1b[' }
			arrow := lead + if e.dy > 0 { 'A' } else { 'B' }
			t.send(arrow.repeat(steps).bytes())
			cx.stop_propagation()
		}
		return
	}
	next := clamp(t.offset + if e.dy > 0 { steps } else { -steps }, 0, t.screen.scrollback.len())
	if next != t.offset {
		t.offset = next
		cx.notify()
		cx.stop_propagation()
	}
}

pub const max_paste = 1 << 20

// paste_clipboard pastes the clipboard, refusing more than max_paste with a warning.
pub fn (mut t Terminal) paste_clipboard(mut cx core.Cx) {
	s := cx.clipboard_read() or { return }
	if s.len > max_paste {
		kit.notify(mut cx, .warning, 'Paste too large', 'The clipboard holds more than 1 MiB.')
		return
	}
	t.paste(s)
}
