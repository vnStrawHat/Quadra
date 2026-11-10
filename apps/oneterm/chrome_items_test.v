// Shells and the chrome: the rename dialog, the lock wording, the strip's menus (specs/22).
module main

import os
import store
import quadra.core
import quadra.kit.term
import quadra.testing

fn test_bash_and_zsh_are_login_shells_with_integration() {
	bash := resolve_shell(ShellConfig{}, 'bash', '')
	assert bash.command.ends_with(' -l')
	assert bash.env['PROMPT_COMMAND'].contains('133;A') && bash.env['PS0'].contains('133;C')
	zsh := resolve_shell(ShellConfig{}, 'zsh', '')
	assert zsh.command.ends_with(' -l')
	assert zsh.env['PS1'].contains('133;D;') && zsh.env['PS1'].contains('133;B')
	assert 'PROMPT_COMMAND' !in zsh.env
	sh := resolve_shell(ShellConfig{}, 'sh', '')
	assert !sh.command.ends_with(' -l') && 'PS1' !in sh.env
}

fn test_the_user_environment_wins_over_the_integration() {
	zsh := resolve_shell(ShellConfig{
		kind: 'zsh'
		env:  {
			'PS1': 'mine> '
		}
	}, '', '')
	assert zsh.env['PS1'] == 'mine> '
	cmd := resolve_shell(ShellConfig{
		env: {
			'PROMPT': 'mine'
		}
	}, '', '')
	assert cmd.env['PROMPT'] == 'mine'
}

fn test_a_user_prompt_command_is_appended_or_left_alone() {
	appended := resolve_shell(ShellConfig{
		kind: 'bash'
		env:  {
			'PROMPT_COMMAND': 'history -a'
		}
	}, '', '')
	assert appended.env['PROMPT_COMMAND'].ends_with('; history -a')
	assert appended.env['PROMPT_COMMAND'].contains('133;A') && 'PS0' in appended.env
	mine := 'emit 133;A'
	optout := resolve_shell(ShellConfig{
		kind: 'bash'
		env:  {
			'PROMPT_COMMAND': mine
		}
	}, '', '')
	assert optout.env['PROMPT_COMMAND'] == mine && 'PS0' !in optout.env
}

fn test_shell_arguments_come_after_the_kinds_own() {
	bash := resolve_shell(ShellConfig{
		kind: 'bash'
		args: ['--noprofile']
	}, '', '')
	assert bash.command.ends_with(' -l --noprofile')
}

fn no_lookup(_ string) ?string {
	return none
}

fn on_path(name string) ?string {
	return '/opt/bin/${name}'
}

fn nothing_exists(_ string) bool {
	return false
}

fn bin_exists(path string) bool {
	return path == '/bin/zsh'
}

fn test_a_unix_shell_is_found_on_path_then_bin_then_shell() {
	assert resolve_unix_shell('bash', '', on_path, nothing_exists) == '/opt/bin/bash'
	assert resolve_unix_shell('zsh', '/x/fish', no_lookup, bin_exists) == '/bin/zsh'
	assert resolve_unix_shell('zsh', '/x/zsh', no_lookup, nothing_exists) == '/x/zsh'
	// $SHELL naming another program never stands in for this one
	assert resolve_unix_shell('zsh', '/bin/bash', no_lookup, nothing_exists) == '/bin/zsh'
}

fn chrome_rig(name string, prep fn (mut s store.SessionStore)) !(&testing.Harness, core.View[OneTerm], &OneTerm) {
	dir := '.tmp/oneterm-chrome-${name}'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	prep(mut one.sessions)
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1200, height: 800)!
	h.frame()
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return h, v, cx.peek(v.model())
}

fn no_sessions(mut _ store.SessionStore) {}

fn ask_rename_active(mut h testing.Harness, v core.View[OneTerm]) {
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	cx.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.ask_rename(o.active, mut cx)
	})
	h.frame()
}

fn clear_field(mut h testing.Harness) ! {
	h.press('ctrl-a')!
	h.press('backspace')!
	h.frame()
}

fn test_rename_saves_cancels_and_refuses_empty() {
	mut h, v, o := chrome_rig('rename', no_sessions)!
	ask_rename_active(mut h, v)
	assert h.has_text('Tab title') && h.has_text('Save') && h.has_text('Cancel')
	clear_field(mut h)!
	h.click('rename-save')!
	h.frame()
	assert h.has_text('Tab title cannot be empty.')
	assert h.has_text('Rename Tab') // stays open
	assert o.tabs[0].name == ''
	h.click('rename-input')! // the click on Save took the focus
	h.type_text('work')
	h.frame()
	h.click('rename-save')!
	h.frame()
	assert o.tabs[0].name == 'work' && !h.has_text('Rename Tab')
	ask_rename_active(mut h, v)
	h.type_text('x')
	h.frame()
	h.click('rename-cancel')!
	h.frame()
	assert o.tabs[0].name == 'work' && !h.has_text('Rename Tab')
}

fn test_the_lock_refusal_says_why() {
	assert lock_refusal('the vault is damaged', false) == 'OneTerm cannot lock: the vault is damaged'
	assert lock_refusal('', true) == 'No master password is set up. Managed from the normal OneTerm window.'
	assert lock_refusal('', false) == 'Set up a master password in Settings > Security first: the lock screen asks for it.'
}

fn test_the_cwd_tooltip() {
	assert cwd_tooltip('C:/a', 'C:/a') == 'Click to copy'
	assert cwd_tooltip('C:/…/a', 'C:/x/a') == 'C:/x/a'
}

fn test_an_empty_space_shows_the_terminal_icon() {
	mut h, v, o := chrome_rig('icon', no_sessions)!
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	cx.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.split(.row, false, mut cx)
	})
	h.frame()
	empty := o.current().spaces[1]
	assert h.find('empty-icon-${empty.id}') != none
	assert h.has_text('Space #${empty.number}')
}

fn sixty_sessions(mut s store.SessionStore) {
	for i in 0 .. 60 {
		s.add(store.Session{ label: 'host${i}', host: '127.0.0.1', username: 'u' })
	}
}

fn test_the_plus_button_has_a_tooltip_and_a_tall_menu_scrolls() {
	mut h, _, _ := chrome_rig('plus', sixty_sessions)!
	r := h.find('new-tab') or { panic('no + button') }
	h.move_to(r.x + 2, r.y + 2)
	h.advance(520)
	assert h.find_role(.tooltip, 'New Terminal') != none
	h.click('new-tab')!
	h.frame()
	list := h.find('menu-list') or { panic('the menu did not cap its height') }
	assert list.height <= 450
}

fn test_the_tab_list_is_headed_and_offers_close() {
	mut h, v, o := chrome_rig('more', no_sessions)!
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	cx.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.new_tab('', '', mut cx)
	})
	h.frame()
	assert o.tabs.len == 2
	h.click('tabs-more')!
	h.frame()
	assert h.has_text('Terminal Tabs') && h.has_text('Close')
	h.click('menu-4')! // heading, two tabs, separator, Close
	h.frame()
	assert o.tabs.len == 1
}
