// Fuzz test of the escape-sequence parser and the screen it drives (specs/11, fuzz tests):
// random bytes and mutated real sequences, with resizes in the middle of streams; the grid
// invariants are checked after every input.
module term

import os
import quadra.native.sys
import quadra.testing.fuzz

const fuzz_corpus = [
	'hello\r\nworld\x1b[2;3HX\x1b[K\x1b[2J',
	'\x1b[1;31;42;4:3;58;5;9;38;2;1;2;3;48;5;200mA\x1b[0m',
	'\x1b[65535;65535H\x1b[99999A\x1b[0;0r\x1b[5;2r\x1b[999L\x1b[999M\x1b[999@\x1b[999P\x1b[999X',
	'\x1b[1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16;17;18;19;20m',
	'\x1b]8;id=x;https://example.com/a?b=c\x07link\x1b]8;;\x07',
	'\x1b]52;c;aGVsbG8=\x07\x1b]52;c;?\x07\x1b]0;title\x07\x1b]2;t\x1b\\\x1b]7;file://h/p%20q\x07',
	'\x1b]4;1;rgb:ff/00/80\x07\x1b]10;#abc\x07\x1b]11;?\x1b\\\x1b]104;1\x07\x1b]110\x07',
	'\x1b]133;A\x07\$ \x1b]133;B\x07cmd\r\n\x1b]133;C\x07out\x1b]133;D;3\x07',
	'\x1b]9;4;1;150\x07\x1b]9;hello\x07\x1b]7501;state=working:id=a\x07\x1b]7501;?\x1b\\',
	'\x1bPq"1;1;20;40#0;2;100;0;0#1;1;120;50;100#0!20~-#1!9999~\$!4~\x1b\\',
	'\x1bPq"1;1;1024;1024#255;2;100;100;100!1024~-!1024~\x1b\\',
	'\x1bP\$qm\x1b\\\x1bP+q544e\x1b\\\x1bP\$q q\x1b\\\x1bPzzz\x1b\\',
	'\x1b[>q\x1b[c\x1b[>c\x1b[5n\x1b[6n\x1b[?6n\x1b[?2026\$p\x1b[18t\x1b[22;0t\x1b[23;0t',
	'\x1b[?1049hALT\x1b[?1049l\x1b[?47h\x1b[?1047h\x1b[?1048h\x1b[?1048l\x1b[?1047l',
	'\x1b[?2026hheld\x1b[?2026l\x1b[?2027h👨‍👩\x1b[?2027l',
	'\x1b[?1000h\x1b[?1006h\x1b[?1015h\x1b[?1005h\x1b[?9h\x1b[?1003h\x1b[?2004h\x1b[?25l',
	'\x1b[>1u\x1b[>31u\x1b[=5;2u\x1b[<9u\x1b[?u\x1b[>4;2m\x1b[3 q\x1b[?12h',
	'\x1b(0qxlk\x1b(B\x1b)0\x0e\x0f\x1bN\x1bO\x1b*0\x1bn',
	'\x1b7\x1b[10;10H\x1b8\x1bD\x1bM\x1bE\x1bH\x1b[3g\x1b[W\x1b[?5W\x1b[2Z\x1b[2I',
	'漢字漢字漢字漢字\x1b[1;10H漢\x1b[?7l漢字漢字漢字漢字漢字\x1b[?7h',
	'é́́🇻🇳🏳️‍🌈\x1b[5b',
	'\x1b[?45h\x08\x08\x08\x1b[?6h\x1b[3;4r\x1b[1;1Hx\x1b[?6l\x1b[4h\x1b[20h',
	'\x1b[${'1;'.repeat(40)}1m\x1b[!!!p\x1b !#8\x1b((0',
	'\x1b]2;${'t'.repeat(3000)}\x07\x1b]7;${'p'.repeat(3000)}\x07\x1b]9;4;1;${'0'.repeat(3000)}\x07',
	'\x1b]52;c;${';'.repeat(3000)}\x07\x1b]4${';1;red'.repeat(30)}\x07',
	'\x1b[?2026h\x1b[?2026ha\x1b[?2026l\x1b[?2026h',
	'\x1b#8\x1b[4l\x1b[\$x\x1b[1;1;2;2\$z\x1b[*x\x1b[1;1;3;3;1*y\x1b[1;2;3;4\$r',
]

// A Sixel image may be 4096 px each way (64 MiB of RGBA) and a screen keeps 64 MiB of images.
// The corpus image is 1024 px (mutations still reach the 4096 cap now and then): blocks of
// 64 MiB fragment the collector's heap (each needs a fresh run of pages no stale word points
// into), and a run of them grew the heap to 450 MiB with one image alive. The ceiling stays
// under the 512 MiB hard cap of quadra.testing.fuzz.
const term_ceiling_mb = 384
const term_grow_mb = 128

fn check_screen(s &Screen, label string) {
	assert s.cols >= 1 && s.rows >= 1, label
	assert s.lines.len == s.rows, '${label}: ${s.lines.len} rows, want ${s.rows}'
	for i, r in s.lines {
		assert r.cells.len == s.cols, '${label}: row ${i} has ${r.cells.len} cells, want ${s.cols}'
	}
	if s.alt_active {
		assert s.main.len == s.rows, '${label}: main grid ${s.main.len} rows'
		for r in s.main {
			assert r.cells.len == s.cols, '${label}: main row has ${r.cells.len} cells'
		}
	}
	assert s.x >= 0 && s.x < s.cols, '${label}: cursor x ${s.x} of ${s.cols}'
	assert s.y >= 0 && s.y < s.rows, '${label}: cursor y ${s.y} of ${s.rows}'
	assert s.top >= 0 && s.top <= s.bottom && s.bottom < s.rows, '${label}: region ${s.top}..${s.bottom}'
	_ = s.text()
	_ = s.line(s.y)
}

fn fuzz_input(mut r fuzz.Rng, corpus [][]u8) []u8 {
	match r.intn(5) {
		0 {
			return r.bytes(r.intn(512))
		}
		1 { // random sequences glued from corpus pieces and control bytes
			mut b := []u8{}
			for _ in 0 .. 1 + r.intn(8) {
				c := corpus[r.intn(corpus.len)]
				at := r.intn(c.len)
				b << c[at..at + r.intn(c.len - at + 1)]
				if r.chance(30) {
					b << [u8(0x1b), 0x07, 0x18, 0x9b, 0x90, 0x9c, `[`, `]`, `P`][r.intn(9)]
				}
			}
			return b
		}
		else {
			return r.mutate(corpus[r.intn(corpus.len)], corpus)
		}
	}
}

// feed_split feeds data in random pieces, with a resize between some of them.
fn feed_split(mut r fuzz.Rng, mut p Parser, mut s Screen, data []u8) {
	mut i := 0
	for i < data.len {
		n := 1 + r.intn(data.len - i)
		p.feed(mut s, data[i..i + n])
		i += n
		if r.chance(15) {
			s.resize(1 + r.intn(120), 1 + r.intn(50))
		}
		if r.chance(5) && p.held_on { // the view's timeout ending an open mode 2026
			s.sync, p.held_on = false, false
			held := p.held
			p.held = []u8{}
			p.feed(mut s, held)
		}
	}
}

fn test_fuzz_parser_fresh_screens() {
	corpus := fuzz_corpus.map(it.bytes())
	mut r := fuzz.new_rng(0x7e2a1)
	n := fuzz.iters(3000)
	mut w := fuzz.watch('term fresh', n)
	w.ceiling_mb = term_ceiling_mb
	for i in 0 .. n {
		data := fuzz_input(mut r, corpus)
		w.start()
		mut s := new_screen(1 + r.intn(100), 1 + r.intn(40))
		s.allow_osc52, s.allow_osc52_read, s.allow_screen_readback = true, true, true
		mut p := Parser{}
		feed_split(mut r, mut p, mut s, data)
		w.done(i, data.len, 2000)
		check_screen(s, 'input ${i}')
	}
	w.finish(term_grow_mb)
}

// One screen and parser across every input, like a long session: state left by one input
// (an open string, a held update, modes, the alternate screen) meets the next.
fn test_fuzz_parser_long_session() {
	corpus := fuzz_corpus.map(it.bytes())
	mut r := fuzz.new_rng(0x5e55)
	n := fuzz.iters(2000)
	mut w := fuzz.watch('term session', n)
	w.ceiling_mb = term_ceiling_mb
	mut s := new_screen(80, 24)
	s.allow_osc52 = true
	mut p := Parser{}
	for i in 0 .. n {
		data := fuzz_input(mut r, corpus)
		w.start()
		feed_split(mut r, mut p, mut s, data)
		w.done(i, data.len, 2000)
		check_screen(s, 'session input ${i}')
		s.replies.clear()
		s.events.clear()
		assert p.held.len <= max_sync_hold, 'held ${p.held.len} bytes'
	}
	w.finish(term_grow_mb)
}

// Wide characters at the right edge, with autowrap on and off, insert mode and resizes.
fn test_fuzz_wide_chars_at_edges() {
	mut r := fuzz.new_rng(0x3d)
	pieces := ['漢', '字', '😀', 'a', 'é', '́', '‍', '\x1b[?7l', '\x1b[?7h', '\x1b[4h',
		'\x1b[4l', '\x1b[1;${r.intn(9) + 1}H', '\x1b[@', '\x1b[P', '\x1b[2X', '\r\n', '\x1b[?2027h',
		'\x1b[?2027l', '\x08', '\t', '\x1b[1b', '\x1b[?45h']
	for i in 0 .. fuzz.iters(1000) {
		mut s := new_screen(1 + r.intn(6), 1 + r.intn(4))
		mut p := Parser{}
		mut log := []string{}
		for _ in 0 .. 40 {
			piece := r.pick(pieces)
			log << piece.replace('\x1b', 'ESC').replace('\r\n', 'CRLF').replace('\x08', 'BS')
			p.feed(mut s, piece.bytes())
			if r.chance(10) {
				c, h := 1 + r.intn(6), 1 + r.intn(4)
				log << 'resize ${c}x${h}'
				s.resize(c, h)
			}
			check_screen(s, 'edges ${i}: ${log}')
		}
	}
}

// Regression: an update opened with mode 2026 and never closed held every byte (and scanned
// the whole buffer again on each feed); past max_sync_hold the parser gives up and parses.
fn test_unclosed_sync_update_is_bounded() {
	mut s := new_screen(20, 5)
	mut p := Parser{}
	p.feed(mut s, '\x1b[?2026h'.bytes())
	chunk := []u8{len: 64 * 1024, init: `x`}
	for _ in 0 .. (max_sync_hold / chunk.len) * 3 {
		p.feed(mut s, chunk)
		assert p.held.len <= max_sync_hold
	}
	assert !p.held_on || p.held.len < max_sync_hold
	// The close split across two feeds still ends the hold.
	mut s2 := new_screen(20, 5)
	mut p2 := Parser{}
	p2.feed(mut s2, '\x1b[?2026hab\x1b[?20'.bytes())
	assert p2.held_on
	p2.feed(mut s2, '26lc'.bytes())
	assert !p2.held_on && s2.text() == 'abc'
}

// Regression: resizing to one column with a wide character on screen looped for good in
// LogicalLine.split (the character waited for a second column), appending rows until the
// collector gave up: gigabytes, then "Too many heap sections".
fn test_one_column_reflow_with_wide_chars() {
	mut s := new_screen(10, 3)
	mut p := Parser{}
	p.feed(mut s, 'a漢字b\r\n漢'.bytes())
	s.resize(1, 5)
	check_screen(s, 'one column')
	assert s.text().contains('漢')
	s.resize(4, 3)
	check_screen(s, 'back to four')
}

// Regression: OSC 52 with bytes above `z` indexed past base64's table (toolchain patch
// tools/patches/v-base64-bounds.diff); it now decodes to nothing. The screen field only: the
// real clipboard is never touched.
fn test_osc52_with_bytes_outside_base64() {
	mut s := new_screen(10, 3)
	s.allow_osc52 = true
	mut p := Parser{}
	p.feed(mut s, [u8(0x1b), `]`, `5`, `2`, `;`, `c`, `;`, `a`, `G`, 0xff, 0xfe, `b`, `G`, `8`,
		`=`, 0x07])
	assert (s.clipboard or { '' }) == ''
	check_screen(s, 'osc 52')
}

// Regression: with a raster size, pixels painted past it were stored though flush cuts them
// off: a 20x40 image painting 4096 px bands filled a 64 MiB decode buffer.
fn test_sixel_paints_only_inside_its_raster() {
	mut x := new_sixel()
	mut src := '"1;1;20;40#0;2;100;0;0#0'
	for _ in 0 .. 700 {
		src += '!9999~-'
	}
	for b in src.bytes() {
		x.put(b)
	}
	assert x.pixels.len <= 64 * 40 * 4, '${x.pixels.len} bytes buffered'
	x.flush()
	assert x.img_w == 20 && x.img_h == 40 && x.rgba.len == 20 * 40 * 4
	assert x.rgba.bytes()[0] == 255 && x.rgba.bytes()[3] == 255
	x.free()
}

// Image pixels live in the C heap (specs/17, images memory): 50 images of 2048 px each way in
// a row leave the collector's heap flat, and dropping the screen's images gives the memory back.
fn test_large_sixels_stay_off_the_collector_heap() {
	mut img := '\x1bPq"1;1;2048;2048#0;2;100;0;0#0'
	for _ in 0 .. 342 {
		img += '!2048~-'
	}
	data := (img + '\x1b\\').bytes()
	mut s := new_screen(10, 5)
	mut p := Parser{}
	p.feed(mut s, data) // warm-up: the collector's first growth does not count
	gc_collect()
	heap0, mem0 := gc_heap_usage().heap_size, private_bytes()
	for _ in 0 .. 50 {
		p.feed(mut s, data)
	}
	gc_collect()
	heap1, mem1 := gc_heap_usage().heap_size, private_bytes()
	s.free_graphics()
	gc_collect()
	mem2 := private_bytes()
	eprintln('50 sixels of 2048 px: collector heap +${(i64(heap1) - i64(heap0)) >> 20} MiB, private bytes ${mem0 >> 20} -> ${mem1 >> 20} -> ${mem2 >> 20} MiB after free_graphics')
	assert i64(heap1) - i64(heap0) < 32 << 20
	if mem0 > 0 {
		assert i64(mem2) - i64(mem0) < 8 << 20, 'C heap kept ${(i64(mem2) - i64(mem0)) >> 20} MiB'
	}
}

fn private_bytes() u64 {
	c := sys.memory_of(u32(os.getpid())) or { return 0 }
	return c.commit
}
