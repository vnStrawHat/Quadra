module term

fn term_with(cols int, rows int, input string) &Terminal {
	mut t := &Terminal{
		screen: new_screen(cols, rows)
	}
	t.feed(input.bytes())
	return t
}

fn test_a_double_click_stops_at_delimiters_and_a_bracket_selects_its_pair() {
	mut t := term_with(40, 3, 'foo.bar:baz (in side) x')
	t.select_expand(.word, Pos{0, 1})
	t.screen.sel.active = true
	assert t.selected_text() == 'foo.bar'
	t.select_expand(.word, Pos{0, 12}) // the opening bracket: the pair, brackets included
	assert t.selected_text() == '(in side)'
	t.select_expand(.word, Pos{0, 20}) // the closing bracket
	assert t.selected_text() == '(in side)'
}

fn test_a_copied_tab_is_one_tab() {
	mut t := term_with(40, 3, 'a\tb')
	t.screen.sel = Selection{
		anchor: Pos{0, 0}
		head:   Pos{0, 40}
		active: true
	}
	assert t.selected_text() == 'a\tb'
}

fn test_block_selection_copies_a_rectangle() {
	mut t := term_with(10, 3, 'abcd\r\nefgh\r\nijkl')
	t.screen.sel = Selection{
		anchor: Pos{0, 1}
		head:   Pos{2, 3}
		active: true
		block:  true
	}
	assert t.selected_text() == 'bc\nfg\njk'
}

fn test_search_folds_case_whole_words_and_wraps() {
	mut t := term_with(30, 4, 'Foo foobar\r\nfoo bar\r\nĐÂY đây')
	t.search('foo', false, false)
	assert t.find.hits.len == 3
	assert t.find.current == 0
	t.search_step(-1)
	assert t.find.current == 2
	t.search('foo', false, true)
	assert t.find.hits == [Hit{0, 0, 3}, Hit{1, 0, 3}]
	t.search('foo', true, false)
	assert t.find.hits.len == 2 // "foobar" and "foo", not "Foo"
	t.search('đây', false, false)
	assert t.find.hits.len == 2
	t.search_clear()
	assert t.find.hits.len == 0
}

fn test_the_current_match_survives_a_refresh() {
	mut t := term_with(30, 4, 'one\r\none\r\n')
	t.search('one', false, false)
	t.search_step(-1)
	assert t.find.current == 1 // wrapped to the last match
	t.feed('one\r\n'.bytes())
	t.search_refresh()
	assert t.find.hits.len == 3
	assert t.find.current == 1 // the index is kept, like the original's
	t.screen.reset()
	t.search_refresh()
	assert t.find.current == -1
}

fn test_osc_8_links_survive_the_scrollback() {
	mut t := term_with(30, 2, '\x1b]8;;https://a.example\x07go\x1b]8;;\x07 no')
	l := t.link_at(Pos{0, 1}) or { panic('no link') }
	assert l.url == 'https://a.example' && l.c0 == 0 && l.c1 == 2
	assert t.link_at(Pos{0, 4}) == none
	t.feed('\r\n\r\n\r\n'.bytes())
	assert t.screen.scrollback.len() > 0
	h := t.link_at(Pos{0, 0}) or { panic('no link in the history') }
	assert h.url == 'https://a.example'
}

fn test_urls_are_found_in_plain_text() {
	t := term_with(50, 2, 'see (https://x.example/a?b=1). and http://y.test')
	l := t.link_at(Pos{0, 10}) or { panic('no url') }
	assert l.url == 'https://x.example/a?b=1'
	assert t.link_at(Pos{0, 1}) == none
	h := t.link_at(Pos{0, 40}) or { panic('no http url') }
	assert h.url == 'http://y.test'
}

fn test_osc_52_obeys_the_permissions() {
	mut t := term_with(30, 2, '\x1b]52;c;aGk=\x07\x1b]52;c;?\x07')
	assert t.screen.clipboard == none && !t.screen.clipboard_query
	t.screen.allow_osc52, t.screen.allow_osc52_read = true, true
	t.feed('\x1b]52;c;aGk=\x07\x1b]52;c;?\x07'.bytes())
	assert t.screen.clipboard or { '' } == 'hi'
	assert t.screen.clipboard_query
}

fn test_the_bell_and_the_scroll_modes() {
	mut t := term_with(30, 2, '\x07')
	assert t.screen.bell
	assert t.screen.alt_scroll
	t.feed('\x1b[?1007l\x1b[?1003h'.bytes())
	assert !t.screen.alt_scroll && t.screen.mouse == .any
}

fn test_the_cursor_setting_beats_the_program() {
	mut t := term_with(30, 2, '\x1b[6 q')
	assert t.shape_now() == .bar
	t.b.cursor = .underline
	assert t.shape_now() == .underline
}
