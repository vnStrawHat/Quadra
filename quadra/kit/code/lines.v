// Line operations: indent, delete, duplicate, move, comment. Each works on the lines the
// cursors touch, as one undo group.
module code

// cursor_lines returns the distinct line ranges [first, last] covered by the cursors. A
// selection ending at column 0 does not include that line.
fn (d &Doc) cursor_lines() [][]int {
	mut out := [][]int{}
	for c in d.cursors {
		a := d.buf.line_of(c.start())
		mut e := d.buf.line_of(c.end())
		if e > a && c.end() == d.buf.line_start(e) {
			e--
		}
		if out.len > 0 && a <= out.last()[1] + 1 {
			if e > out.last()[1] {
				out[out.len - 1] = [out.last()[0], e]
			}
		} else {
			out << [a, e]
		}
	}
	return out
}

// indent_lines adds (or removes, outdent) one indent level on every cursor line.
pub fn (mut d Doc) indent_lines(add bool) {
	unit := d.indent_unit()
	mut edits := []Edit{}
	for r in d.cursor_lines() {
		for n in r[0] .. r[1] + 1 {
			start := d.buf.line_start(n)
			if add {
				if d.buf.line_end(n) > start {
					edits << Edit{
						a:    start
						e:    start
						text: unit
					}
				}
				continue
			}
			lead := d.leading(n)
			k := if lead.starts_with('\t') {
				1
			} else {
				mut s := 0
				for s < lead.len && s < d.tab_size && lead[s] == ` ` {
					s++
				}
				s
			}
			if k > 0 {
				edits << Edit{
					a:    start
					e:    start + k
					text: ''
				}
			}
		}
	}
	d.apply(edits, 'indent')
}

// delete_lines removes every cursor line.
pub fn (mut d Doc) delete_lines() {
	mut edits := []Edit{}
	for r in d.cursor_lines() {
		mut a := d.buf.line_start(r[0])
		mut e := if r[1] + 1 < d.buf.line_count() {
			d.buf.line_start(r[1] + 1)
		} else {
			d.buf.len()
		}
		if e == d.buf.len() && a > 0 && r[1] + 1 >= d.buf.line_count() {
			a-- // the last line: take the newline before it
		}
		edits << Edit{
			a:    a
			e:    e
			text: ''
		}
	}
	d.apply(edits, 'delete-line')
}

// duplicate_lines copies the cursor lines below themselves.
pub fn (mut d Doc) duplicate_lines() {
	mut edits := []Edit{}
	for r in d.cursor_lines() {
		a := d.buf.line_start(r[0])
		e := d.buf.line_end(r[1])
		edits << Edit{
			a:    e
			e:    e
			text: '\n' + d.buf.slice(a, e)
		}
	}
	d.apply(edits, 'duplicate')
}

// move_lines moves the cursor lines one line up or down, cursors included.
pub fn (mut d Doc) move_lines(down bool) {
	ranges := d.cursor_lines()
	last := d.buf.line_count() - 1
	if (down && ranges.last()[1] >= last) || (!down && ranges[0][0] == 0) {
		return
	}
	before := d.cursors.clone()
	// The range each cursor belongs to, found before the text changes.
	mut owner := []int{len: before.len}
	for ci, c in before {
		line := d.buf.line_of(c.head)
		for i, r in ranges {
			if line >= r[0] && line <= r[1] + 1 {
				owner[ci] = i
				if line <= r[1] {
					break
				}
			}
		}
	}
	mut edits := []Edit{}
	mut delta := []int{}
	for r in ranges {
		block := d.buf.slice(d.buf.line_start(r[0]), d.buf.line_end(r[1]))
		if down {
			other := d.buf.line(r[1] + 1)
			edits << Edit{
				a:    d.buf.line_start(r[0])
				e:    d.buf.line_end(r[1] + 1)
				text: other + '\n' + block
			}
			delta << other.len + 1
		} else {
			other := d.buf.line(r[0] - 1)
			edits << Edit{
				a:    d.buf.line_start(r[0] - 1)
				e:    d.buf.line_end(r[1])
				text: block + '\n' + other
			}
			delta << -(other.len + 1)
		}
	}
	d.apply(edits, 'move-lines')
	// Cursors travel with their lines (the length of the document part they cross).
	mut out := []Cursor{cap: before.len}
	for ci, c in before {
		k := delta[owner[ci]]
		out << Cursor{
			anchor: c.anchor + k
			head:   c.head + k
		}
	}
	d.cursors = out
	d.normalize()
	if d.undo_stack.len > 0 {
		d.undo_stack[d.undo_stack.len - 1].after = d.cursors.clone()
	}
}

// toggle_comment comments the cursor lines with Doc.comment, or uncomments them when every
// non-blank one is commented.
pub fn (mut d Doc) toggle_comment() {
	mark := d.comment + ' '
	mut edits := []Edit{}
	for r in d.cursor_lines() {
		mut all := true
		mut min_indent := max_int
		for n in r[0] .. r[1] + 1 {
			s := d.buf.line(n)
			t := s.trim_left(' \t')
			if t == '' {
				continue
			}
			if !t.starts_with(d.comment) {
				all = false
			}
			ind := s.len - t.len
			if ind < min_indent {
				min_indent = ind
			}
		}
		if min_indent == max_int {
			continue
		}
		for n in r[0] .. r[1] + 1 {
			s := d.buf.line(n)
			t := s.trim_left(' \t')
			if t == '' {
				continue
			}
			start := d.buf.line_start(n)
			if all {
				ind := s.len - t.len
				k := if t.starts_with(mark) { mark.len } else { d.comment.len }
				edits << Edit{
					a:    start + ind
					e:    start + ind + k
					text: ''
				}
			} else {
				edits << Edit{
					a:    start + min_indent
					e:    start + min_indent
					text: mark
				}
			}
		}
	}
	d.apply(edits, 'comment')
}
