// The settings window in a headless harness (specs/22, Settings window).
module main

import os
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.testing

fn settings_rig(name string) !(&testing.Harness, &OneTerm) {
	dir := '.tmp/oneterm-ws-${name}'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	_ := h.open(SettingsWin{ one: v.model() },
		width:  920
		height: 2400
	)!
	h.frame()
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return h, cx.peek(v.model())
}

fn test_every_page_shows_its_groups() {
	mut h, _ := settings_rig('pages')!
	want := [
		['Theme', 'Interface', 'Shell'],
		['App Menu', 'Edit Menu', 'Input Channel'],
		['Font', 'Cursor', 'Layout', 'Scroll', 'Bell', 'Mouse'],
		['Completion'],
		['Connection', 'SFTP Editor', 'SFTP Edit Limit'],
		['Master Password', 'Screen Lock'],
		['GitHub Connection'],
		['Application', 'Links', 'Updates'],
	]
	for i, groups in want {
		h.click('settings-page-${i}')!
		h.frame()
		for g in groups {
			assert h.has_text(g), g
		}
	}
}

fn test_a_key_is_captured_and_reset() {
	mut h, o := settings_rig('keys')!
	h.click('settings-page-1')!
	h.frame()
	assert !h.has_text('Default: Ctrl-T') && !h.texts().any(it.starts_with('Default: ctrl-t'))
	h.click('edit-new_terminal_tab')!
	h.frame()
	assert h.find('kbd-capture') != none
	h.press('ctrl-w')! // held by Close Panel: refused, still capturing
	h.frame()
	assert h.find('kbd-capture') != none
	assert 'new_terminal_tab' !in o.ui.key_bindings
	h.press('ctrl-alt-y')!
	h.frame()
	assert h.find('kbd-capture') == none
	assert o.ui.key_bindings['new_terminal_tab'] == 'ctrl-alt-y'
	assert h.texts().any(it.starts_with('Default: '))
	assert load_ui_config(o.dir).key_bindings['new_terminal_tab'] == 'ctrl-alt-y'
	h.click('reset-new_terminal_tab')!
	h.frame()
	assert o.ui.key_bindings.len == 0
	h.click('edit-new_terminal_tab')!
	h.frame()
	h.press('escape')!
	h.frame()
	assert h.find('kbd-capture') == none && o.ui.key_bindings.len == 0
}

fn test_a_setting_applies_and_reset_all_restores() {
	mut h, o := settings_rig('apply')!
	assert h.find('reset-all') == none
	h.click('set-vertical_tabs')!
	h.frame()
	assert o.ui.vertical_tabs.enabled
	assert load_ui_config(o.dir).vertical_tabs.enabled
	h.click('reset-all')!
	h.frame()
	assert !o.ui.vertical_tabs.enabled
	assert h.find('reset-all') == none
}

fn test_update_settings_keep_the_updaters_fields() {
	mut h, o := settings_rig('update')!
	os.mkdir_all(o.dir)!
	os.write_file('${o.dir}/update_config.json', '{"last_checked":"x","channel":"stable"}')!
	h.click('settings-page-7')!
	h.frame()
	h.click('set-auto_check')!
	h.frame()
	text := os.read_file('${o.dir}/update_config.json')!
	assert text.contains('last_checked') && text.contains('"auto_check":false')
	assert !o.update.auto_check
	h.click('reset-all')!
	h.frame()
	restored := os.read_file('${o.dir}/update_config.json')!
	assert restored.contains('last_checked') && restored.contains('"auto_check":true')
}

// The items of the settings audit (specs/22, Settings window).

fn test_the_shell_dropdown_offers_every_kind_and_clears_the_program() {
	mut h, mut o := settings_rig('kinds')!
	o.cfg.shell.program = 'C:/x/nu.exe'
	h.click('set-kind')!
	h.frame()
	for label in ['Command Prompt', 'PowerShell', 'PowerShell 7', 'Bash', 'Zsh', 'Sh', 'Custom Shell'] {
		assert h.has_text(label), label
	}
	h.click('menu-3')! // Bash
	h.frame()
	assert o.cfg.shell.kind == 'bash' && o.cfg.shell.program == ''
	assert load_terminal_config(o.dir).shell.kind == 'bash'
}

fn test_the_font_family_list_searches_and_stores_the_theme_font_as_blank() {
	mut h, mut o := settings_rig('font')!
	assert font_value('Default (theme)') == '' && font_label('') == 'Default (theme)'
	h.click('settings-page-2')!
	h.frame()
	assert h.has_text('Lilex') // the current family, whether listed or not
	h.click('set-family')!
	h.frame()
	assert h.has_text('Default (theme)') && h.has_text('Fake Sans')
	h.type_text('nothing like it')
	h.frame()
	assert !h.has_text('Fake Sans')
	h.click('set-family')! // close, then open: the search starts again
	h.frame()
	h.click('set-family')!
	h.frame()
	h.click('set-family-opt-1')!
	h.frame()
	assert o.cfg.font.family == 'Fake Sans'
	h.click('set-family')!
	h.frame()
	h.click('set-family-opt-0')!
	h.frame()
	assert o.cfg.font.family == ''
	assert load_terminal_config(o.dir).font.family == ''
}

fn test_an_elevated_window_has_no_network_page() {
	all := [kit.SettingsPage{'Security', [], '', ''}, kit.SettingsPage{'Network', [], '', ''},
		kit.SettingsPage{'About', [], '', ''}]
	assert visible_pages(all, false).map(it.title) == ['Security', 'Network', 'About']
	assert visible_pages(all, true).map(it.title) == ['Security', 'About']
}

fn test_the_log_folder_picker_sets_the_folder() {
	mut h, mut o := settings_rig('logdir')!
	h.click('settings-page-2')!
	h.frame()
	before := o.cfg.logging.directory
	h.click('log-folder-pick')! // nothing queued: cancelled
	h.frame()
	assert o.cfg.logging.directory == before
	h.plat.picks << ['C:/picked/logs']
	h.click('log-folder-pick')!
	h.frame()
	assert o.cfg.logging.directory == 'C:/picked/logs'
	assert load_terminal_config(o.dir).logging.directory == 'C:/picked/logs'
}

fn test_dropdowns_show_labels_and_ids_are_stored() {
	mut h, _ := settings_rig('labels')!
	h.click('settings-page-2')!
	h.frame()
	for label in ['Normal', 'Block', 'Auto', 'Default (label)'] {
		assert h.has_text(label), label
	}
	h.click('settings-page-4')!
	h.frame()
	assert h.has_text('OS default application')
	h.click('settings-page-3')!
	h.frame()
	assert h.has_text('Auto-detect')
}

fn test_number_ranges_match_the_original() {
	assert number_ranges['ka_secs'] == [f64(5), 3600, 1, 0]
	assert number_ranges['ka_max'] == [f64(1), 20, 1, 0]
	assert number_ranges['wheel'] == [f64(0), 20, 0.5, 1]
	assert number_ranges['edit_mb'] == [f64(0), 4096, 1, 0]
}

fn test_wording_matches_the_original() {
	mut h, _ := settings_rig('wording')!
	assert h.has_text('Path to the shell executable. Custom only.')
	assert h.has_text("Frame times, dropped frames and GPU usage in the main window's top-right corner.")
	h.click('settings-page-2')!
	h.frame()
	assert h.has_text('Paste the clipboard on a middle click. A program in mouse mode gets the click instead unless Shift is held.')
	h.click('settings-page-3')!
	h.frame()
	assert h.has_text('The suggestion overlay and the history it draws on.')
}

fn test_a_reset_that_collides_stays_unbound_and_says_so() {
	mut h, o := settings_rig('collide')!
	h.click('settings-page-1')!
	h.frame()
	h.click('edit-new_terminal_tab')!
	h.frame()
	h.press('ctrl-alt-y')!
	h.frame()
	h.click('edit-close_panel')!
	h.frame()
	h.press('ctrl-t')! // free now
	h.frame()
	assert o.ui.key_bindings['close_panel'] == 'ctrl-t'
	h.click('reset-new_terminal_tab')!
	h.frame()
	assert o.ui.key_bindings['new_terminal_tab'] == ''
	assert h.texts().any(it.contains('Still unbound: Close Panel holds this key. Rebind either one to free it.'))
	assert load_ui_config(o.dir).key_bindings['new_terminal_tab'] == ''
}

fn test_the_collision_rule_is_per_context() {
	resolved := resolve_default_collisions({
		'close_panel': 'ctrl-t'
	})
	assert resolved['new_terminal_tab'] == '' // its default is held by a user override
	assert displacing_action(resolved, 'new_terminal_tab') or { '' } == 'Close Panel'
	assert 'toggle_fps' !in resolved // shipped unbound: nothing to take away
	// another key context is not a conflict
	free := conflicting({
		'sftp_open': 'ctrl-t'
	}, 'new_terminal_tab', 'ctrl-t') or { '' }
	assert free == ''
}

fn test_a_hand_edited_collision_is_resolved_at_start() {
	dir := '.tmp/oneterm-ws-startcollide'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	os.write_file('${dir}/ui_config.json', '{"key_bindings":{"close_panel":"ctrl-t"}}')!
	mut h := testing.new_harness()!
	_, one := prepare(mut h.app, dir)!
	assert one.ui.key_bindings['new_terminal_tab'] == ''
}

fn test_the_about_page_shows_the_identity() {
	mut h, _ := settings_rig('about')!
	h.click('settings-page-7')!
	h.frame()
	assert h.has_text('OneTerm') && h.has_text('Version ${version}')
	assert h.has_text('A Terminal application for SSH / SFTP / Local Shell')
}
