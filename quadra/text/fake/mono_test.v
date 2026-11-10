// layout_mono on the deterministic backend (specs/04): it draws every grapheme one cell
// (6 px) wide, so a wide character is off the grid until the layout snaps it to two cells.
module fake

import quadra.text

const cell = f32(6)
const mono_st = text.TextStyle{
	size:        10
	line_height: 1.5
}

fn mono_sys() &text.TextSystem {
	return text.new_system(new())
}

// pipes_on_grid checks that every `|` of s sits at display_width(s[..i]) × cell.
fn pipes_on_grid(l &text.LineLayout, s string) []int {
	mut cols := []int{}
	for i in 0 .. s.len {
		if s[i] == `|` {
			col := text.display_width(s[..i])
			assert l.x_for_index(i) == f32(col) * cell, '${s} at ${i}'
			cols << col
		}
	}
	return cols
}

fn test_table_rows_keep_the_grid() {
	mut ts := mono_sys()
	rows := [
		'| # | Key   | Name  | Pri | Tag  | State     |',
		"| 3 | F-123 | Việt  | Med | 漢字 | Won't fix |",
		'| 4 | 😀    | tiếng | Lo  | x́y   | ok        |',
	]
	mut want := []int{}
	for r in rows {
		cols := pipes_on_grid(ts.layout_mono(r, mono_st, cell), r)
		if want.len == 0 {
			want = cols.clone()
		}
		assert cols == want // the rows line up
	}
}

fn test_wide_graphemes_take_two_cells() {
	mut ts := mono_sys()
	s := '漢字ab'
	l := ts.layout_mono(s, mono_st, cell)
	assert l.x_for_index(3) == 2 * cell // after 漢
	assert l.x_for_index(6) - l.x_for_index(0) == 4 * cell // a selection over 漢字
	assert l.width == 6 * cell
	assert l.index_for_x(2.4 * cell) == 3
	assert l.index_for_x(1.4 * cell) == 3 // past the middle of 漢
	assert l.index_for_x(0.6 * cell) == 0
	// The one-cell glyph is centred in its two cells; ASCII after it starts on its column.
	assert l.glyphs[0].x == cell / 2
	assert l.glyphs[2].x == 4 * cell
	// Emoji are wide, a letter with a combining mark is one cell.
	e := ts.layout_mono('😀x́!', mono_st, cell)
	assert e.x_for_index('😀'.len) == 2 * cell
	assert e.x_for_index('😀x́'.len) == 3 * cell
}

fn test_ascii_lines_are_the_plain_layout_and_snapped_ones_are_cached() {
	mut ts := mono_sys()
	a := ts.layout_mono('fn main() {}', mono_st, cell)
	assert voidptr(a) == voidptr(ts.layout_line('fn main() {}', mono_st))
	w := ts.layout_mono('漢字', mono_st, cell)
	built := ts.stats.lines_built
	assert voidptr(ts.layout_mono('漢字', mono_st, cell)) == voidptr(w)
	assert ts.stats.lines_built == built
	assert ts.layout_line('漢字', mono_st).width == 2 * cell // the plain layout is untouched
}
