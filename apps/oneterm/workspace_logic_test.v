// Workspace shell logic (specs/22): no window, the pure rules of the status bar, dock, themes,
// key bindings and elevation.
module main

import os

fn test_every_bundled_theme_parses() {
	assert bundled_themes.len == 39
	for b in bundled_themes {
		t := parse_theme(ThemeInfo{b.name, b.mode == 'light', b.text, ''}) or {
			assert false, '${b.name}: ${err}'
			return
		}
		assert t.name == b.name
		assert (t.mode == .light) == (b.mode == 'light')
	}
}

fn test_theme_rows_section_light_and_dark() {
	list := [ThemeInfo{'b dark', false, '', ''}, ThemeInfo{'A dark', false, '', ''},
		ThemeInfo{'L', true, '', ''}]
	rows := theme_rows(list, 'b dark')
	assert rows.map(it.heading) == ['Dark themes', '', '', 'Light themes', '']
	assert rows.map(it.name) == ['', 'b dark', 'A dark', '', 'L']
	other := theme_rows(list, 'nope')
	assert other[0].heading == 'Light themes'
	assert other.map(it.name) == ['', 'L', '', 'A dark', 'b dark']
}

fn test_dock_and_rail_widths_clamp() {
	assert clamp_right_dock_width(480, 2000) == 480
	assert clamp_right_dock_width(9000, 2000) == 700 // 35 %
	assert clamp_right_dock_width(10, 2000) == 240
	assert clamp_right_dock_width(480, 400) == 240 // the floor wins in a narrow window
	assert clamp_tab_rail_width(220, 1000) == 220
	assert clamp_tab_rail_width(9000, 3000) == 400
	assert clamp_tab_rail_width(9000, 500) == 175
	assert clamp_tab_rail_width(10, 300) == 140
}

fn test_status_bar_centre_and_elision() {
	assert elide_left('C:/a/b/c', 20, fn (s string) f32 {
		return f32(s.len)
	}) == 'C:/a/b/c'
	short := elide_left('C:/aaaa/bbbb/cccc', 8, fn (s string) f32 {
		return f32(s.runes().len)
	})
	assert short.starts_with('…') && short.ends_with('cccc') && short.runes().len <= 8
	end := elide_right('abcdefghij', 5, fn (s string) f32 {
		return f32(s.runes().len)
	})
	assert end.ends_with('…') && end.starts_with('abc') && end.runes().len <= 5
}

fn test_git_label() {
	assert build_git_label('# branch.head main\n', 0, 0).whole() == 'main'
	dirty := build_git_label('# branch.head main\n# branch.ab +3 -1\n1 .M N... 100644 100644 100644 a b f\n',
		1, 2)
	assert dirty.whole().starts_with('main*')
	assert dirty.whole().contains('+1') && dirty.whole().contains('-2')
	assert dirty.whole().contains('↑3') && dirty.whole().contains('↓1')
	added, removed := parse_numstat('1\t2\ta.txt\n-\t-\tb.bin\n3\t0\tc\n')
	assert added == 4 && removed == 2
}

fn test_formats() {
	assert format_memory(512 * 1048576).contains('512')
	assert format_memory(3 * 1073741824).contains('3')
	assert format_duration(61000) != ''
	assert format_speed(0) != ''
	assert format_speed(2048).contains('2')
}

fn test_key_binding_rules() {
	id := bindables[0].id
	def := key_default(id)
	assert show_default(map[string]string{}, id) == ''
	assert without_defaults({
		id: def
	}).len == 0
	moved := {
		id: 'ctrl-alt-f9'
	}
	assert effective_key(moved, id) == 'ctrl-alt-f9'
	assert !is_at_default(moved, id)
	assert show_default(moved, id).starts_with('Default: ')
	assert without_defaults(moved).len == 1
	if def != '' {
		other := bindables[1].id
		taken := conflicting(map[string]string{}, other, def) or {
			assert false, 'the key is taken'
			return
		}
		assert taken != ''
		same := conflicting(map[string]string{}, id, def)
		assert same == none
		// freeing the key frees the conflict
		freed := conflicting(moved, other, def)
		assert freed == none
	}
}

fn test_every_bindable_is_classified_for_an_elevated_window() {
	for b in bindables {
		// the elevated table is exhaustive: an unknown id is denied, a known one has one answer
		assert action_allowed_when_elevated(b.id, false)
		_ := action_allowed_when_elevated(b.id, true)
	}
	assert !action_allowed_when_elevated('an_action_nobody_listed', true)
}

fn test_elevation_policy() {
	assert window_title(.elevated).ends_with('(Administrator)')
	assert window_title(.not_elevated) == 'OneTerm'
	assert parse_elevated_shell(['oneterm.exe'])! == ''
	assert parse_elevated_shell(['oneterm.exe', '--elevated-shell', 'cmd'])! == 'cmd'
	if _ := parse_elevated_shell(['oneterm.exe', '--elevated-shell', 'calc.exe']) {
		assert false, 'only the three trusted shells are accepted'
	}
	assert should_start_launch(false)
	assert !should_start_launch(true)
}

fn test_config_new_blocks_round_trip() {
	dir := '.tmp/oneterm-ws-config'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	c := load_terminal_config(dir)
	assert c.completion.enabled && c.completion.sources.memory
	assert c.ssh.keepalive_enabled
	assert c.logging.write_mode == 'append'
	u := load_ui_config(dir)
	assert u.lock_timeout_minutes() == 15
	assert !u.vertical_tabs.enabled && u.vertical_tabs.width == 220
}
