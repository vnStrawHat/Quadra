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
