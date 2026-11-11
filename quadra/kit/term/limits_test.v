// Every limit in specs/17 (Limits), at the limit and just past it; the bytes also go one at a
// time, so the byte loop and the fast paths agree.
module term

import quadra.core
import quadra.platform
import quadra.testing

fn screen_with(cols int, rows int, input string) &Screen {
	mut s := new_screen(cols, rows)
	mut p := Parser{}
	p.feed(mut s, input.bytes())
	return s
}

fn kitty_enc(k platform.Key, m platform.Mods, a KeyAction, input KeyInput) string {
	return (encode_key_event(k, m, a, input) or { return '<none>' }).bytestr()
}

fn screens(input string) []&Screen {
	whole := screen_with(40, 5, input)
	mut bytewise := new_screen(40, 5)
	mut p := Parser{}
	for b in input.bytes() {
		p.feed(mut bytewise, [b])
	}
	return [whole, bytewise]
}

fn numbers(n int, sep string) string {
	return []string{len: n, init: '1'}.join(sep)
}

fn test_csi_takes_32_parameters_and_ignores_33() {
	for s in screens('\x1b[${numbers(32, ';')}m') {
		assert s.pen.flags != 0
	}
	for s in screens('\x1b[${numbers(33, ';')}m') {
		assert s.pen.flags == 0
	}
	for s in screens('\x1b[${':'.repeat(32)}1m') { // sub-parameters use the same budget
		assert s.pen.flags == 0
	}
}

fn test_a_third_intermediate_ignores_the_sequence() {
	for s in screens('\x1b[5;5H\x1b[!p') {
		assert s.x == 0 && s.y == 0 // one intermediate: the soft reset, cursor home
	}
	for seq in ['\x1b[!!p', '\x1b[!!!p'] {
		for s in screens('\x1b[5;5H' + seq) {
			assert s.x == 4 && s.y == 4
		}
	}
	for s in screens('\x1b !#8') {
		assert s.text() == ''
	}
	for s in screens('\x1b#8') {
		assert s.text().starts_with('EEE')
	}
}

fn test_osc_is_cut_at_2048_and_still_dispatches() {
	for s in screens('\x1b]2;${'x'.repeat(3000)}\x07') {
		assert s.title == 'x'.repeat(2047) // the number counts, its `;` does not
	}
	for s in screens('\x1b]2;${'x'.repeat(2047)}\x07') {
		assert s.title.len == 2047
	}
	// The first 15 separators are free, later ones are payload.
	for s in screens('\x1b]2${';'.repeat(20)}${'x'.repeat(3000)}\x07') {
		assert s.title == ';'.repeat(19) + 'x'.repeat(2042)
	}
}

fn test_a_cut_path_or_progress_is_refused() {
	assert screen_with(20, 3, '\x1b]7;${'p'.repeat(2047)}\x07').cwd.len == 2047
	cut_path := '\x1b]7;/a\x07\x1b]7;${'p'.repeat(2048)}\x07'
	assert screen_with(20, 3, cut_path).cwd == '/a'
	// The number and the fields, `9` `4` `1` `50`, are 5 of the 2048 bytes.
	whole := screen_with(20, 3, '\x1b]9;4;1;50;${'0'.repeat(2043)}\x07')
	assert whole.progress.percent == 50 && whole.progress.state == .set
	cut := screen_with(20, 3, '\x1b]9;4;1;50;${'0'.repeat(2044)}\x07')
	assert cut.progress.percent == 0 && cut.progress.state == .remove
	// A cut notification is still shown.
	note := screen_with(20, 3, '\x1b]9;${'n'.repeat(3000)}\x07')
	assert note.events.len == 1 && note.events[0].text.len == 2047
}

fn test_only_52_and_7501_may_go_large_and_the_buffer_shrinks() {
	mut s := new_screen(20, 3)
	mut p := Parser{}
	p.feed(mut s, '\x1b]52;c;'.bytes())
	chunk := []u8{len: 1 << 20, init: `A`}
	for _ in 0 .. 8 {
		p.feed(mut s, chunk)
	}
	assert p.osc.len == osc_large + 2 && p.osc_cut // the `;` of `52;c;` are outside the cap
	p.feed(mut s, '\x07\x1b]2;t\x07'.bytes())
	assert p.osc.cap <= osc_inline && s.title == 't'
	p.feed(mut s, '\x1b]5200;'.bytes())
	p.feed(mut s, chunk)
	assert p.osc.len == osc_inline + 1
}

fn test_osc_params_past_sixteen_keep_a_bulk_palette() {
	mut pairs := ''
	for i in 0 .. 20 {
		pairs += ';${i};rgb:ff/00/00'
	}
	s := screen_with(20, 3, '\x1b]4${pairs}\x07')
	assert s.over[0] != 0 && s.over[19] != 0
	// A large payload's split stays bounded: all that matters is that it returns.
	big := screen_with(20, 3, '\x1b]52;c;${';'.repeat(100000)}\x07')
	assert big.clipboard == none
}

fn test_clean_text() {
	assert clean_text('a\x01\t\n\x7f\u0085‮⁦b', 99, true) == 'a\t\nb'
	assert clean_text('a\t\nb\r', 99, false) == 'ab'
	x := 'x'.repeat(4096)
	assert clean_text(x, 4096, true) == x
	assert clean_text(x + 'y', 4096, true) == x
	assert clean_text('é'.repeat(3), 3, true) == 'é' // never half a character
	assert screen_with(20, 3, '\x1b]2;a‮b\x07').title == 'ab'
	assert screen_with(20, 3, '\x1b]7;/a\tb\x07').cwd == '/ab'
	assert screen_with(20, 3, '\x1b]7;/a\x07\x1b]7;\x01\x07').cwd == '/a' // empty: kept
	assert sanitize_notification('n'.repeat(8193)).len == 8192
}

fn test_notice_gate_is_a_fixed_window() {
	mut g := NoticeGate{}
	for _ in 0 .. 10 {
		assert g.allow(5000)
	}
	assert !g.allow(5000) && !g.allow(5999)
	assert g.allow(6000)
}

fn test_product_name() {
	assert clean_product('Abc\x01\x7f\u0085d', 'def') == 'Abcd'
	assert clean_product('\x01', 'def') == 'def'
	assert clean_product('é'.repeat(40), 'def').len == 64
	assert clean_product('x'.repeat(65), 'def').len == 64
}

fn test_grid_and_scrollback_are_clamped() {
	mut s := new_screen(5000, 3000)
	assert s.cols == max_cols && s.rows == max_rows && s.lines.len == max_rows
	s = new_screen(10, 5)
	s.resize(2048, 1024)
	assert s.cols == 2048 && s.rows == 1024
	s.resize(2049, 1025)
	assert s.cols == 2048 && s.rows == 1024 && s.lines.len == 1024
	s.set_scrollback(1_000_000)
	assert s.scrollback.max == 1_000_000
	s.set_scrollback(1_000_001)
	assert s.scrollback.max == 1_000_000
}

fn test_links_share_entries_up_to_65535() {
	mut s := new_screen(10, 2)
	s.set_link('', 'http://a')
	first := s.extra_of(s.pen.pad).link
	s.set_link('', '')
	s.set_link('', 'http://a')
	assert s.extra_of(s.pen.pad).link == first && s.links.len == 1 // no id: by URI
	s.set_link('id=x', 'http://a')
	assert s.links.len == 2 // an id is its own entry
	s.set_link('id=x', 'http://a')
	assert s.links.len == 2
	s.set_link('', 'http://' + 'u'.repeat(2076))
	assert s.links.len == 3 // 2083 bytes
	s.set_link('', 'http://' + 'u'.repeat(2077))
	assert s.links.len == 3
	for i in s.links.len .. max_links {
		s.set_link('', 'http://${i}')
	}
	assert s.links.len == max_links && s.extra_of(s.pen.pad).link == u16(max_links)
	s.set_link('', 'http://one-more')
	assert s.links.len == max_links && s.extra_of(s.pen.pad).link == 0
	s.reset()
	s.set_link('', 'http://a')
	assert s.extra_of(s.pen.pad).link == 1
}

fn test_a_cluster_keeps_16_code_points() {
	mut s := screen_with(10, 2, 'a' + '́'.repeat(14))
	assert s.lines[0].marks[0].s.runes().len == 14
	s = screen_with(10, 2, 'a' + '́'.repeat(15))
	assert s.lines[0].marks[0].s.runes().len == 15
	s = screen_with(10, 2, 'a' + '́'.repeat(30))
	assert s.lines[0].marks[0].s.runes().len == 15
}

fn sync_term() &Terminal {
	return &Terminal{
		screen: new_screen(20, 2)
	}
}

fn test_sync_hold_lapse_keeps_the_mode() {
	mut t := sync_term()
	t.feed('\x1b[?2026habc'.bytes())
	assert t.parser.held_on && t.screen.sync_hold && t.screen.sync_opens == 1
	t.feed('\x1b[?2026hdef'.bytes()) // an `h` that arrives held back refreshes
	assert t.screen.sync_opens == 2 && t.screen.line(0) == ''
	t.flush_sync()
	assert t.screen.line(0) == 'abcdef' && t.screen.sync && !t.screen.sync_hold
	assert !t.parser.held_on && t.screen.sync_opens == 2 // replayed: no new hold
	t.feed('x'.bytes()) // frames resume
	assert t.screen.line(0) == 'abcdefx'
	t.feed('\x1b[?2026hy'.bytes()) // a new `h` holds again
	assert t.parser.held_on && t.screen.line(0) == 'abcdefx'
	t.feed('\x1b[?2026l'.bytes())
	assert t.screen.line(0) == 'abcdefxy' && !t.screen.sync && !t.screen.sync_hold
}

fn test_sync_hold_past_4_mib_lapses() {
	mut t := sync_term()
	t.feed('\x1b[?2026h'.bytes())
	t.feed([]u8{len: max_sync_hold, init: `a`})
	assert t.parser.held_on
	t.feed('b'.bytes())
	assert !t.parser.held_on && t.screen.sync && !t.screen.sync_hold
}

fn test_gpu_images_stay_within_64_and_64_mib_oldest_out() {
	mut t := sync_term()
	for id in 1 .. 66 {
		t.gfx[u32(id)] = &core.ImageEntry{
			width:  1
			height: 1
		}
		t.evict_gfx()
	}
	assert t.gfx.len == 64 && u32(1) !in t.gfx && u32(2) in t.gfx && t.gfx_dead.len == 1
	t = sync_term()
	for id in 1 .. 6 {
		t.gfx[u32(id)] = &core.ImageEntry{
			width:  2048
			height: 2048
		}
	}
	t.evict_gfx() // 4 fill 64 MiB exactly
	assert t.gfx.len == 4 && u32(1) !in t.gfx
}

fn test_releasing_the_view_ends_the_session_and_frees_the_pixels() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	v := new_terminal(mut cx, id: 't')
	mut t := cx.peek(v.model())
	mut fake := &FakePty{}
	start(mut cx, v, fake.spawner(), platform.PtyOptions{})!
	t.feed('\x1bPq"1;1;4;6#0;2;100;0;0#0!4~\x1b\\'.bytes())
	assert t.screen.graphics.len == 1 && t.screen.graphics[0].pixels.len > 0
	cx.release(v.model())
	assert fake.closed && t.screen.graphics[0].pixels.len == 0 && t.screen.images_off
}

fn test_kitty_associated_text() {
	none_ := platform.Mods.zero()
	input := KeyInput{false, kitty_all_as_esc | kitty_event_types | kitty_text, 0}
	assert kitty_enc(.a, none_, .press, input) == '\x1b[97;;97u'
	assert kitty_enc(.a, .shift, .press, input) == '\x1b[97;2;65u'
	assert kitty_enc(.a, none_, .release, input) == '\x1b[97;1:3u'
	assert kitty_enc(.a, .ctrl, .press, input) == '\x1b[97;5u'
	assert kitty_enc(.enter, none_, .press, input) == '\x1b[13u' // a control code
	no_all := KeyInput{false, kitty_disambiguate | kitty_text, 0}
	assert kitty_enc(.f5, none_, .press, no_all) == '\x1b[15~'
}
