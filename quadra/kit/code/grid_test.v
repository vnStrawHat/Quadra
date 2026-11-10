// The monospace grid in the editor (specs/18, View): carets, clicks, up/down, column
// selection, tab stops and soft wrap all count a wide character as two cells. The fake
// text backend draws every grapheme one cell wide, so wide ones are off the grid unsnapped.
module code

import quadra.prim
import quadra.text
import quadra.text.fake

fn grid_editor(s string) (&text.TextSystem, &Editor) {
	mut ts := text.new_system(fake.new())
	st := text.TextStyle{
		size:        10
		line_height: 1.5
	}
	cell := ts.layout_line('M', st).width
	e := &Editor{
		doc:    new_doc(s)
		style:  st
		cell:   prim.Size{cell, 15}
		bounds: prim.rect(100, 0, 800, 400)
	}
	return ts, e
}

fn test_caret_and_clicks_count_two_cells_per_wide_character() {
	mut ts, mut e := grid_editor('漢字ab\n| 漢字 | x |')
	w := e.cell.width
	x, r := e.caret(ts, 3) // after 漢
	assert x == 2 * w && r == 0
	x2, _ := e.caret(ts, 7) // after 字a
	assert x2 == 5 * w
	y := e.cell.height / 2
	assert e.offset_at(ts, 100 + 2.4 * w, y) == 3
	assert e.offset_at(ts, 100 + 1.4 * w, y) == 3 // the half rule: past the middle of 漢
	assert e.offset_at(ts, 100 + 0.6 * w, y) == 0
	// The URL underline and the selection read the same layout.
	rects := e.link_rects(ts, 0, 0, 6)
	assert rects.len == 1 && rects[0].x == 100 && rects[0].width == 4 * w
	// The second line's last pipe is at its display column.
	line := e.doc.buf.line(1)
	x3, _ := e.caret(ts, e.doc.buf.line_start(1) + line.len - 1)
	assert x3 == f32(text.display_width(line) - 1) * w
}

fn test_columns_count_display_width() {
	mut d := new_doc('漢字x\nabcdefgh\n\t漢\tz')
	assert d.col_of(6) == 4 // after 漢字
	assert d.off_at_col(0, 1) == 0 // inside 漢: the half rule keeps it before
	assert d.off_at_col(0, 2) == 3
	d.set_cursor(6, 6, false)
	d.move(.down, false, 0)
	assert d.primary().head == d.buf.line_start(1) + 4 // the goal column is 4, not 2
	d.move(.up, false, 0)
	assert d.primary().head == 6
	// A tab after a wide character reaches the stop after its two cells.
	dl := display_line('漢\tz', 4)
	assert dl.shown == '漢  z'
	assert text.display_width(dl.shown[..dl.to_shown(4)]) == 4
	assert d.col_of(d.buf.line_start(2) + d.buf.line(2).len) == 9
}

fn test_soft_wrap_counts_two_columns_per_wide_character() {
	for s, want in {
		'漢字漢字':  [0, 6]
		'ab 漢字漢': [0, 3, 9]
		'😀😀😀':    [0, 8]
	} {
		mut out := []int{}
		row_breaks(s, if s.starts_with('ab') { 5 } else { 4 }, mut out)
		assert out == want, s
	}
	// Up and down by rows keep the display column.
	_, mut e := grid_editor('漢字漢字漢字')
	e.wrap = true
	e.bounds = prim.rect(100, 0, 9 * e.cell.width, 400) // 8 columns
	e.sync_rows()
	assert e.rows.starts == [0, 12]
	e.doc.set_cursor(6, 6, false)
	e.move_rows(1, false)
	assert e.doc.primary().head == 18
	e.move_rows(-1, false)
	assert e.doc.primary().head == 6
}
