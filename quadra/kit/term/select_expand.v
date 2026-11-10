// Selection expansion: the word or bracket under a double-click and the logical line under a
// triple-click, across soft wraps (specs/17). Positions here are cells, ends inclusive.
module term

const word_stops = [`,`, `│`, `\``, `|`, `:`, `"`, `'`, ` `, `(`, `)`, `[`, `]`, `{`, `}`, `<`,
	`>`, `\t`]
const opening = [`(`, `[`, `{`, `<`]
const closing = [`)`, `]`, `}`, `>`]

pub enum SelExpand {
	none
	word
	lines
}

// Walker reads cells and wrap flags from the history and the screen, keeping the last row.
struct Walker {
	t &Terminal = unsafe { nil }
mut:
	line int = -1
	row  Row
}

fn (mut w Walker) cell(p Pos) Cell {
	w.load(p.line)
	return w.row.cells[p.col]
}

fn (mut w Walker) wrapped(line int) bool {
	w.load(line)
	return w.row.wrapped
}

@[inline]
fn (mut w Walker) load(line int) {
	if line != w.line {
		w.row = w.t.row_at(line)
		w.line = line
	}
}

fn (mut w Walker) scalar(p Pos) rune {
	return if w.cell(p).ch == 0 { rune(` `) } else { rune(w.cell(p).ch) }
}

fn (mut w Walker) is_escape(p Pos) bool {
	c := w.cell(p)
	return c.flags & f_spacer == 0 && (c.ch == 0 || rune(c.ch) in word_stops)
}

fn (w &Walker) last_line() int {
	return w.t.screen.scrollback.len() + w.t.screen.rows - 1
}

fn (w &Walker) next(p Pos) ?Pos {
	if p.col + 1 < w.t.screen.cols {
		return Pos{p.line, p.col + 1}
	}
	if p.line >= w.last_line() {
		return none
	}
	return Pos{p.line + 1, 0}
}

fn (w &Walker) prev(p Pos) ?Pos {
	if p.col > 0 {
		return Pos{p.line, p.col - 1}
	}
	if p.line <= 0 {
		return none
	}
	return Pos{p.line - 1, w.t.screen.cols - 1}
}

// search_left finds the nearest escape to the left (`hit`), or the furthest cell reached.
fn (mut w Walker) search_left(start Pos) (Pos, bool) {
	mut pos := start
	for {
		candidate := w.prev(pos) or { break }
		if candidate.col == w.t.screen.cols - 1 && !w.wrapped(candidate.line) {
			break
		}
		pos = candidate
		if w.is_escape(pos) {
			return pos, true
		}
	}
	return pos, false
}

fn (mut w Walker) search_right(start Pos) (Pos, bool) {
	mut pos := start
	if pos.col == w.t.screen.cols - 1 && !w.wrapped(pos.line) {
		return pos, false
	}
	for {
		candidate := w.next(pos) or { break }
		pos = candidate
		if w.is_escape(pos) {
			return pos, true
		}
		if pos.col == w.t.screen.cols - 1 && !w.wrapped(pos.line) {
			break
		}
	}
	return pos, false
}

// semantic_left is the first cell of the word: past the escape, skipping any spacer.
fn (mut w Walker) semantic_left(p Pos) Pos {
	found, hit := w.search_left(p)
	if !hit {
		return found
	}
	mut cursor := found
	for {
		cursor = w.next(cursor) or { return found }
		if w.cell(cursor).flags & f_spacer == 0 {
			return cursor
		}
	}
	return found
}

fn (mut w Walker) semantic_right(p Pos) Pos {
	found, hit := w.search_right(p)
	if !hit {
		return found
	}
	return w.prev(found) or { found }
}

// bracket_search is the matching bracket, counting nested pairs of the same kind.
fn (mut w Walker) bracket_search(p Pos) ?Pos {
	ch := w.scalar(p)
	i, j := opening.index(ch), closing.index(ch)
	if i < 0 && j < 0 {
		return none
	}
	forward := i >= 0
	other := if forward { closing[i] } else { opening[j] }
	mut nested := 0
	mut cursor := p
	for {
		cursor = if forward { w.next(cursor) or { return none } } else { w.prev(cursor) or {
				return none} }
		c := w.scalar(cursor)
		if c == other && nested == 0 {
			return cursor
		}
		if c == ch {
			nested++
		} else if c == other {
			nested--
		}
	}
	return none
}

// expand_range turns two cells of a word or line selection into the boundaries to select.
fn (t &Terminal) expand_range(kind SelExpand, a Pos, b Pos) (Pos, Pos) {
	mut w := Walker{
		t: t
	}
	mut begin, mut end := if b.before(a) { b, a } else { a, b }
	if kind == .lines {
		for begin.line > 0 && w.wrapped(begin.line - 1) {
			begin = Pos{begin.line - 1, 0}
		}
		for end.line < w.last_line() && w.wrapped(end.line) {
			end = Pos{end.line + 1, 0}
		}
		return Pos{begin.line, 0}, Pos{end.line, t.screen.cols}
	}
	if begin == end {
		if m := w.bracket_search(begin) {
			if m.before(begin) {
				begin = m
			} else {
				end = m
			}
			return begin, Pos{end.line, end.col + 1}
		}
	}
	s := w.semantic_left(begin)
	e := w.semantic_right(end)
	return s, Pos{e.line, e.col + 1}
}

// select_expand starts a word or line selection on a cell.
fn (mut t Terminal) select_expand(kind SelExpand, cell Pos) {
	t.screen.sel.expand, t.screen.sel.origin = kind, cell
	t.screen.sel.anchor, t.screen.sel.head = t.expand_range(kind, cell, cell)
}

// drag_expand re-expands from the first cell of the gesture to the one under the pointer.
fn (mut t Terminal) drag_expand(cell Pos) {
	t.screen.sel.anchor, t.screen.sel.head = t.expand_range(t.screen.sel.expand,
		t.screen.sel.origin, cell)
}
