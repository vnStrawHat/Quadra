// URL detection across wraps, with the editor's and Markdown's rules (specs/22, Links).
module term

fn turl(cols int, rows int, input string) &Terminal {
	mut t := &Terminal{
		screen: new_screen(cols, rows)
	}
	t.feed(input.bytes())
	return t
}

fn test_a_url_follows_a_soft_wrap() {
	mut t := turl(20, 4, 'see https://example.com/a/b/c.txt, ok')
	l := t.link_at(Pos{1, 3}) or { panic('no url') }
	assert l.url == 'https://example.com/a/b/c.txt'
	assert l.line == 0 && l.c0 == 4 && l.end_line == 1 && l.c1 == 13
	// The same link answers from both of its rows.
	first := t.link_at(Pos{0, 5}) or { panic('no url') }
	assert first == l
}

fn test_schemes_www_and_non_ascii() {
	mut t := turl(40, 4, 'ftp://a.org/x http://b.org www.c.org\r\nhttps://d.org/đường/路')
	assert t.link_at(Pos{0, 3}) == none // ftp is no link any more (it never opened)
	assert t.link_cols([t.row_at(0)]).map(it.c0) == [14, 27] // nor colored as one
	www := t.link_at(Pos{0, 30}) or { panic('www') }
	assert www.url == 'https://www.c.org'
	uni := t.link_at(Pos{1, 16}) or { panic('non-ascii') }
	assert uni.url == 'https://d.org/đường/路'
}

fn test_punctuation_and_spaces_end_a_url() {
	mut t := turl(50, 2, '(http://a.b/c). <https://d.e> "http://f.g"')
	assert (t.link_at(Pos{0, 5}) or { panic('a') }).url == 'http://a.b/c'
	assert (t.link_at(Pos{0, 20}) or { panic('d') }).url == 'https://d.e'
	assert t.link_at(Pos{0, 0}) == none // the opening parenthesis is outside
	// A click on the stripped punctuation still opens the URL, as in the original.
	assert (t.link_at(Pos{0, 14}) or { panic('p') }).url == 'http://a.b/c'
	assert t.link_at(Pos{0, 15}) == none // the space after it
}

fn test_an_osc8_link_keeps_its_label() {
	mut t := turl(40, 2, '\x1b]8;;https://evil.com\x1b\\good.com\x1b]8;;\x1b\\ rest')
	l := t.link_at(Pos{0, 2}) or { panic('no link') }
	assert l.osc8 && l.url == 'https://evil.com' && l.display == 'good.com'
	assert validate_target_with_display(l.url, l.display).verdict == .confirm
}

fn test_link_cols_colors_a_url_across_a_wrap() {
	mut t := turl(20, 4, 'see https://example.com/a/b/c.txt, ok')
	rows := [t.row_at(0), t.row_at(1)]
	assert rows[0].wrapped
	spans := t.link_cols(rows)
	// The first row ends at the wrap, the second stops before the trailing comma.
	assert spans[0].line == 0 && spans[0].c0 == 4 && spans[0].c1 == 20
	assert spans[1].line == 1 && spans[1].c0 == 0 && spans[1].c1 == 13
}

fn test_a_wrap_that_lands_inside_a_url_keeps_a_period() {
	// `x.` at the end of the first row is not trailing punctuation: the URL runs on.
	mut t := turl(12, 3, 'www.example.com')
	rows := [t.row_at(0), t.row_at(1)]
	spans := t.link_cols(rows)
	assert spans[0].c1 == 12 && spans[1].c0 == 0 && spans[1].c1 == 3
}

fn test_a_url_continues_onto_a_row_with_no_colon_or_period() {
	// The second row alone could hold no link (scan_row skips it); the wrap carries one in.
	mut t := turl(20, 3, 'see https://example.com/abcdefghij')
	rows := [t.row_at(0), t.row_at(1)]
	assert !t.may_link(rows[1])
	spans := t.link_cols(rows)
	assert spans.len == 2 && spans[1].line == 1 && spans[1].c0 == 0 && spans[1].c1 == 14
}

fn test_a_balanced_paren_stays_and_a_scheme_needs_a_word_boundary() {
	mut t := turl(60, 2, '(see https://w.org/Foo_(bar)) xhttps://a.b <https://c.d>x')
	l := t.link_at(Pos{0, 10}) or { panic('no url') }
	assert l.url == 'https://w.org/Foo_(bar)' && l.c0 == 5 && l.c1 == 28
	assert t.link_at(Pos{0, 33}) == none // glued to a word
	assert (t.link_at(Pos{0, 47}) or { panic('c') }).url == 'https://c.d'
	spans := t.link_cols([t.row_at(0)])
	assert spans.map(it.c1) == [28, 55] // the coloring keeps the `)` too
}

fn test_a_balanced_paren_across_a_wrap() {
	// The `(` ends the first row, its `)` is on the second.
	mut t := turl(20, 3, 'see https://w.org/F(bar) ok')
	l := t.link_at(Pos{1, 1}) or { panic('no url') }
	assert l.url == 'https://w.org/F(bar)' && l.end_line == 1 && l.c1 == 4
	spans := t.link_cols([t.row_at(0), t.row_at(1)])
	assert spans[1].c0 == 0 && spans[1].c1 == 4
}
