// Protocol tests for what the engine gained over the plain grid (specs/17).
module term

import quadra.kit
import quadra.platform
import quadra.prim

fn screen_with(cols int, rows int, input string) &Screen {
	mut s := new_screen(cols, rows)
	mut p := Parser{}
	p.feed(mut s, input.bytes())
	return s
}

fn replies(input string) string {
	return screen_with(20, 8, input).replies.bytestr()
}

fn test_osc_133_zones_survive_the_scrollback_and_reflow() {
	mut t := &Terminal{
		screen: new_screen(20, 3)
	}
	t.feed('\x1b]133;A\x07\$ \x1b]133;B\x07cmd'.bytes())
	assert t.in_command_input()
	assert t.input_start(0) == 2
	assert t.zone_at(Pos{0, 0}) == .prompt
	assert t.row_zone(0) == .input
	t.feed('\r\n\x1b]133;C\x07out\r\n\x1b]133;D;3\x07'.bytes())
	assert !t.in_command_input()
	assert t.screen.last_exit? == 3
	assert t.screen.prompt_count == 1
	for _ in 0 .. 4 {
		t.feed('\r\nmore'.bytes())
	}
	assert t.screen.scrollback.len() > 2
	m := t.shell_marks_at(0)
	assert m.marks & mark_a != 0 && m.marks & mark_b != 0 && t.zone_at(Pos{0, 0}) == .prompt
	assert t.shell_marks_at(1).marks & mark_c != 0 && t.zone_at(Pos{1, 0}) == .output
	assert t.shell_marks_at(2).exit? == 3
	t.screen.resize(10, 3)
	assert t.shell_marks_at(0).marks & mark_a != 0 && t.input_start(0) == 2
	assert t.screen.events.len == 4 && t.screen.events[3].mark == mark_d
}

fn test_osc_9_progress_and_notifications() {
	mut s := screen_with(20, 3, '\x1b]9;4;1;150\x07\x1b]9;hello;world\x07')
	assert s.progress.state == .set && s.progress.percent == 100
	assert s.events[0].kind == .progress && s.events[1].kind == .notification
	assert s.events[1].text == 'hello;world'
	assert sanitize_notification('a\x07b\x1b[31m') == 'ab[31m'
}

fn test_osc_7501_program_status() {
	s := screen_with(20, 3, '\x1b]7501;state=working:id=a\x07\x1b]7501;state=error:id=b\x07')
	assert s.program_status? == .error
	q := screen_with(20, 3, '\x1b]7501;?\x1b\\')
	assert q.replies.bytestr() == '\x1b]7501;?\x1b\\'
	bad := screen_with(20, 3, '\x1b]7501;state=working:id=a/\x07')
	assert bad.program_status == none
	mut p := screen_with(20, 3, '\x1b]7501;state=working:id=x\x07\x1b]7501;state=done:id=y\x07')
	assert p.program_status? == .working
	mut pr := Parser{}
	pr.feed(mut p, '\x1b]133;A\x07'.bytes())
	assert p.program_status? == .done
}

fn test_dynamic_colors() {
	pal := kit.TerminalColors{
		foreground: prim.rgb(0x112233)
		background: prim.rgb(0x445566)
		cursor:     prim.rgb(0x778899)
	}
	mut s := screen_with(20, 3, '\x1b]4;1;rgb:ff/00/80\x07\x1b]10;#abc\x07\x1b]11;?\x1b\\')
	assert s.over[1] == rgb_color(0xff, 0, 0x80)
	assert s.over[key_fg] == rgb_color(0xa0, 0xb0, 0xc0)
	s.answer_queries(pal)
	assert s.replies.bytestr() == '\x1b]11;rgb:4444/5555/6666\x1b\\'
	assert s.paint_palette(pal).foreground == prim.rgb(0xa0b0c0)
	mut p := Parser{}
	p.feed(mut s, '\x1b]110\x07\x1b]104;1\x07'.bytes())
	assert s.over[key_fg] == 0 && s.over[1] == 0
}

fn test_underline_color_links_and_sgr_extras() {
	s := screen_with(20, 3,
		'\x1b[4:3;58;5;9mA\x1b[59;53mB\x1b]8;id=x;http://a\x07L\x1b[0mM\x1b]8;;\x07Z')
	c0, c1 := s.lines[0].cells[0], s.lines[0].cells[1]
	assert c0.flags & f_curly != 0 && s.extra_of(c0.pad).ul == palette(9)
	assert s.extra_of(c1.pad).ul == 0 && s.extra_of(c1.pad).over
	l := s.lines[0].cells[2]
	assert s.links[s.extra_of(l.pad).link - 1] == LinkTarget{'x', 'http://a'}
	// SGR 0 drops the attributes, not the link; OSC 8 with no url ends it.
	m := s.lines[0].cells[3]
	assert s.extra_of(m.pad).link == s.extra_of(l.pad).link && s.extra_of(m.pad).ul == 0
	assert s.extra_of(s.lines[0].cells[4].pad).link == 0
	assert replies('\x1bP\$qm\x1b\\').starts_with('\x1bP1\$r0m')
}

fn test_sixel_places_scrolls_and_dies() {
	// One 6 px band of 2 columns, with a raster size, then text.
	mut s := screen_with(20, 4, 'ab\x1bPq"1;1;20;40#0;2;100;0;0#0!20~\x1b\\X')
	assert s.graphics.len == 1 && s.graphics[0].width == 20 && s.graphics[0].height == 40
	px := s.graphics[0].pixels.bytes()
	assert px[0] == 255 && px[3] == 255
	assert s.lines[0].extra.images[0].row == 0 && s.lines[1].extra.images[0].row == 1
	assert s.lines[0].extra.images[0].cols == 2 && s.x == 3 // the X follows at the same row
	// The image scrolls with the text into the scrollback.
	mut p := Parser{}
	p.feed(mut s, '\r\n\r\n\r\n\r\n\r\n\r\n'.bytes())
	assert s.scrollback.at(0).extra.images.len == 1
	// Erasing a row with a reference drops it, and an unreferenced graphic is swept.
	p.feed(mut s, '\x1b[2J\x1b[3J'.bytes())
	s.sweep_graphics()
	assert s.graphics.len == 0
	s2 := screen_with(20, 4, '\x1bPq#0;1;0;50;100#0!4~-!4~\x1b\\')
	assert s2.graphics[0].width == 4 && s2.graphics[0].height == 12 && s2.y == 0
	aborted := screen_with(20, 4, '\x1bPq"1;1;9;9qqq\x18ab')
	assert aborted.graphics.len == 0 && aborted.line(0) == 'ab'
	assert replies('\x1b[c') == '\x1b[?62;4;22c'
}

fn test_modes_and_queries() {
	assert replies('\x1b[?1006h\x1b[?1006\$p') == '\x1b[?1006;1\$y'
	assert replies('\x1b[?3\$p') == '\x1b[?3;0\$y' && replies('\x1b[?9001\$p') == '\x1b[?9001;2\$y'
	assert replies('\x1b[4\$p') == '\x1b[4;2\$y'
	assert replies('\x1b[?2026h\x1b[?2026l\x1b[?2026\$p') == '\x1b[?2026;2\$y'
	assert replies('\x1b[>q').starts_with('\x1bP>|Quadra')
	assert replies('\x1bP+q544e\x1b\\') == '\x1bP1+r544e=517561647261\x1b\\'
	assert replies('\x1b[3 q\x1bP\$q q\x1b\\') == '\x1bP1\$r3 q\x1b\\'
	assert replies('\x1b[?6n') == '\x1b[?1;1;1R'
	mut b := screen_with(20, 8, 'x')
	b.set_dec_mode(45, true)
	assert b.reverse_wrap
	s := screen_with(4, 2, 'abcde\x1b[?45h\r\x08')
	assert s.y == 0 && s.x == 3
	// DEC special graphics through G1 and SO, with a single shift.
	d := screen_with(10, 2, '\x1b)0\x0eq\x0fq\x1bNx')
	assert d.line(0) == '─qx'
}

fn test_sync_update_holds_the_screen_until_it_ends() {
	mut t := &Terminal{
		screen: new_screen(10, 2)
	}
	t.feed('\x1b[?2026habc'.bytes())
	assert t.screen.line(0) == '' && t.parser.held_on
	t.feed('def\x1b[?2026l'.bytes())
	assert t.screen.line(0) == 'abcdef' && !t.parser.held_on
	t.feed('\x1b[?2026hz'.bytes())
	t.flush_sync()
	assert t.screen.line(0).ends_with('z') && !t.screen.sync
}

fn kitty_enc(k platform.Key, m platform.Mods, a KeyAction, input KeyInput) string {
	return (encode_key_event(k, m, a, input) or { return '<none>' }).bytestr()
}

fn test_kitty_keyboard() {
	none_ := platform.Mods.zero()
	mut s := screen_with(20, 3, '\x1b[>1u')
	assert s.keyboard.active.live == 1
	assert replies('\x1b[>3u\x1b[?u') == '\x1b[?3u'
	input := KeyInput{false, kitty_all_as_esc | kitty_event_types | kitty_alternates, 0}
	assert kitty_enc(.a, none_, .press, input) == '\x1b[97u'
	assert kitty_enc(.a, .shift, .press, input) == '\x1b[97:65;2u'
	assert kitty_enc(.enter, .ctrl, .repeat, input) == '\x1b[13;5:2u'
	assert kitty_enc(.up, none_, .release, input) == '\x1b[1;1:3A'
	dis := KeyInput{false, kitty_disambiguate, 0}
	assert (encode_key_event(.f5, none_, .press, dis) or { []u8{} }).bytestr() == '\x1b[15~'
	assert encode_key_event(.a, none_, .press, dis) == none // text stays text
	legacy := KeyInput{false, 0, 2}
	assert (encode_key_event(.a, platform.Mods.ctrl, .press, legacy) or { []u8{} }).bytestr() == '\x1b[27;5;97~'
	mut screen := screen_with(20, 3, '\x1b[?1049h\x1b[>1u')
	assert screen.keyboard.active.live == 1
	screen.set_alt(false)
	assert screen.keyboard.active.live == 0
}

fn test_grapheme_clusters_with_mode_2027() {
	s := screen_with(10, 2,
		'\x1b[?2027h\xf0\x9f\x91\xa9\xe2\x80\x8d\xf0\x9f\x92\xbbX\xe2\x9d\xa4\xef\xb8\x8f')
	// woman + ZWJ + laptop is one wide cell; heart + VS16 widens.
	assert s.lines[0].cells[0].flags & f_wide != 0 && s.lines[0].marks[0].s.len > 0
	assert s.lines[0].cells[2].ch == u32(`X`)
	assert s.lines[0].cells[3].flags & f_wide != 0 && s.x == 5
	plain := screen_with(10, 2, '\xf0\x9f\x91\xa9\xe2\x80\x8d\xf0\x9f\x92\xbb')
	assert plain.x == 4 // per scalar without the mode
}

fn test_c1_prints_nothing_and_widths_cover_the_table() {
	s := screen_with(10, 2, 'a\xc2\x85b')
	assert s.line(0) == 'ab'
	assert width(0x0301) == 0 && width(0x1F600) == 2 && width(u32(`A`)) == 1 && width(0x1160) == 0
}

fn reds(b &PixelBuf) []u8 {
	bs := b.bytes()
	return []u8{len: bs.len / 4, init: bs[index * 4]}
}

// PixelBuf lays rows out in place (specs/17, Images memory): growth is transparent, cuts drop.
fn test_pixelbuf_restride_and_scale_keep_pixels() {
	mut b := PixelBuf{}
	b.restride(2, 2)
	for i in 0 .. 4 {
		b.fill(i, 1, u32(i + 1))
	}
	b.fill(3, 2, 9) // past the end: ignored
	b.restride(3, 3)
	assert reds(b) == [u8(1), 2, 0, 3, 4, 0, 0, 0, 0]
	b.restride(1, 2)
	assert reds(b) == [u8(1), 3]
	mut s := b.scaled(2, 4)
	assert reds(s) == [u8(1), 1, 1, 1, 3, 3, 3, 3]
	mut moved := s.take()
	assert s.len == 0 && moved.len == 32
	moved.free()
	b.free()
	b.free() // a no-op
	assert b.bytes().len == 0
}
