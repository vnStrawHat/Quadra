module main

import os
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.testing

fn open(mut h testing.Harness) !(core.View[OneTerm], &term.FakePty) {
	_, mut one := prepare(mut h.app, '.tmp/oneterm-test')!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1000, height: 700)!
	return v, fake
}

fn state(h &testing.Harness, v core.View[OneTerm]) &OneTerm {
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return cx.peek(v.model())
}

fn test_config_defaults_and_partial_files() {
	dir := '.tmp/oneterm-config'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	assert load_terminal_config(dir).font.size == 15
	os.write_file('${dir}/terminal.json',
		'{ // the only change\n "font": {"size": 12},\n /* block */ "shell": {"kind": "pwsh"} }')!
	c := load_terminal_config(dir)
	assert c.font.size == 12 && c.shell.kind == 'pwsh'
	assert c.font.family == 'Lilex' && c.shell.utf8 && c.scroll.scrollback_history == 10000
	assert c.layout.padding.left == 10 && c.mouse.copy_on_select
	os.write_file('${dir}/terminal.json', '{ nope')!
	assert load_terminal_config(dir).font.size == 15
	assert os.ls(dir)!.any(it.starts_with('terminal.json.corrupt-'))
}

fn test_comments_in_strings_survive() {
	assert strip_comments('{"a": "http://x"} // c') == '{"a": "http://x"} '
}

fn test_shell_command_lines_and_environment() {
	cmd := resolve_shell(ShellConfig{}, '', '')
	assert cmd.command.ends_with('/K chcp 65001 >nul') && cmd.name == 'Command Prompt'
	assert cmd.env['PROMPT'].contains(']133;A') && cmd.env['TERM'] == 'xterm-256color'
	assert cmd.env['WT_SESSION'] == '' && 'WT_SESSION' in cmd.env
	ps := resolve_shell(ShellConfig{}, 'pwsh', 'C:/w')
	assert ps.command.starts_with('pwsh.exe -NoLogo -NoExit -Command') && ps.cwd == 'C:/w'
	custom := resolve_shell(ShellConfig{
		kind:    'custom'
		program: 'C:/x/nu.exe'
		args:    [
			'-l',
		]
	}, '', '')
	assert custom.command == 'C:/x/nu.exe -l' && custom.name == 'nu'
	lines := platform.merged_env({
		'Path': 'a'
		'B':    'b'
		'WT':   'c'
	}, {
		'PATH': 'z'
		'wt':   ''
	})
	assert lines == ['B=b', 'PATH=z']
}

fn test_tabs_open_close_and_reset() {
	mut h := testing.new_harness()!
	v, fake := open(mut h)!
	assert fake.opts.env['TERM_PROGRAM'] == 'OneTerm'
	assert state(h, v).tabs.len == 1
	h.press('ctrl-t')!
	h.frame()
	assert state(h, v).tabs.len == 2
	h.press('ctrl-w')!
	h.frame()
	assert state(h, v).tabs.len == 1
	h.press('ctrl-w')! // the last tab resets to an empty one
	h.frame()
	o := state(h, v)
	assert o.tabs.len == 1 && o.tabs[0].spaces[0].view.id == 0
	assert h.texts().any(it.starts_with('Space #'))
}

fn test_spaces_split_and_close() {
	mut h := testing.new_harness()!
	v, _ := open(mut h)!
	h.press('ctrl-shift-right')!
	h.frame()
	mut o := state(h, v)
	assert o.current().spaces.len == 2 && o.current().spaces[1].view.id == 0
	assert o.current().active == o.current().spaces[1].id
	assert h.texts().any(it.starts_with('Right-click'))
	h.press('ctrl-shift-down')!
	h.frame()
	assert o.current().spaces.len == 3
	h.press('ctrl-shift-x')!
	h.press('ctrl-shift-x')!
	h.frame()
	assert o.current().spaces.len == 1 && o.current().root.is_leaf()
}

fn test_tab_title_follows_the_shell_and_cwd_reaches_the_status_bar() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	fake.output('\x1b]7;C:\\Users\\me\x1b\\')
	h.frame()
	assert h.has_text('C:\\Users\\me')
	assert title(h, v) == 'Command Prompt'
}

fn title(h &testing.Harness, v core.View[OneTerm]) string {
	return state(h, v).title_of(state(h, v).tabs[0], &core.Cx{
		app: unsafe { h.app }
	})
}

// active returns the active Space's terminal.
fn active(h &testing.Harness, v core.View[OneTerm]) &term.Terminal {
	cx := core.Cx{
		app: unsafe { h.app }
	}
	o := state(h, v)
	return cx.peek(o.current().spaces[0].view.model())
}

fn cell_x(t &term.Terminal, col int) f32 {
	return t.bounds.x + (f32(col) + 0.5) * t.cell.width
}

fn cell_y(t &term.Terminal, row int) f32 {
	return t.bounds.y + (f32(row) + 0.5) * t.cell.height
}

fn test_osc_titles_follow_the_setting_and_drop_the_folder() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	fake.output('\x1b]2;C:\\tools\\htop.exe\x07')
	h.frame()
	assert title(h, v) == 'Command Prompt'
	mut o := state(h, v)
	o.cfg.layout.tab_title = 'osc'
	assert title(h, v) == 'htop.exe'
}

fn test_padding_shrinks_the_grid() {
	mut h := testing.new_harness()!
	v, _ := open(mut h)!
	t := active(h, v)
	assert t.bounds.x == 10 // padding.left
	assert t.screen.cols == int((1000 - 15) / t.cell.width)
}

fn test_a_selection_is_copied_and_the_middle_button_pastes_it() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	fake.output('hello world')
	h.frame()
	t := active(h, v)
	h.drag(t.bounds.x + 1, cell_y(t, 0), t.bounds.x + 5 * t.cell.width + 1, cell_y(t, 0), true)
	assert t.selected_text() == 'hello'
	fake.written.clear()
	h.inject(platform.MouseEvent{
		action: .down
		button: .middle
		x:      cell_x(t, 2)
		y:      cell_y(t, 2)
	})
	assert fake.written.bytestr() == 'hello'
}

fn test_the_wheel_goes_to_the_program_or_the_arrows() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	h.frame()
	t := active(h, v)
	fake.output('\x1b[?1049h')
	h.frame()
	fake.written.clear()
	h.scroll_at(cell_x(t, 1), cell_y(t, 1), t.cell.height)
	assert fake.written.bytestr() == '\x1b[A'
	fake.output('\x1b[?1000h\x1b[?1006h')
	h.frame()
	fake.written.clear()
	h.scroll_at(cell_x(t, 1), cell_y(t, 1), -t.cell.height)
	assert fake.written.bytestr() == '\x1b[<65;2;2M'
}

fn test_the_bell_badge_clears_when_typing() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	fake.output('\x07')
	h.frame()
	t := active(h, v)
	assert h.find('${t.id}-bell') != none
	h.type_text('x')
	h.frame()
	assert h.find('${t.id}-bell') == none
}

fn test_zoom_is_not_typed() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	h.frame()
	fake.written.clear()
	base := active(h, v).b.size
	h.press('ctrl-=')!
	h.press('ctrl-shift-=')! // `ctrl-+`
	h.press('ctrl-minus')!
	assert active(h, v).b.size == base + 1 && fake.written.len == 0
	h.press('ctrl-0')!
	assert active(h, v).b.size == base
}

fn test_the_search_bar() {
	mut h := testing.new_harness()!
	v, mut fake := open(mut h)!
	fake.output('one two\r\none\r\n')
	h.frame()
	h.press('ctrl-f')!
	assert h.find('find-bar') != none
	h.type_text('one')
	h.advance(200)
	assert h.has_text('1/2') // the first match, oldest first
	h.press('enter')!
	h.frame()
	assert h.has_text('2/2')
	h.press('shift-enter')!
	h.frame()
	assert h.has_text('1/2')
	h.press('escape')!
	h.frame()
	assert h.find('find-bar') == none
	assert active(h, v).find.hits.len == 0
}

fn test_settings_apply_at_once_and_are_saved() {
	dir := '.tmp/oneterm-save'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	mut o := state(h, v)
	o.cfg.cursor.shape = 'bar'
	o.ui.theme_name = 'One Light'
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	o.apply_and_persist(mut cx)
	h.frame()
	assert active(h, v).b.cursor == .bar
	assert kit.theme_of(h.app).name == 'One Light'
	assert load_terminal_config(dir).cursor.shape == 'bar'
	assert load_ui_config(dir).theme_name == 'One Light'
	o.apply_and_persist(mut cx)
	assert os.exists('${dir}/terminal.json.bak')
}

fn test_key_bindings_can_be_replaced() {
	mut h := testing.new_harness()!
	v, _ := open(mut h)!
	mut o := state(h, v)
	h.app.unbind[Do]()
	o.ui.key_bindings['new_terminal_tab'] = 'ctrl-y'
	bind_keys(mut h.app, o.ui.key_bindings)!
	h.press('ctrl-t')!
	h.frame()
	assert o.tabs.len == 1
	h.press('ctrl-y')!
	h.frame()
	assert o.tabs.len == 2
	o.ui.key_bindings['new_terminal_tab'] = ''
	h.app.unbind[Do]()
	bind_keys(mut h.app, o.ui.key_bindings)!
	h.press('ctrl-y')!
	h.frame()
	assert o.tabs.len == 2
}

fn test_the_settings_window_opens() {
	mut h := testing.new_harness()!
	_, _ := open(mut h)!
	h.press('ctrl-,')!
	h.frame()
	assert h.app.windows.len == 2
}
