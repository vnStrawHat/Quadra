module input

fn test_insert_and_grapheme_delete_vietnamese() {
	mut b := Buffer{}
	b.insert('Tiê')
	b.insert('́') // a combining acute typed separately composes with the ê
	b.insert('ng')
	assert b.text == 'Tiếng'
	b.move_left(false)
	b.move_left(false)
	b.delete_backward() // removes the whole "ế"
	assert b.text == 'Ting'
	assert b.head == 2
}

fn test_selection_movement_and_words() {
	mut b := Buffer{}
	b.set_text('xin chào bạn')
	b.move_word_left(false)
	assert b.head == 'xin chào '.len
	b.move_word_left(true)
	assert b.selected_text() == 'chào '
	b.move_line_start(false)
	b.move_word_right(true)
	assert b.selected_text() == 'xin'
	b.insert('Hello')
	assert b.text == 'Hello chào bạn'
	s, e := b.word_at('Hello ch'.len)
	assert b.text[s..e] == 'chào'
	b.select_all()
	b.delete_word_backward()
	assert b.text == ''
}

fn test_multiline_lines() {
	mut b := Buffer{}
	b.set_text('one\ntwo\nthree')
	b.set_caret(5, false) // inside "two"
	assert b.line_start(b.head) == 4
	assert b.line_end(b.head) == 7
	b.move_line_end(true)
	assert b.selected_text() == 'wo'
}

fn test_history_coalesces_words() {
	mut b := Buffer{}
	mut h := History{}
	for ch in ['h', 'i', ' ', 'y', 'o', 'u'] {
		h.record(&b, .typing, ch == ' ')
		b.insert(ch)
	}
	assert b.text == 'hi you'
	assert h.undo(mut b)
	assert b.text == 'hi'
	assert h.undo(mut b)
	assert b.text == ''
	assert h.redo(mut b)
	assert b.text == 'hi'
	h.record(&b, .other, false)
	b.insert(' there')
	assert !h.redo(mut b) // a new edit clears redo
	assert h.undo(mut b) && b.text == 'hi'
}
