// Editing actions and their default key bindings (context "TextInput").
module input

import quadra.core

pub enum EditOp {
	left
	right
	select_left
	select_right
	word_left
	word_right
	select_word_left
	select_word_right
	line_start
	line_end
	select_line_start
	select_line_end
	doc_start
	doc_end
	up
	down
	select_up
	select_down
	select_all
	copy
	cut
	paste
	undo
	redo
	delete_back
	delete_forward
	delete_word_back
	delete_word_forward
	submit
	newline
}

// Edit is the one action type for all editing operations; users rebind it per op.
pub struct Edit {
pub:
	op EditOp
}

const default_keys = {
	'left':             EditOp.left
	'right':            .right
	'shift-left':       .select_left
	'shift-right':      .select_right
	'ctrl-left':        .word_left
	'ctrl-right':       .word_right
	'ctrl-shift-left':  .select_word_left
	'ctrl-shift-right': .select_word_right
	'home':             .line_start
	'end':              .line_end
	'shift-home':       .select_line_start
	'shift-end':        .select_line_end
	'ctrl-home':        .doc_start
	'ctrl-end':         .doc_end
	'up':               .up
	'down':             .down
	'shift-up':         .select_up
	'shift-down':       .select_down
	'ctrl-a':           .select_all
	'ctrl-c':           .copy
	'ctrl-insert':      .copy
	'ctrl-x':           .cut
	'shift-delete':     .cut
	'ctrl-v':           .paste
	'shift-insert':     .paste
	'ctrl-z':           .undo
	'ctrl-y':           .redo
	'ctrl-shift-z':     .redo
	'backspace':        .delete_back
	'shift-backspace':  .delete_back
	'delete':           .delete_forward
	'ctrl-backspace':   .delete_word_back
	'ctrl-delete':      .delete_word_forward
	'enter':            .submit
}

// install binds the default editing keys once per app (called by new_input).
pub fn install(mut app core.App) ! {
	if app.bindings_for[Edit]().len > 0 {
		return
	}
	for keys, op in default_keys {
		app.bind(keys, Edit{op}, context: 'TextInput')!
	}
	// In multi-line fields enter inserts a newline; ctrl-enter submits.
	app.bind('enter', Edit{.newline}, context: 'TextInput && Multiline')!
	app.bind('shift-enter', Edit{.newline}, context: 'TextInput && Multiline')!
	app.bind('ctrl-enter', Edit{.submit}, context: 'TextInput && Multiline')!
}
