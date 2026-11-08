module code

// doc builds a document; '|' marks cursors and '[...]' a selection (anchor at '[').
fn doc(s string) &Doc {
	mut d := new_doc('')
	mut text := []u8{}
	mut cs := []Cursor{}
	mut anchor := -1
	for b in s.bytes() {
		match b {
			`|` {
				cs << at(text.len)
			}
			`[` {
				anchor = text.len
			}
			`]` {
				cs << Cursor{
					anchor: anchor
					head:   text.len
				}
			}
			else {
				text << b
			}
		}
	}
	d.buf = new_buffer(text.bytestr())
	d.cursors = if cs.len > 0 { cs } else { [Cursor{}] }
	d.normalize()
	mut t := i64(0)
	d.clock = fn [mut t] () i64 {
		t += 10
		return t
	}
	return d
}

// show renders the document with its cursors in the same notation.
fn show(d &Doc) string {
	s := d.text()
	mut marks := map[int]string{}
	for c in d.cursors {
		if c.empty() {
			marks[c.head] = marks[c.head] + '|'
		} else {
			marks[c.start()] = marks[c.start()] + '['
			marks[c.end()] = ']' + marks[c.end()]
		}
	}
	mut out := ''
	for i in 0 .. s.len + 1 {
		out += marks[i] or { '' }
		if i < s.len {
			out += s[i..i + 1]
		}
	}
	return out
}

fn test_motion() {
	mut d := doc('ab|c\nxy')
	d.move(.right, false, 0)
	assert show(d) == 'abc|\nxy'
	d.move(.right, false, 0)
	assert show(d) == 'abc\n|xy'
	d.move(.up, false, 0)
	assert show(d) == '|abc\nxy'
	d = doc('foo bar|.baz')
	d.move(.word_left, false, 0)
	assert show(d) == 'foo |bar.baz'
	d.move(.word_right, true, 0)
	assert show(d) == 'foo [bar].baz'
	d.move(.left, false, 0)
	assert show(d) == 'foo |bar.baz' // collapses to the start
	d = doc('    if x|')
	d.move(.home, false, 0)
	assert show(d) == '    |if x'
	d.move(.home, false, 0)
	assert show(d) == '|    if x'
	// The goal column survives a short line.
	d = doc('abcdef|\nx\nabcdefgh')
	d.move(.down, false, 0)
	d.move(.down, false, 0)
	assert show(d) == 'abcdef\nx\nabcdef|gh'
	// Graphemes: one step over a combined character.
	d = doc('|éx')
	d.move(.right, false, 0)
	assert show(d) == 'é|x'
}

fn test_typing_and_undo_groups() {
	mut d := doc('|')
	for ch in 'hello'.split('') {
		d.insert_text(ch, 'type')
	}
	d.newline()
	d.insert_text('x', 'type')
	assert show(d) == 'hello\nx|'
	assert d.undo() && show(d) == 'hello\n|'
	assert d.undo() && show(d) == 'hello|'
	assert d.undo() && show(d) == '|' // the five keystrokes are one group
	assert !d.undo()
	assert d.redo() && show(d) == 'hello|'
	// A cursor move ends the group.
	d = doc('|')
	d.insert_text('a', 'type')
	d.move(.left, false, 0)
	d.move(.right, false, 0)
	d.insert_text('b', 'type')
	d.undo()
	assert show(d) == 'a|'
}

fn test_newline_indent_and_deletes() {
	mut d := doc('fn main() {|}')
	d.newline()
	assert show(d) == 'fn main() {\n    |}'
	d = doc('\tx := 1|')
	d.newline()
	assert show(d) == '\tx := 1\n\t|'
	d = doc('ab|c')
	d.delete_back(false)
	assert show(d) == 'a|c'
	d = doc('foo bar|')
	d.delete_back(true)
	assert show(d) == 'foo |'
	d = doc('a|b\nc')
	d.delete_forward(false)
	d.delete_forward(false)
	assert show(d) == 'a|c'
	d = doc('a[bc]d')
	d.delete_back(false)
	assert show(d) == 'a|d'
}

fn test_multiple_cursors() {
	mut d := doc('a|\nb|\nc|')
	d.insert_text('!', 'type')
	assert show(d) == 'a!|\nb!|\nc!|'
	d.delete_back(false)
	d.delete_back(false)
	assert show(d) == '|\n|\n|'
	d = doc('x = foo|; y = foo; foo')
	d.select_next()
	assert show(d) == 'x = [foo]; y = foo; foo'
	d.select_next()
	d.select_next()
	assert show(d) == 'x = [foo]; y = [foo]; [foo]'
	d.insert_text('bar', 'type')
	assert d.text() == 'x = bar; y = bar; bar'
	d = doc('ab|c\nabc\nabc')
	d.add_cursor_vertical(true)
	d.add_cursor_vertical(true)
	d.insert_text('-', 'type')
	assert show(d) == 'ab-|c\nab-|c\nab-|c'
	// Cursors that meet merge.
	d = doc('a|b|c')
	d.delete_back(false)
	d.delete_back(false)
	assert show(d) == '|c'
}

fn test_line_operations() {
	mut d := doc('one\nt|wo\nthree')
	d.move_lines(true)
	assert show(d) == 'one\nthree\nt|wo'
	d.move_lines(false)
	d.move_lines(false)
	assert show(d) == 't|wo\none\nthree'
	d.duplicate_lines()
	assert d.text() == 'two\ntwo\none\nthree'
	d = doc('a\nb|\nc')
	d.delete_lines()
	assert show(d) == 'a\n|c'
	d = doc('a\nb|')
	d.delete_lines()
	assert d.text() == 'a'
	d = doc('[x\ny]')
	d.indent_lines(true)
	assert d.text() == '    x\n    y'
	d.indent_lines(false)
	assert d.text() == 'x\ny'
	d = doc('  [a\n  b]')
	d.toggle_comment()
	assert d.text() == '  // a\n  // b'
	d.toggle_comment()
	assert d.text() == '  a\n  b'
	d = doc('a|')
	d.tab()
	assert show(d) == 'a   |'
}

fn test_clipboard() {
	mut d := doc('one|\ntwo')
	assert d.copy_text() == 'one\n' // no selection: the line
	assert d.cut() == 'one\n' && d.text() == 'two'
	d = doc('[ab] [cd]')
	assert d.copy_text() == 'ab\ncd'
	d = doc('x|\ny|')
	d.paste('1\n2')
	assert show(d) == 'x1|\ny2|' // one line per cursor
	d = doc('|')
	d.paste('a\r\nb')
	assert show(d) == 'a\nb|'
}

fn test_find_and_replace() {
	mut d := doc('|Foo foo food\nfoo')
	mut f := Find{
		query: 'foo'
	}
	assert f.matches(d).len == 8 // case-insensitive by default, inside words too
	f.word = true
	assert f.matches(d) == [0, 3, 4, 7, 13, 16]
	f.case = true
	assert f.matches(d) == [4, 7, 13, 16]
	assert f.next(mut d, false) && show(d) == 'Foo [foo] food\nfoo'
	assert f.current(d) == 0
	f.next(mut d, false)
	f.next(mut d, false) // wraps
	assert f.current(d) == 0
	f.next(mut d, true)
	assert f.current(d) == 1
	f.replace(mut d, 'x')
	assert d.text() == 'Foo foo food\nx' && f.current(d) == 0
	d = doc('|a1 b22 c333')
	f = Find{
		query: r'\d+'
		regex: true
	}
	assert f.replace_all(mut d, '#') == 3
	assert d.text() == 'a# b# c#'
	assert d.undo() && d.text() == 'a1 b22 c333' // one group
	f.query = '(' // a bad pattern reports, not panics
	assert f.matches(d).len == 0 && f.error() != ''
}
