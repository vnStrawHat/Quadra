// Text buffer with a selection, grapheme-aware movement and NFC on insert.
// Offsets are UTF-8 byte indices on grapheme boundaries.
module input

import quadra.text

pub struct Buffer {
pub mut:
	text   string
	anchor int // where the selection started
	head   int // the caret
}

pub fn (b &Buffer) selection() (int, int) {
	return if b.anchor < b.head { b.anchor, b.head } else { b.head, b.anchor }
}

pub fn (b &Buffer) has_selection() bool {
	return b.anchor != b.head
}

pub fn (b &Buffer) selected_text() string {
	s, e := b.selection()
	return b.text[s..e]
}

pub fn (mut b Buffer) set_text(s string) {
	b.text = text.normalize_nfc(s)
	b.anchor, b.head = b.text.len, b.text.len
}

pub fn (mut b Buffer) select_all() {
	b.anchor, b.head = 0, b.text.len
}

// insert replaces the selection with s (NFC-normalized, together with its neighbours so
// a combining mark typed after a letter composes with it).
pub fn (mut b Buffer) insert(s string) {
	start, end := b.selection()
	before := b.text[..start]
	merged := text.normalize_nfc(before + s)
	b.text = merged + b.text[end..]
	b.anchor, b.head = merged.len, merged.len
}

pub fn (mut b Buffer) delete_range(start int, end int) {
	if start >= end {
		return
	}
	b.text = b.text[..start] + b.text[end..]
	b.anchor, b.head = start, start
}

// delete_backward removes the selection, or the grapheme before the caret.
pub fn (mut b Buffer) delete_backward() {
	if b.has_selection() {
		s, e := b.selection()
		b.delete_range(s, e)
		return
	}
	b.delete_range(text.prev_grapheme(b.text, b.head), b.head)
}

pub fn (mut b Buffer) delete_forward() {
	if b.has_selection() {
		s, e := b.selection()
		b.delete_range(s, e)
		return
	}
	b.delete_range(b.head, text.next_grapheme(b.text, b.head))
}

pub fn (mut b Buffer) delete_word_backward() {
	if b.has_selection() {
		b.delete_backward()
		return
	}
	b.delete_range(b.word_left(b.head), b.head)
}

pub fn (mut b Buffer) delete_word_forward() {
	if b.has_selection() {
		b.delete_forward()
		return
	}
	b.delete_range(b.head, b.word_right(b.head))
}

fn (mut b Buffer) move_to(pos int, extend bool) {
	b.head = pos
	if !extend {
		b.anchor = pos
	}
}

// move_left moves one grapheme (or collapses the selection to its start).
pub fn (mut b Buffer) move_left(extend bool) {
	if b.has_selection() && !extend {
		s, _ := b.selection()
		b.move_to(s, false)
		return
	}
	b.move_to(text.prev_grapheme(b.text, b.head), extend)
}

pub fn (mut b Buffer) move_right(extend bool) {
	if b.has_selection() && !extend {
		_, e := b.selection()
		b.move_to(e, false)
		return
	}
	b.move_to(text.next_grapheme(b.text, b.head), extend)
}

pub fn (mut b Buffer) move_word_left(extend bool) {
	b.move_to(b.word_left(b.head), extend)
}

pub fn (mut b Buffer) move_word_right(extend bool) {
	b.move_to(b.word_right(b.head), extend)
}

pub fn (mut b Buffer) move_line_start(extend bool) {
	b.move_to(b.line_start(b.head), extend)
}

pub fn (mut b Buffer) move_line_end(extend bool) {
	b.move_to(b.line_end(b.head), extend)
}

pub fn (mut b Buffer) set_caret(pos int, extend bool) {
	b.move_to(clamp_index(b.text, pos), extend)
}

pub fn (b &Buffer) line_start(pos int) int {
	i := b.text[..pos].last_index_u8(`\n`)
	return if i < 0 { 0 } else { i + 1 }
}

pub fn (b &Buffer) line_end(pos int) int {
	i := b.text[pos..].index_u8(`\n`)
	return if i < 0 { b.text.len } else { pos + i }
}

fn is_word(c u8) bool {
	// Every non-ASCII byte belongs to a word: Vietnamese and other scripts are letters too.
	return c >= 0x80 || (c >= `0` && c <= `9`) || (c >= `a` && c <= `z`)
		|| (c >= `A` && c <= `Z`) || c == `_`
}

// word_left finds the start of the word before pos (skipping spaces first).
pub fn (b &Buffer) word_left(pos int) int {
	mut i := pos
	for i > 0 && !is_word(b.text[i - 1]) {
		i = text.prev_grapheme(b.text, i)
	}
	for i > 0 && is_word(b.text[i - 1]) {
		i = text.prev_grapheme(b.text, i)
	}
	return i
}

pub fn (b &Buffer) word_right(pos int) int {
	mut i := pos
	for i < b.text.len && !is_word(b.text[i]) {
		i = text.next_grapheme(b.text, i)
	}
	for i < b.text.len && is_word(b.text[i]) {
		i = text.next_grapheme(b.text, i)
	}
	return i
}

// word_at returns the word around pos (double-click selection).
pub fn (b &Buffer) word_at(pos int) (int, int) {
	if (pos < b.text.len && is_word(b.text[pos])) || (pos > 0 && is_word(b.text[pos - 1])) {
		mut s := pos
		for s > 0 && is_word(b.text[s - 1]) {
			s = text.prev_grapheme(b.text, s)
		}
		mut e := pos
		for e < b.text.len && is_word(b.text[e]) {
			e = text.next_grapheme(b.text, e)
		}
		return s, e
	}
	return pos, text.next_grapheme(b.text, pos)
}

// clamp_index snaps a byte offset to the nearest grapheme boundary at or before it.
fn clamp_index(s string, pos int) int {
	if pos <= 0 {
		return 0
	}
	if pos >= s.len {
		return s.len
	}
	mut i := 0
	for {
		n := text.next_grapheme(s, i)
		if n > pos {
			return i
		}
		i = n
		if i >= s.len {
			return s.len
		}
	}
	return pos
}
