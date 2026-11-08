// Formatting (specs/21, Toolbar): Markdown marks around the selection, line prefixes
// (headings, lists, quotes) and inserted blocks. Every command is one Doc.apply: one undo
// step, and the editor's own change path (highlighting, rows) sees an ordinary edit.
module main

import quadra.kit.code

// format_doc applies cmd at the primary cursor (other cursors are dropped).
fn format_doc(mut d code.Doc, cmd Cmd) {
	d.cursors = [d.primary()]
	match cmd {
		.bold { toggle_mark(mut d, '**') }
		.italic { toggle_mark(mut d, '*') }
		.strike { toggle_mark(mut d, '~~') }
		.code { toggle_mark(mut d, '`') }
		.math { toggle_mark(mut d, '$') }
		.link { insert_link(mut d, false) }
		.image { insert_link(mut d, true) }
		.para, .h1, .h2, .h3, .bullet, .numbered, .task, .quote { set_lines(mut d, cmd) }
		.code_block { fence(mut d, '```') }
		.math_block { fence(mut d, '$$') }
		.diagram { insert_block(mut d, diagram_snippet, 31, 26) }
		.table { insert_block(mut d, table_snippet, 10, 2) }
		.rule { insert_block(mut d, '---', 3, -1) }
		else {}
	}
}

const diagram_snippet = '```mermaid\ngraph TD\n    A[Start] --> B[End]\n```' // "Start" selected

const table_snippet = '| Column 1 | Column 2 |\n| --- | --- |\n|  |  |' // "Column 1" selected

// run_of counts the bytes equal to ch next to p: before it (dir -1) or from it (dir 1).
fn run_of(d &code.Doc, p int, dir int, ch u8) int {
	mut n := 0
	mut i := if dir < 0 { p - 1 } else { p }
	for i >= 0 && i < d.buf.len() && d.buf.byte_at(i) == ch {
		n++
		i += dir
	}
	return n
}

// is_mark: a run of n mark characters is this mark (`*` inside `***` is italic and bold).
fn is_mark(n int, mark string) bool {
	return match mark {
		'*' { n == 1 || n == 3 }
		'**' { n == 2 || n == 3 }
		else { n == mark.len }
	}
}

// toggle_mark wraps the selection (or the word at the caret) in mark, or removes the mark
// when it is already there, just outside or just inside the selection.
fn toggle_mark(mut d code.Doc, mark string) {
	c := d.primary()
	mut a, mut e := c.start(), c.end()
	for a < e && d.buf.byte_at(a) in [` `, `\t`, `\n`] {
		a++
	}
	for e > a && d.buf.byte_at(e - 1) in [` `, `\t`, `\n`] {
		e--
	}
	if c.empty() {
		a, e = d.word_at(c.head)
	}
	k := mark.len
	ch := mark[0]
	if a == e { // nothing to wrap: an empty pair, or remove the empty pair the caret is in
		if is_mark(run_of(d, a, -1, ch), mark) && is_mark(run_of(d, a, 1, ch), mark) {
			d.apply([
				code.Edit{
					a:     a - k
					e:     a + k
					text:  ''
					caret: 0
				},
			], 'format')
		} else {
			d.apply([
				code.Edit{
					a:     a
					e:     a
					text:  mark + mark
					caret: k
				},
			], 'format')
		}
		return
	}
	if is_mark(run_of(d, a, -1, ch), mark) && is_mark(run_of(d, e, 1, ch), mark) {
		d.cursors = [code.Cursor{
			anchor: a
			head:   e
		}]
		d.apply([code.Edit{
			a:    a - k
			e:    a
			text: ''
		}, code.Edit{
			a:    e
			e:    e + k
			text: ''
		}], 'format')
		return
	}
	inner := d.buf.slice(a, e)
	head := run_of(d, a, 1, ch)
	tail := run_of(d, e, -1, ch)
	// Marked just inside (and only at the ends: "**a** and **b**" is wrapped, not unwrapped).
	t := if inner.len > 2 * k && head < inner.len && is_mark(head, mark) && is_mark(tail, mark)
		&& !inner[k..inner.len - k].contains(mark) {
		inner[k..inner.len - k]
	} else {
		mark + inner + mark
	}
	off := if t.len > inner.len { k } else { 0 }
	d.apply([
		code.Edit{
			a:     a
			e:     e
			text:  t
			sel:   off
			caret: t.len - off
		},
	], 'format')
}

// insert_link makes the selection a link (or image): text becomes the label with the URL
// selected to type over; a URL becomes the target with the caret in the empty label.
fn insert_link(mut d code.Doc, image bool) {
	c := d.primary()
	sel := d.buf.slice(c.start(), c.end())
	bang := if image { '!' } else { '' }
	mut t, mut s0, mut s1 := '', 0, 0
	if sel.starts_with('http://') || sel.starts_with('https://') {
		t = bang + '[](' + sel + ')'
		s0 = bang.len + 1
		s1 = s0
	} else {
		label := if sel != '' {
			sel
		} else if image {
			'alt text'
		} else {
			'text'
		}
		url := if image { 'image.png' } else { 'https://' }
		t = bang + '[' + label + '](' + url + ')'
		s0 = if sel == '' { bang.len + 1 } else { bang.len + label.len + 3 }
		s1 = if sel == '' { s0 + label.len } else { s0 + url.len }
	}
	d.apply([
		code.Edit{
			a:     c.start()
			e:     c.end()
			text:  t
			sel:   s0
			caret: s1
		},
	], 'format')
}

// line_range is the lines the selection touches (a selection ending at the start of a
// line does not take that line).
fn line_range(d &code.Doc) (int, int) {
	c := d.primary()
	l0 := d.buf.line_of(c.start())
	mut l1 := d.buf.line_of(c.end())
	if l1 > l0 && c.end() == d.buf.line_start(l1) {
		l1--
	}
	return l0, l1
}
