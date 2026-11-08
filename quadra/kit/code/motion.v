// Cursor movement and selection commands.
module code

import quadra.text

pub enum Move {
	left
	right
	up
	down
	word_left
	word_right
	home
	end
	doc_start
	doc_end
	page_up
	page_down
}

// col_of is the display column of off (tabs reach the next stop).
pub fn (d &Doc) col_of(off int) int {
	line := d.buf.line_of(off)
	s := d.buf.slice(d.buf.line_start(line), off)
	return display_cols(s, d.tab_size)
}

fn display_cols(s string, tab int) int {
	mut col := 0
	for r in s.runes() {
		col = if r == `\t` { (col / tab + 1) * tab } else { col + 1 }
	}
	return col
}

// off_at_col returns the offset on line n closest to display column col.
pub fn (d &Doc) off_at_col(n int, col int) int {
	start := d.buf.line_start(n)
	s := d.buf.line(n)
	mut c := 0
	mut i := 0
	for i < s.len {
		next := text.next_grapheme(s, i)
		w := if s[i] == `\t` { (c / d.tab_size + 1) * d.tab_size - c } else { 1 }
		if c + w > col {
			if col - c > w / 2 && w > 1 {
				i = next
			}
			break
		}
		c += w
		i = next
	}
	return start + i
}

enum CharClass {
	space
	word
	punct
}

fn class_of(b u8) CharClass {
	if b == ` ` || b == `\t` || b == `\n` {
		return .space
	}
	if b >= 0x80 || b.is_letter() || b.is_digit() || b == `_` {
		return .word
	}
	return .punct
}

pub fn (d &Doc) word_right(off int) int {
	n := d.buf.len()
	if off >= n {
		return n
	}
	if d.buf.byte_at(off) == `\n` {
		return off + 1
	}
	mut i := off
	for i < n && d.buf.byte_at(i) in [u8(` `), `\t`] {
		i++
	}
	if i < n && d.buf.byte_at(i) != `\n` {
		k := class_of(d.buf.byte_at(i))
		for i < n && d.buf.byte_at(i) != `\n` && class_of(d.buf.byte_at(i)) == k {
			i++
		}
	}
	return i
}

pub fn (d &Doc) word_left(off int) int {
	if off <= 0 {
		return 0
	}
	if d.buf.byte_at(off - 1) == `\n` {
		return off - 1
	}
	mut i := off
	for i > 0 && d.buf.byte_at(i - 1) in [u8(` `), `\t`] {
		i--
	}
	if i > 0 && d.buf.byte_at(i - 1) != `\n` {
		k := class_of(d.buf.byte_at(i - 1))
		for i > 0 && d.buf.byte_at(i - 1) != `\n` && class_of(d.buf.byte_at(i - 1)) == k {
			i--
		}
	}
	return i
}

fn (d &Doc) grapheme_left(off int) int {
	line := d.buf.line_of(off)
	start := d.buf.line_start(line)
	if off == start {
		return if off > 0 { off - 1 } else { 0 }
	}
	return start + text.prev_grapheme(d.buf.slice(start, off), off - start)
}

fn (d &Doc) grapheme_right(off int) int {
	line := d.buf.line_of(off)
	end := d.buf.line_end(line)
	if off >= end {
		return if off < d.buf.len() { off + 1 } else { off }
	}
	start := d.buf.line_start(line)
	return start + text.next_grapheme(d.buf.line(line), off - start)
}

// first_text is the offset of the first non-blank character of line n.
pub fn (d &Doc) first_text(n int) int {
	s := d.buf.line(n)
	mut i := 0
	for i < s.len && (s[i] == ` ` || s[i] == `\t`) {
		i++
	}
	return d.buf.line_start(n) + i
}

// move moves every cursor; extend keeps the anchor. page is the page height in lines.
pub fn (mut d Doc) move(m Move, extend bool, page int) {
	for mut c in d.cursors {
		if !extend && !c.empty() && m in [.left, .right] {
			p := if m == .left { c.start() } else { c.end() }
			c.anchor, c.head, c.goal = p, p, -1
			continue
		}
		vertical := m in [.up, .down, .page_up, .page_down]
		if vertical && c.goal < 0 {
			c.goal = d.col_of(c.head)
		}
		line := d.buf.line_of(c.head)
		last := d.buf.line_count() - 1
		head := match m {
			.left {
				d.grapheme_left(c.head)
			}
			.right {
				d.grapheme_right(c.head)
			}
			.word_left {
				d.word_left(c.head)
			}
			.word_right {
				d.word_right(c.head)
			}
			.home {
				if c.head == d.first_text(line) {
					d.buf.line_start(line)
				} else {
					d.first_text(line)
				}
			}
			.end {
				d.buf.line_end(line)
			}
			.doc_start {
				0
			}
			.doc_end {
				d.buf.len()
			}
			.up {
				if line == 0 { 0 } else { d.off_at_col(line - 1, c.goal) }
			}
			.down {
				if line == last { d.buf.len() } else { d.off_at_col(line + 1, c.goal) }
			}
			.page_up {
				d.off_at_col(clamp(line - page, 0, last), c.goal)
			}
			.page_down {
				d.off_at_col(clamp(line + page, 0, last), c.goal)
			}
		}

		c.head = head
		if !extend {
			c.anchor = head
		}
		if !vertical {
			c.goal = -1
		}
	}
	d.normalize()
	d.break_group()
}

fn clamp(v int, lo int, hi int) int {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}
