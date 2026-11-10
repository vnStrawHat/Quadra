module term

fn run(cols int, rows int, input string) &Screen {
	mut s := new_screen(cols, rows)
	mut p := Parser{}
	p.feed(mut s, input.bytes())
	return s
}

struct Case {
	input string
	text  string
	x     int
	y     int
}

fn test_text_and_cursor() {
	cases := [
		Case{'hello', 'hello', 5, 0},
		Case{'ab\r\ncd', 'ab\ncd', 2, 1},
		Case{'abcdefghijkl', 'abcdefghij\nkl', 2, 1}, // wraps at 10 columns
		Case{'abcdefghij\r\nx', 'abcdefghij\nx', 1, 1}, // a pending wrap adds no line
		Case{'\x1b[2;3HX', '\n  X', 3, 1},
		Case{'abcdef\x1b[3D\x1b[K', 'abc', 3, 0},
		Case{'abcd\x1b[2J', '', 4, 0},
		Case{'abcdefghi\x1b[1;2H\x1b[2@', 'a  bcdefgh', 1, 0}, // insert pushes i off
		Case{'abcd\x1b[1;2H\x1b[2P', 'ad', 1, 0},
		Case{'abcd\x1b[1;2H\x1b[2X', 'a  d', 1, 0},
		Case{'a\tb', 'a    b'.replace('    ', '       '), 9, 0},
		Case{'ab\x08\x08X', 'Xb', 1, 0},
		Case{'\x1b[5;5H\x1b[2AX', '\n\n    X', 5, 2},
		Case{'xyz\x1b[2GQ', 'xQz', 2, 0},
		Case{'\x1b[3dR', '\n\nR', 1, 2},
		Case{'\x1b[?999zA', 'A', 1, 0}, // unknown private mode: consumed
		Case{'\x1bPqabc\x1b\\B', 'B', 1, 0}, // DCS skipped until ST
		Case{'\x1b(0qx\x1b(Bq', '─│q', 3, 0}, // DEC line drawing
		Case{'Xin chào', 'Xin chào', 8, 0},
		Case{'漢字', '漢字', 4, 0},
		Case{'é!', 'é!', 2, 0}, // combining mark joins the e
		Case{'main\x1b[?1049hALT\x1b[?1049l', 'main', 4, 0},
	]
	for c in cases {
		s := run(10, 5, c.input)
		assert s.text() == c.text, 'input ${c.input.bytes()}: got ${s.text()}'
		assert s.x == c.x && s.y == c.y, 'input ${c.input}: cursor ${s.x},${s.y}'
	}
	assert run(5, 3, 'abcdefg').lines[0].wrapped
	assert run(10, 3, [u8(0xff), `a`].bytestr()).text() == '�a'
}

fn test_scrolling_and_regions() {
	mut s := run(10, 3, '1\r\n2\r\n3\r\n4')
	assert s.text() == '2\n3\n4'
	assert s.scrollback.len() == 1 && s.scrollback.at(0).text == '1'
	// A region scrolls inside itself and keeps the scrollback untouched.
	s = run(10, 4, 'a\r\nb\r\nc\r\nd\x1b[2;3r\x1b[3;1H\nX')
	assert s.text() == 'a\nc\nX\nd'
	assert s.scrollback.len() == 0
	s = run(10, 4, 'a\r\nb\r\nc\r\nd\x1b[2;1H\x1b[L')
	assert s.text() == 'a\n\nb\nc'
	s = run(10, 4, 'a\r\nb\r\nc\r\nd\x1b[2;1H\x1b[M')
	assert s.text() == 'a\nc\nd'
	s = run(10, 3, 'a\r\nb\x1b[1;1H\x1bM')
	assert s.text() == '\na\nb' // reverse index at the top scrolls down
	s = run(10, 3, 'a\r\nb\r\nc\x1b[S')
	assert s.text() == 'b\nc'
}

fn test_sgr_attributes() {
	s := run(20, 2,
		'\x1b[1;31mA\x1b[0mB\x1b[38;2;10;20;30mC\x1b[38:2::1:2:3mD\x1b[48;5;200mE\x1b[0;4:3mF\x1b[94;7mG\x1b[39;49;27;24mH')
	c := s.lines[0].cells
	assert c[0].fg == palette(1) && c[0].flags & f_bold != 0
	assert c[1].fg == 0 && c[1].flags == 0
	assert c[2].fg == rgb_color(10, 20, 30)
	assert c[3].fg == rgb_color(1, 2, 3)
	assert c[4].bg == palette(200)
	assert c[5].flags & f_curly != 0 && c[5].bg == 0
	assert c[6].fg == palette(12) && c[6].flags & f_inverse != 0
	assert c[7].fg == 0 && c[7].flags & (f_inverse | f_underline) == 0
}

fn test_wide_characters_take_two_cells() {
	s := run(5, 2, 'ab漢字')
	assert s.lines[0].cells[2].flags & f_wide != 0
	assert s.lines[0].cells[3].flags & f_spacer != 0
	assert s.text() == 'ab漢\n字' // the second one does not fit in the last column
	// Overwriting half of a wide character blanks the other half.
	t := run(6, 1, '漢\x1b[1;2Hx')
	assert t.lines[0].cells[0].ch == 0 && t.text() == ' x'
}

fn test_replies_title_and_split_input() {
	mut s := run(10, 5, '\x1b[2;3H\x1b[6n\x1b[c')
	assert s.replies.bytestr() == '\x1b[2;3R\x1b[?62;4;22c'
	assert run(10, 1, '\x1b]0;one\x07').title == 'one'
	assert run(10, 1, '\x1b]2;two\x1b\\x').title == 'two'
	// Sequences and UTF-8 may be cut anywhere between two reads.
	s = new_screen(10, 1)
	mut p := Parser{}
	for part in ['\x1b[3', '1mX', [u8(0xc3)].bytestr(), [u8(0xa0)].bytestr()] {
		p.feed(mut s, part.bytes())
	}
	assert s.lines[0].cells[0].fg == palette(1) && s.text() == 'Xà'
	// OSC 52 only reaches the clipboard when allowed.
	assert run(10, 1, '\x1b]52;c;aGk=\x07').clipboard == none
	mut q := new_screen(10, 1)
	q.allow_osc52 = true
	p.feed(mut q, '\x1b]52;c;aGk=\x07'.bytes())
	assert q.clipboard or { '' } == 'hi'
}

fn test_modes() {
	s := run(10, 2, '\x1b[?1h\x1b[?2004h\x1b[?25l\x1b[?1000;1006h\x1b[5 q')
	assert s.app_cursor && s.bracketed && !s.cursor_on && s.mouse == .click && s.mouse_enc == .sgr
	assert s.shape == .bar
	t := run(5, 2, '\x1b[?7labcdefg')
	assert t.text() == 'abcdg' // no auto wrap: the last column is overwritten
}

fn test_scrollback_ring_and_packing() {
	mut sb := Scrollback{
		max: 3
	}
	for i in 0 .. 5 {
		sb.push(Line{
			text: '${i}'
		})
	}
	assert sb.len() == 3 && sb.at(0).text == '2' && sb.at(2).text == '4'
	l := sb.pop() or { panic('') }
	assert l.text == '4' && sb.len() == 2 && sb.at(1).text == '3'
	// pack and unpack keep text, styles, wide cells and marks.
	s := run(10, 1, '\x1b[31mab\x1b[0m漢é')
	row := unpack(pack(s.lines[0]), 10)
	for x in 0 .. 10 {
		assert row.cells[x] == s.lines[0].cells[x], 'cell ${x}'
	}
	assert row.marks == s.lines[0].marks
	assert pack(new_screen(80, 1).lines[0]).runs.len == 0 // blank rows store nothing
}

fn test_resize_reflows_the_main_screen() {
	mut s := run(10, 5, 'abcdefghijkl')
	s.resize(20, 5)
	assert s.text() == 'abcdefghijkl' && s.x == 12 && s.y == 0
	s.resize(4, 5)
	assert s.text() == 'abcd\nefgh\nijkl' && s.y == 2
	s = run(5, 3, 'abcd漢')
	s.resize(3, 3)
	assert s.text() == 'abc\nd漢'
	// Height only: rows move to the scrollback and come back.
	s = run(10, 3, '1\r\n2\r\n3')
	s.resize(10, 2)
	assert s.text() == '2\n3' && s.y == 1 && s.scrollback.len() == 1
	s.resize(10, 3)
	assert s.text() == '1\n2\n3' && s.y == 2 && s.scrollback.len() == 0
	// Blank rows under the cursor go first.
	s = run(10, 4, 'a')
	s.resize(10, 2)
	assert s.text() == 'a' && s.scrollback.len() == 0
	// Width change with history: scrollback lines reflow too.
	s = run(6, 2, 'aaaaaaaaa\r\nb\r\nc')
	s.resize(12, 2)
	assert s.scrollback.len() == 1 && s.scrollback.at(0).text == 'aaaaaaaaa'
	assert s.text() == 'b\nc'
}

fn test_resize_cuts_the_alternate_screen() {
	mut s := run(10, 3, 'main\x1b[?1049h\x1b[Hfull-width') // 1049 keeps the cursor column
	s.resize(4, 2)
	assert s.text() == 'full' && s.x == 3
	mut q := Parser{}
	q.feed(mut s, '\x1b[?1049l'.bytes())
	assert s.text() == 'main' && s.cols == 4
}

fn test_history_reflow_keeps_text_styles_and_spaces() {
	// 10 columns: 'hello ' wraps exactly after a space; the styled word spans the wrap.
	mut s := run(10, 2, 'aaaa hello\x1b[31mworld\x1b[0m end\r\n漢字漢字漢字x\r\nz\r\n\r\n')
	assert s.scrollback.len() >= 3
	s.resize(30, 2)
	mut lines := []string{}
	for i in 0 .. s.scrollback.len() {
		lines << s.scrollback.at(i).text
	}
	assert lines[0] == 'aaaa helloworld end'
	assert lines[1] == '漢字漢字漢字x'
	red := unpack(s.scrollback.at(0), 30)
	assert red.cells[10].fg == palette(1) && red.cells[14].fg == palette(1)
	assert red.cells[15].fg == 0 && red.cells[9].fg == 0
	s.resize(6, 2)
	assert s.scrollback.at(0).text == 'aaaa h' && s.scrollback.at(0).wrapped
	again := unpack(s.scrollback.at(1), 6)
	assert s.scrollback.at(1).text == 'ellowo' && again.cells[4].fg == palette(1)
	assert again.cells[5].fg == palette(1) && again.cells[3].fg == 0 && again.cells[0].fg == 0
	// Spaces at a wrap survive the round trip.
	mut sp := run(6, 2, 'ab    cd\r\n\r\n\r\n')
	sp.resize(20, 2)
	assert sp.scrollback.at(0).text == 'ab    cd'
}

fn test_style_spans_strip_and_color() {
	s, spans :=
		style_spans('\x1b[32mINFO \x1b[0m web: \x1b[1;38;5;208mhot\x1b[m done\x1b]0;title\x07!')
	assert s == 'INFO  web: hot done!'
	assert spans.len == 2
	assert s[spans[0].start..spans[0].end] == 'INFO '
	assert spans[0].pen.fg == palette(2)
	assert s[spans[1].start..spans[1].end] == 'hot'
	assert spans[1].pen.fg == palette(208) && spans[1].pen.flags & f_bold != 0
	plain, none_ := style_spans('no escapes')
	assert plain == 'no escapes' && none_.len == 0
	t, _ := style_spans('\x1b[2K\x1b[1Aup')
	assert t == 'up' // other CSI sequences are dropped
}
