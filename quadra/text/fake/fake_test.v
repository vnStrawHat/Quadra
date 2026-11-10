// TextSystem tests on the deterministic backend (10 px text = 6 px per grapheme).
module fake

import quadra.text

fn sys() &text.TextSystem {
	return text.new_system(new())
}

const st = text.TextStyle{
	size:        10
	line_height: 1.5
}

fn test_line_layout_and_caret() {
	mut ts := sys()
	l := ts.layout_line('Việt Nam', st)
	assert l.width == 48 // 8 graphemes
	assert l.ascent == 8 && l.descent == 2
	assert l.x_for_index(0) == 0
	assert l.x_for_index(5) == 18 // after "Việ"
	assert l.index_for_x(17) == 5
	assert l.index_for_x(14) == 2 // nearer to the "ệ" start
	assert l.index_for_x(1000) == 'Việt Nam'.len
}

fn test_wrap_greedy_and_hard_breaks() {
	mut ts := sys()
	w := ts.wrap('aaa bbb ccc\ndd', st, 45) // 45 px fits 7 graphemes
	assert w.lines.len == 3
	assert w.lines[0].layout.text == 'aaa bbb '
	assert w.lines[1].layout.text == 'ccc'
	assert w.lines[2].layout.text == 'dd'
	assert w.width == 42 // trailing space does not count
	assert w.height == 45 && w.lines[2].y == 30
}

fn test_wrap_breaks_long_words() {
	mut ts := sys()
	w := ts.wrap('abcdefghij', st, 25) // 4 graphemes per line
	assert w.lines.map(it.layout.text) == ['abcd', 'efgh', 'ij']
}

fn test_word_cache_reuses_shaping() {
	mut ts := sys()
	ts.layout_line('hello world', st)
	shaped := ts.stats.words_shaped
	ts.end_frame()
	ts.layout_line('hello there world', st)
	// "hello", " " and "world" come from the cache; only "there" is new.
	assert ts.stats.words_shaped == shaped + 1
	ts.end_frame()
	ts.layout_line('hello', st) // still cached: entries are swept, not dropped every frame
	assert ts.stats.words_shaped == shaped + 1
	for _ in 0 .. text.cache_sweep_frames {
		ts.end_frame() // unused through a sweep: evicted
	}
	ts.layout_line('hello', st)
	assert ts.stats.words_shaped == shaped + 2
}

fn test_glyph_tiles_upload_once() {
	mut ts := sys()
	a := ts.glyph_tile(0, u16(`a`), 20, 0)
	b := ts.glyph_tile(0, u16(`a`), 20, 0)
	assert a.tile == b.tile && !a.empty
	assert ts.take_uploads().len == 1
	assert ts.glyph_tile(0, u16(` `), 20, 0).empty
}

fn test_wrap_number_matches_wrap() {
	mut ts := sys()
	mut reuse := &text.WrappedText(unsafe { nil })
	for s in ['0', '12.50', '-3,141.59', '100%', '12:30', '1/2'] {
		for w in [f32(1.0e9), 200, 30.01] {
			got := ts.wrap_number(s, st, w, reuse) or {
				assert w < 100 // only too narrow a box refuses
				continue
			}
			if reuse != unsafe { nil } {
				assert got == reuse // refilled in place
			}
			reuse = unsafe { got }
			want := ts.wrap(s, st, w)
			assert got.lines.len == 1 && want.lines.len == 1
			assert got.width == want.width && got.height == want.height
			assert got.line_height == want.line_height && got.width_breaks == want.width_breaks
			a, b := got.lines[0], want.lines[0]
			assert a.start == b.start && a.end == b.end && a.y == b.y
			assert a.layout.glyphs == b.layout.glyphs && a.layout.stops == b.layout.stops
			assert a.layout.width == b.layout.width && a.layout.text == b.layout.text
		}
	}
	assert ts.wrap_number('12 34', st, 1.0e9, unsafe { nil }) == none
	assert ts.wrap_number('abc', st, 1.0e9, unsafe { nil }) == none
	assert ts.wrap_number('123456', st, 12, unsafe { nil }) == none // breaks between digits
}
