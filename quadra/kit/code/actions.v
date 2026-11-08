// Editor actions and their default keys (context "Editor"; not inside the find bar's
// inputs, which keep their own editing keys).
module code

import quadra.core

pub enum Op {
	move
	select_all
	select_line
	select_next
	add_cursor_up
	add_cursor_down
	undo
	redo
	copy
	cut
	paste
	delete_back
	delete_forward
	delete_word_back
	delete_word_forward
	newline
	tab
	outdent
	delete_line
	duplicate
	lines_up
	lines_down
	comment
	find
	replace
	find_next
	find_prev
	escape
}

pub struct EditorAction {
pub:
	op     Op
	move   Move
	extend bool
}

fn mv(m Move, extend bool) EditorAction {
	return EditorAction{
		op:     .move
		move:   m
		extend: extend
	}
}

fn editor_keys() map[string]EditorAction {
	mut k := map[string]EditorAction{}
	moves := {
		'left':       Move.left
		'right':      .right
		'up':         .up
		'down':       .down
		'ctrl-left':  .word_left
		'ctrl-right': .word_right
		'home':       .home
		'end':        .end
		'ctrl-home':  .doc_start
		'ctrl-end':   .doc_end
		'pageup':     .page_up
		'pagedown':   .page_down
	}
	for key, m in moves {
		k[key] = mv(m, false)

		k[if key.starts_with('ctrl-') {
			'ctrl-shift-' + key[5..]
		} else {
			'shift-' + key
		}] = mv(m, true)
	}
	k['alt-shift-left'] = mv(.left, true)
	k['alt-shift-right'] = mv(.right, true)
	ops := {
		'ctrl-a':          Op.select_all
		'ctrl-l':          .select_line
		'ctrl-d':          .select_next
		'ctrl-alt-up':     .add_cursor_up
		'ctrl-alt-down':   .add_cursor_down
		'alt-shift-up':    .add_cursor_up
		'alt-shift-down':  .add_cursor_down
		'ctrl-z':          .undo
		'ctrl-y':          .redo
		'ctrl-shift-z':    .redo
		'ctrl-c':          .copy
		'ctrl-insert':     .copy
		'ctrl-x':          .cut
		'shift-delete':    .cut
		'ctrl-v':          .paste
		'shift-insert':    .paste
		'backspace':       .delete_back
		'shift-backspace': .delete_back
		'delete':          .delete_forward
		'ctrl-backspace':  .delete_word_back
		'ctrl-delete':     .delete_word_forward
		'enter':           .newline
		'shift-enter':     .newline
		'tab':             .tab
		'shift-tab':       .outdent
		'ctrl-shift-k':    .delete_line
		'ctrl-shift-d':    .duplicate
		'alt-up':          .lines_up
		'alt-down':        .lines_down
		'ctrl-/':          .comment
	}
	for key, op in ops {
		k[key] = EditorAction{
			op: op
		}
	}
	return k
}

// install binds the default editor keys once per app (called by new_editor).
pub fn install(mut app core.App) ! {
	if app.bindings_for[EditorAction]().len > 0 {
		return
	}
	for keys, a in editor_keys() {
		app.bind(keys, a, context: 'Editor && !TextInput')!
	}
	// These work inside the find bar too.
	app.bind('f3', EditorAction{ op: .find_next },
		context: 'Editor'
	)!
	// Enter in the query input goes to the next match (on_submit); shift-enter goes back.
	app.bind('shift-enter', EditorAction{ op: .find_prev },
		context: 'Editor && TextInput'
	)!
	app.bind('shift-f3', EditorAction{ op: .find_prev },
		context: 'Editor'
	)!
	app.bind('escape', EditorAction{ op: .escape },
		context: 'Editor'
	)!
	app.bind('ctrl-f', EditorAction{ op: .find },
		context: 'Editor'
	)!
	app.bind('ctrl-h', EditorAction{ op: .replace },
		context: 'Editor'
	)!
}

const edit_ops = [Op.cut, .paste, .delete_back, .delete_forward, .delete_word_back,
	.delete_word_forward, .newline, .tab, .outdent, .delete_line, .duplicate, .lines_up, .lines_down,
	.comment, .undo, .redo]

pub fn (mut e Editor) run(a &EditorAction, mut cx core.ViewCx[Editor]) {
	if e.readonly && a.op in edit_ops {
		return
	}
	v := e.doc.version
	mut d := e.doc
	match a.op {
		.move {
			if e.wrap && a.move in [.up, .down, .page_up, .page_down] {
				step := if a.move in [.up, .down] { 1 } else { e.page() }
				e.move_rows(if a.move in [.up, .page_up] { -step } else { step }, a.extend)
			} else {
				d.move(a.move, a.extend, e.page())
			}
		}
		.select_all {
			d.select_all()
		}
		.select_line {
			d.select_line()
		}
		.select_next {
			d.select_next()
		}
		.add_cursor_up {
			d.add_cursor_vertical(false)
		}
		.add_cursor_down {
			d.add_cursor_vertical(true)
		}
		.undo {
			d.undo()
		}
		.redo {
			d.redo()
		}
		.copy {
			cx.clipboard_write(d.copy_text())
		}
		.cut {
			cx.clipboard_write(d.cut())
		}
		.paste {
			if s := cx.clipboard_read() {
				d.paste(s)
			}
		}
		.delete_back {
			d.delete_back(false)
		}
		.delete_forward {
			d.delete_forward(false)
		}
		.delete_word_back {
			d.delete_back(true)
		}
		.delete_word_forward {
			d.delete_forward(true)
		}
		.newline {
			d.newline()
		}
		.tab {
			d.tab()
		}
		.outdent {
			d.indent_lines(false)
		}
		.delete_line {
			d.delete_lines()
		}
		.duplicate {
			d.duplicate_lines()
		}
		.lines_up {
			d.move_lines(false)
		}
		.lines_down {
			d.move_lines(true)
		}
		.comment {
			d.toggle_comment()
		}
		.find, .replace {
			e.open_find(a.op == .replace, mut cx)
		}
		.find_next, .find_prev {
			e.find.next(mut d, a.op == .find_prev)
		}
		.escape {
			if e.find_open {
				e.close_find(mut cx)
			} else if d.cursors.len > 1 || !d.primary().empty() {
				p := d.primary()
				d.set_cursor(p.head, p.head, false)
			} else {
				cx.skip()
				return
			}
		}
	}

	cx.stop_propagation()
	e.edited(v, mut cx)
}
