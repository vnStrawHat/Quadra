// Editing behaviour of TextInput: actions, typing, IME and mouse selection.
module input

import quadra.core
import quadra.prim

fn (mut v TextInput) changed(mut cx core.ViewCx[TextInput]) {
	v.last_edit = cx.now_ms()
	if l := v.change {
		value := v.buf.text
		cx.app.invoke(l, &value, cx.win, true)
	}
}

fn (mut v TextInput) moved(mut cx core.ViewCx[TextInput]) {
	v.hist.break_run()
	v.last_edit = cx.now_ms() // the caret is visible right after it moves
}

// type_text inserts committed text (keyboard or IME commit).
pub fn (mut v TextInput) type_text(s_ string, mut cx core.ViewCx[TextInput]) {
	if v.readonly {
		return
	}
	mut s := if v.multiline { s_ } else { s_.replace('\n', ' ') }
	if f := v.filter {
		s = f(s)
	}
	if s == '' {
		return
	}
	v.hist.record(&v.buf, .typing, s == ' ' || s.contains('\n'))
	v.buf.insert(s)
	v.changed(mut cx)
}

fn (mut v TextInput) on_ime(e &core.ImeInputEvent, mut cx core.ViewCx[TextInput]) {
	match e.kind {
		.start {
			v.preedit = ''
		}
		.preedit {
			v.preedit = e.text
			v.preedit_cursor = e.cursor
		}
		.commit {
			v.preedit = ''
			v.type_text(e.text, mut cx)
		}
		.end {
			v.preedit = ''
		}
	}
}

fn (mut v TextInput) edit(kind EditKind, f fn (mut b Buffer), mut cx core.ViewCx[TextInput]) {
	if v.readonly {
		return
	}
	before := v.buf.text
	v.hist.record(&v.buf, kind, false)
	f(mut v.buf)
	if v.buf.text != before {
		v.changed(mut cx)
	}
}

pub fn (mut v TextInput) apply(op EditOp, mut cx core.ViewCx[TextInput]) {
	match op {
		.left {
			v.buf.move_left(false)
		}
		.right {
			v.buf.move_right(false)
		}
		.select_left {
			v.buf.move_left(true)
		}
		.select_right {
			v.buf.move_right(true)
		}
		.word_left {
			v.buf.move_word_left(false)
		}
		.word_right {
			v.buf.move_word_right(false)
		}
		.select_word_left {
			v.buf.move_word_left(true)
		}
		.select_word_right {
			v.buf.move_word_right(true)
		}
		.line_start {
			v.buf.move_line_start(false)
		}
		.line_end {
			v.buf.move_line_end(false)
		}
		.select_line_start {
			v.buf.move_line_start(true)
		}
		.select_line_end {
			v.buf.move_line_end(true)
		}
		.doc_start {
			v.buf.set_caret(0, false)
		}
		.doc_end {
			v.buf.set_caret(v.buf.text.len, false)
		}
		.up, .down, .select_up, .select_down {
			if !v.multiline {
				cx.skip() // single-line fields let arrows reach their owner (lists, steppers)
				return
			}
			dir := if op in [.up, .select_up] { -1 } else { 1 }
			v.buf.set_caret(v.vertical(cx.app.text, dir), op in [.select_up, .select_down])
		}
		.select_all {
			v.buf.select_all()
		}
		.copy {
			if v.buf.has_selection() && !v.password {
				cx.clipboard_write(v.buf.selected_text())
			}
		}
		.cut {
			if v.buf.has_selection() && !v.password {
				cx.clipboard_write(v.buf.selected_text())
				v.edit(.other, fn (mut b Buffer) {
					b.delete_backward()
				}, mut cx)
			}
		}
		.paste {
			if s := cx.clipboard_read() {
				v.hist.record(&v.buf, .other, false)
				v.type_text(s, mut cx)
			}
		}
		.undo {
			if v.hist.undo(mut v.buf) {
				v.changed(mut cx)
			}
		}
		.redo {
			if v.hist.redo(mut v.buf) {
				v.changed(mut cx)
			}
		}
		.delete_back {
			v.edit(.deleting, fn (mut b Buffer) {
				b.delete_backward()
			}, mut cx)
		}
		.delete_forward {
			v.edit(.deleting, fn (mut b Buffer) {
				b.delete_forward()
			}, mut cx)
		}
		.delete_word_back {
			v.edit(.other, fn (mut b Buffer) {
				b.delete_word_backward()
			}, mut cx)
		}
		.delete_word_forward {
			v.edit(.other, fn (mut b Buffer) {
				b.delete_word_forward()
			}, mut cx)
		}
		.submit {
			if v.submit == none {
				cx.skip() // nobody listens: let enter reach the owner (dialogs, comboboxes)
			}
			if l := v.submit {
				value := v.buf.text
				cx.app.invoke(l, &value, cx.win, true)
			}
		}
		.newline {
			v.type_text('\n', mut cx)
		}
	}

	if op !in [.copy, .cut, .paste, .undo, .redo, .delete_back, .delete_forward, .delete_word_back,
		.delete_word_forward, .submit, .newline] {
		v.moved(mut cx)
	}
}

fn (mut v TextInput) mouse_down(e &core.MouseDownEvent, mut cx core.ViewCx[TextInput]) {
	if e.button != .left {
		return
	}
	i := v.index_at(cx.app.text, prim.pt(e.x, e.y))
	if e.clicks == 2 {
		s, end := v.buf.word_at(i)
		v.buf.anchor, v.buf.head = s, end
	} else if e.clicks >= 3 {
		v.buf.anchor, v.buf.head = v.buf.line_start(i), v.buf.line_end(i)
	} else {
		v.buf.set_caret(i, e.mods.has(.shift))
	}
	v.dragging = true
	cx.capture_mouse()
	v.moved(mut cx)
}

fn (mut v TextInput) mouse_drag(e &core.MouseMoveEvent, mut cx core.ViewCx[TextInput]) {
	if !e.pressed || !v.dragging {
		v.dragging = false
		return
	}
	i := v.index_at(cx.app.text, prim.pt(e.x, e.y))
	if i != v.buf.head {
		v.buf.set_caret(i, true)
		cx.notify()
	}
}
