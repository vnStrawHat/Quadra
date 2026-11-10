// Completion, semantic classes and terminal events on a FakePty (specs/22, P3).
module main

import os
import highlight
import quadra.core
import quadra.kit.term
import quadra.platform
import quadra.testing

fn term_rig(name string) !(&testing.Harness, &OneTerm, &term.FakePty) {
	dir := '.tmp/wire_term_${name}'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return h, cx.peek(v.model()), fake
}

fn wire_terminal(o &OneTerm, h &testing.Harness) (&Space, &term.Terminal) {
	cx := core.Cx{
		app: unsafe { h.app }
	}
	s := o.current().active_space() or { panic('no space') }
	return s, cx.peek(s.view.model())
}

fn test_the_overlay_appears_tab_accepts_enter_records() {
	mut h, o, mut fake := term_rig('complete')!
	fake.output('\x1b]133;A\x07C:\\work>\x1b]133;B\x07')
	h.frame()
	h.type_text('gi')
	h.frame()
	assert h.find('completion') != none
	before := fake.written.len
	h.press('tab')! // selects, writes nothing
	h.frame()
	assert fake.written.len == before
	h.press('tab')! // accepts the remainder
	h.frame()
	assert fake.written[before..].bytestr() == 't'
	assert h.find('completion') == none
	h.press('enter')! // nothing selected: runs, and is recorded
	h.frame()
	assert !o.history.is_empty()
}

fn test_an_alternate_screen_gates_the_overlay() {
	mut h, _, mut fake := term_rig('alt')!
	fake.output('$ ')
	h.frame()
	fake.output('\x1b[?1049h')
	h.frame()
	h.type_text('gi')
	h.frame()
	assert h.find('completion') == none
}

fn test_escape_dismisses_and_trigger_forces() {
	mut h, _, mut fake := term_rig('esc')!
	fake.output('C:\\work> ')
	h.frame()
	h.type_text('gi')
	h.frame()
	assert h.find('completion') != none
	h.press('escape')!
	h.frame()
	assert h.find('completion') == none
	h.press('ctrl-shift-space')!
	h.frame()
	assert h.find('completion') != none
}

fn test_output_gets_semantic_classes() {
	mut h, o, mut fake := term_rig('classes')!
	fake.output('error: connection refused 10.0.0.1\r\n')
	h.frame()
	_, t := wire_terminal(o, h)
	assert t.classes.len == t.screen.rows
	assert t.classes[0][0] == u8(highlight.Class.error)
	assert t.classes[0][26] == u8(highlight.Class.ip)
	mut oo := unsafe { o }
	oo.cfg.layout.semantic_highlighting = 'off'
	oo.apply(mut core.Cx{ app: unsafe { h.app } })
	h.frame()
	assert t.classes[0].len == 0
}

fn test_events_toast_progress_status() {
	mut h, o, mut fake := term_rig('events')!
	fake.output('\x1b]9;build done\x07')
	h.frame()
	assert h.has_text('build done')
	fake.output('\x1b]9;4;1;40\x07')
	h.frame()
	assert h.find('progress') != none
	fake.output('\x1b]9;4;0\x07')
	h.frame()
	assert h.find('progress') == none
	s, _ := wire_terminal(o, h)
	fake.output('\x1b]7501;state=working\x07')
	h.frame()
	assert program_status(s) == .working
	fake.exit(0)
	h.frame()
	assert program_status(s) == .idle
}

fn test_a_kitty_release_is_sent_only_after_its_press() {
	mut h, _, mut fake := term_rig('kitty')!
	fake.output('\x1b[>3u')
	h.frame()
	h.press('up')!
	h.frame()
	assert fake.written.bytestr().contains(':3')
}

fn test_a_terminal_logs_its_output() {
	mut h, o, mut fake := term_rig('log')!
	s, _ := wire_terminal(o, h)
	mut x := s.ext
	path := x.start_log(o.log_config())!
	assert is_recording(s)
	fake.output('hello log\r\n')
	x.log.stop()!
	assert !is_recording(s)
	assert os.read_file(path)!.contains('hello log')
}

fn test_find_is_per_terminal_and_survives_a_tab_switch() {
	mut h, o, mut fake := term_rig('find')!
	fake.output('alpha alpha\r\n')
	h.frame()
	h.press('ctrl-f')!
	h.frame()
	assert h.find('find-bar') != none
	s, _ := wire_terminal(o, h)
	cx := core.Cx{
		app: unsafe { h.app }
	}
	assert cx.peek(s.ext.find.input.model()).placeholder == 'Find in terminal'
	h.type_text('alpha')
	h.advance(200)
	assert h.has_text('1/2') // starts at the first match
	h.press('enter')!
	h.frame()
	assert h.has_text('2/2')
	first := o.active
	h.press('ctrl-t')!
	h.frame()
	assert h.find('find-bar') == none // the new terminal has no bar of its own
	h.click('tabs-${first}')!
	h.frame()
	assert h.find('find-bar') != none // still open on the first
	assert h.has_text('2/2')
	h.press('ctrl-f')! // the terminal has the keyboard now: the shortcut closes the bar
	h.frame()
	assert h.find('find-bar') == none
}

fn test_zoom_changes_the_shared_size_of_every_terminal() {
	mut h, o, _ := term_rig('zoom')!
	_, t1 := wire_terminal(o, h)
	base := t1.b.size
	h.press('ctrl-t')!
	h.frame()
	_, t2 := wire_terminal(o, h)
	h.press('ctrl-=')!
	h.frame()
	assert t2.b.size == base + 1
	assert t1.b.size == base + 1
	h.press('ctrl-0')!
	h.frame()
	assert t1.b.size == base && t2.b.size == base
}

fn test_links_go_through_the_policy() {
	mut h, o, mut fake := term_rig('links')!
	fake.output('http://a.example/x https://b.example/y file:///etc/passwd\r\n')
	h.frame()
	_, t := wire_terminal(o, h)
	cell_x := t.bounds.x + t.cell.width * 3.5
	cell_y := t.bounds.y + t.cell.height * 0.5
	// ctrl-click http: the confirmation dialog, nothing opened yet
	ctrl_click(mut h, cell_x, cell_y)
	h.frame()
	assert h.has_text('Open link?')
	assert h.has_text('Open http://a.example/x in your browser?')
	assert h.plat.opened.len == 0
	h.click('confirm-yes')!
	h.frame()
	assert h.plat.opened == ['http://a.example/x']
	// ctrl-click https: opens at once
	x2 := t.bounds.x + t.cell.width * 22.5
	ctrl_click(mut h, x2, cell_y)
	h.frame()
	assert h.plat.opened == ['http://a.example/x', 'https://b.example/y']
	// ctrl-click file:// denied silently
	x3 := t.bounds.x + t.cell.width * 40.5
	ctrl_click(mut h, x3, cell_y)
	h.frame()
	assert h.plat.opened.len == 2
	assert !h.has_text('Open link?')
}

fn ctrl_click(mut h testing.Harness, x f32, y f32) {
	h.move_to(x, y)
	h.inject(platform.MouseEvent{ action: .down, button: .left, x: x, y: y, mods: .ctrl })
	h.inject(platform.MouseEvent{ action: .up, button: .left, x: x, y: y, mods: .ctrl })
}

fn test_weight_names_are_case_insensitive() {
	assert weight_of('Regular') == 400 && weight_of('NORMAL') == 400
	assert weight_of('ExtraLight') == 200 && weight_of('extra_light') == 200
	assert weight_of('EXTRABOLD') == 800 && weight_of('bogus') == 400
}

fn test_settings_reach_the_terminal() {
	mut h, o, _ := term_rig('settings')!
	mut oo := unsafe { o }
	oo.cfg.cursor.color = '#ff00aa'
	oo.cfg.layout.cell_width = 11
	oo.cfg.colors.min_contrast = 7
	oo.cfg.font.features = ['ss01']
	oo.apply(mut core.Cx{ app: unsafe { h.app } })
	h.frame()
	_, t := wire_terminal(o, h)
	assert t.b.cursor_color.a > 0 && t.b.cursor_color.r == 1
	assert t.cell.width == 11
	assert t.b.min_contrast == 7 && t.b.features == ['ss01']
	oo.cfg.colors.min_contrast = 0
	oo.cfg.layout.cell_width = 0
	oo.apply(mut core.Cx{ app: unsafe { h.app } })
	h.frame()
	assert t.b.min_contrast == 4.5 // the original's default, not off
	assert t.cell.width != 11
}
