// Selection anchored to rows; word and line expansion across wraps (specs/17).
module term

fn termw(cols int, rows int, input string) &Terminal {
	mut t := &Terminal{
		screen: new_screen(cols, rows)
	}
	t.feed(input.bytes())
	return t
}

fn sel_lines(mut t Terminal, a Pos, b Pos) {
	t.screen.sel.anchor, t.screen.sel.head, t.screen.sel.active = a, b, true
}

fn test_scrolling_keeps_a_selection_on_its_rows() {
	mut t := termw(10, 3, 'aaa\r\nbbb\r\nccc')
	sel_lines(mut t, Pos{1, 0}, Pos{1, 3}) // bbb
	t.feed('\r\nddd\r\neee'.bytes()) // two rows go to the history
	assert t.screen.sel.active
	assert t.selected_text() == 'bbb'
}

fn test_eviction_clears_a_selection_when_its_row_leaves_the_history() {
	mut t := termw(10, 2, 'a\r\nb\r\nc\r\nd')
	t.screen.scrollback.max = 2 // history holds a, b
	sel_lines(mut t, Pos{1, 0}, Pos{1, 1}) // b
	t.feed('\r\nf'.bytes()) // evicts a: b moves to line 0
	assert t.screen.sel.active
	assert t.screen.sel.anchor.line == 0
	assert t.selected_text() == 'b'
	t.feed('\r\ng'.bytes()) // evicts b
	assert !t.screen.sel.active
}

fn test_a_region_scroll_moves_the_selection_and_kills_what_leaves() {
	mut t := termw(10, 4, 'a\r\nb\r\nc\r\nd')
	t.feed('\x1b[2;4r'.bytes()) // region rows 1..3, off the top: no history
	sel_lines(mut t, Pos{2, 0}, Pos{2, 1}) // c, inside the region
	t.feed('\x1b[4;1H\n'.bytes()) // scrolls the region up one
	assert t.screen.sel.active
	assert t.screen.sel.anchor.line == 1, t.screen.sel.anchor.str()
	sel_lines(mut t, Pos{1, 0}, Pos{1, 1})
	t.feed('\n'.bytes()) // the top row of the region scrolls out
	assert !t.screen.sel.active
}

fn test_a_region_from_the_top_shifts_the_rows_below_it() {
	mut t := termw(10, 4, 'a\r\nb\r\nc\r\nd')
	t.feed('\x1b[1;3r'.bytes()) // region rows 0..2 from the top: rows leave for the history
	sel_lines(mut t, Pos{3, 0}, Pos{3, 1}) // d, below the region
	t.feed('\x1b[3;1H\n'.bytes())
	assert t.screen.sel.anchor.line == 4
	assert t.selected_text() == 'd'
}

fn test_insert_and_delete_lines_move_it() {
	mut t := termw(10, 4, 'a\r\nb\r\nc\r\nd')
	sel_lines(mut t, Pos{2, 0}, Pos{2, 1}) // c
	t.feed('\x1b[1;1H\x1b[L'.bytes())
	assert t.screen.sel.anchor.line == 3
	t.feed('\x1b[1;1H\x1b[M'.bytes())
	assert t.screen.sel.anchor.line == 2
	t.feed('\x1b[3;1H\x1b[M'.bytes()) // deletes the selected row
	assert !t.screen.sel.active
}

fn test_erase_rules() {
	// EL: only the cursor row.
	mut t := termw(10, 4, 'a\r\nb\r\nc\r\nd')
	sel_lines(mut t, Pos{0, 0}, Pos{0, 1})
	t.feed('\x1b[4;1H\x1b[K'.bytes())
	assert t.screen.sel.active
	t.feed('\x1b[1;1H\x1b[K'.bytes())
	assert !t.screen.sel.active
	// ED 0: the cursor row and below.
	sel_lines(mut t, Pos{0, 0}, Pos{1, 1})
	t.feed('\x1b[3;1H\x1b[J'.bytes())
	assert t.screen.sel.active
	t.feed('\x1b[2;1H\x1b[J'.bytes())
	assert !t.screen.sel.active
	// ED 1: the top down to the cursor row.
	t.feed('x\r\ny\r\nz'.bytes())
	sel_lines(mut t, Pos{2, 0}, Pos{3, 1})
	t.feed('\x1b[2;1H\x1b[1J'.bytes())
	assert t.screen.sel.active
	t.feed('\x1b[3;1H\x1b[1J'.bytes())
	assert !t.screen.sel.active
	// ED 2 and the alternate screen always.
	sel_lines(mut t, Pos{3, 0}, Pos{3, 1})
	t.feed('\x1b[2J'.bytes())
	assert !t.screen.sel.active
	sel_lines(mut t, Pos{0, 0}, Pos{0, 1})
	t.feed('\x1b[?1049h'.bytes())
	assert !t.screen.sel.active
	t.feed('\x1b[?1049l'.bytes())
	sel_lines(mut t, Pos{0, 0}, Pos{0, 1})
	t.feed('\x1bc'.bytes())
	assert !t.screen.sel.active
}

fn test_a_width_change_clears_a_height_change_keeps() {
	mut t := termw(10, 4, 'aaaaaa\r\nb')
	sel_lines(mut t, Pos{0, 0}, Pos{0, 6})
	t.screen.resize(10, 6)
	assert t.screen.sel.active
	t.screen.resize(8, 6)
	assert !t.screen.sel.active
}

fn test_double_click_crosses_a_soft_wrap_and_stops_at_a_box_rule() {
	mut t := termw(10, 3, 'abcdefghijklmno pq│r')
	t.select_expand(.word, Pos{1, 2})
	t.screen.sel.active = true
	assert t.selected_text() == 'abcdefghijklmno'
	t.select_expand(.word, Pos{1, 7}) // q, stops at the rule
	assert t.selected_text() == 'pq'
}

fn test_double_click_quirks_of_the_original() {
	// A bracket selects its pair, brackets included, across lines.
	mut t := termw(8, 3, '(ab\r\ncd) x')
	t.select_expand(.word, Pos{0, 0})
	t.screen.sel.active = true
	assert t.screen.sel.anchor == Pos{0, 0} && t.screen.sel.head == Pos{1, 3}
	t.select_expand(.word, Pos{1, 2}) // the closing one
	assert t.screen.sel.anchor == Pos{0, 0} && t.screen.sel.head == Pos{1, 3}
	// A click on a separator selects from the separator before it to the one after.
	mut s := termw(30, 3, 'foo bar baz')
	s.select_expand(.word, Pos{0, 3})
	s.screen.sel.active = true
	assert s.selected_text() == 'foo bar'
}

fn test_triple_click_selects_the_logical_line() {
	mut t := termw(5, 4, 'abcdefghij\r\nx')
	t.select_expand(.lines, Pos{1, 0})
	t.screen.sel.active = true
	assert t.screen.sel.anchor == Pos{0, 0} && t.screen.sel.head == Pos{1, 5}
	assert t.selected_text() == 'abcdefghij'
	t.select_expand(.lines, Pos{2, 0})
	assert t.screen.sel.anchor == Pos{2, 0} && t.screen.sel.head == Pos{2, 5}
}

fn test_a_drag_keeps_the_word_unit() {
	mut t := termw(30, 3, 'one two three')
	t.select_expand(.word, Pos{0, 1})
	t.screen.sel.active = true
	t.drag_expand(Pos{0, 9})
	assert t.selected_text() == 'one two three'
}

fn test_a_match_stays_on_its_row_like_the_original() {
	mut t := termw(5, 4, 'abcdefgh')
	t.search('cdef', false, false)
	assert t.find.hits.len == 0 // "cde" / "f" are on two rows; each row is matched alone
	t.search('fgh', false, false)
	assert t.find.hits == [Hit{1, 0, 3}]
}
