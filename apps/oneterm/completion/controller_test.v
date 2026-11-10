module completion

import json

fn memory_only() Config {
	return Config{
		sources: SourceToggles{
			memory:   true
			manual:   false
			external: false
		}
		fuzzy:   false
	}
}

fn test_gating_blocks_in_alt_screen() {
	mut c := new_controller('cmd', Config{})
	assert c.gating_allows()
	c.set_alt_screen(true)
	assert !c.gating_allows()
	c.set_alt_screen(false)
	assert c.gating_allows()
}

fn test_gating_requires_prompt_region_by_default() {
	mut c := new_controller('cmd', Config{})
	c.set_in_prompt_region(false)
	assert !c.gating_allows()
	c.set_in_prompt_region(true)
	assert c.gating_allows()
}

fn test_disabled_master_switch_blocks() {
	c := new_controller('cmd', Config{ enabled: false })
	assert !c.gating_allows()
}

fn test_recompute_gated_off_yields_no_suggestions() {
	mut c := new_controller('cmd', Config{})
	c.set_alt_screen(true)
	h := new_history(10)
	c.recompute('d', 1, 1000, h, false)
	assert !c.is_visible()
}

fn test_recompute_lists_commands_and_navigation_and_accept() {
	mut c := new_controller('cmd', Config{})
	h := new_history(10)
	c.recompute('di', 2, 1000, h, false)
	assert c.is_visible()
	assert c.suggestions().any(it.text == 'dir')
	assert c.selected() == -1
	assert c.accept() == none
	idx := c.suggestions().map(it.text).index('dir')
	assert c.select_first_if_none()
	for _ in 0 .. idx {
		c.select_next()
	}
	assert c.selected() == idx
	a := c.accept() or { panic('accept') }
	assert a == Accept{0, 'r'}
}

fn test_sole_exact_match_is_hidden_but_actionable_results_remain_visible() {
	mut h := new_history(10)
	h.record(.cmd, 'ls extra', 1000)
	mut c := new_controller('cmd', memory_only())
	c.recompute('ls', 2, 2000, h, false)
	assert !c.is_visible()
	c.recompute('l', 1, 2000, h, false)
	assert c.is_visible()
	c.recompute('LS', 2, 2000, h, false)
	assert c.is_visible()
}

fn test_exact_match_remains_visible_when_other_suggestions_exist() {
	mut h := new_history(10)
	h.record(.cmd, 'ls extra', 1000)
	h.record(.cmd, 'lsof', 1100)
	mut c := new_controller('cmd', memory_only())
	c.recompute('ls', 2, 2000, h, false)
	assert c.is_visible()
	assert c.suggestions().len == 2
}

fn select_row(mut c Controller, text string) {
	idx := c.suggestions().map(it.text).index(text)
	assert idx >= 0
	assert c.select_first_if_none()
	for _ in 0 .. idx {
		c.select_next()
	}
}

fn test_windows_history_acceptance_applies_exact_suggestion_casing() {
	for kind in ['cmd', 'powershell'] {
		family := family_from_kind(kind)
		mut h := new_history(10)
		h.record(family, 'cd Project', 1000)
		for case_ in [['cd ', 'Project'], ['cd p', '\x7fProject'],
			['cd P', 'roject']] {
			line := case_[0]
			mut c := new_controller(kind, Config{})
			c.recompute(line, line.len, 2000, h, false)
			select_row(mut c, 'cd Project')
			a := c.accept() or { panic(line) }
			assert a.bytes() == case_[1].bytes(), line
		}
	}
}

fn test_unix_history_acceptance_remains_case_sensitive() {
	mut h := new_history(10)
	h.record(.unix, 'cd Project', 1000)
	mut c := new_controller('bash', Config{})
	c.recompute('cd p', 4, 2000, h, false)
	assert c.suggestions().all(it.text != 'cd Project')
	c.recompute('cd P', 4, 2000, h, false)
	select_row(mut c, 'cd Project')
	a := c.accept() or { panic('accept') }
	assert a.bytes() == 'roject'.bytes()
}

fn test_navigation_does_nothing_until_first_item_is_selected() {
	mut c := new_controller('cmd', Config{})
	h := new_history(10)
	c.recompute('d', 1, 1000, h, false)
	c.select_next()
	assert c.selected() == -1
	c.select_prev()
	assert c.selected() == -1
	assert c.select_first_if_none()
	assert !c.select_first_if_none()
	assert c.selected() == 0
	c.select_next()
	assert c.selected() == 1
	c.select_prev()
	assert c.selected() == 0
}

fn test_force_bypasses_min_prefix() {
	mut c := new_controller('cmd', Config{ min_prefix_len: 3 })
	h := new_history(10)
	c.recompute('d', 1, 1000, h, false)
	assert !c.is_visible()
	c.recompute('d', 1, 1000, h, true)
	assert c.is_visible()
}

fn test_capture_redacts_before_recording() {
	c := new_controller('bash', Config{})
	mut h := new_history(10)
	c.capture('az login --password S3cr3t!', 1000, mut h)
	assert h.entries(.unix).len == 1
	assert h.entries(.unix)[0].line == 'az login --password'
}

fn test_capture_skips_in_alt_screen() {
	mut c := new_controller('bash', Config{})
	c.set_alt_screen(true)
	mut h := new_history(10)
	c.capture('ls -la', 1000, mut h)
	assert h.is_empty()
}

fn test_max_history_zero_disables_memory_source() {
	assert !params_from_config(Config{ max_history: 0 }).sources.memory
}

fn test_force_family_overrides_kind() {
	c := new_controller('cmd', Config{ force_family: 'unix' })
	assert c.family() == .unix
}

fn test_sync_settings_applies_only_on_change() {
	mut c := new_controller('cmd', Config{})
	c.sync_settings('cmd', Config{})
	assert c.accept_tab()
	c.sync_settings('cmd', Config{ accept_tab: false })
	assert !c.accept_tab()
}

fn test_config_json_keeps_defaults_for_missing_keys() {
	c := json.decode(Config, '{ "max_history": 7, "sources": { "memory": false } }')!
	assert c.max_history == 7 && !c.sources.memory
	assert c.enabled && c.min_prefix_len == 1 && c.max_visible_items == 8 && c.sources.manual
}

fn test_strip_prompt() {
	cmd, found := strip_prompt(r'C:\Users\Trung>d')
	assert cmd == 'd' && found
	unix, ok := strip_prompt('trung@pc:~/proj$ git c')
	assert unix == 'git c' && ok
	plain, none_found := strip_prompt('just some text')
	assert plain == 'just some text' && !none_found
	pwsh, pwsh_found := strip_prompt(r'PS C:\Users\Trung> Get-Ch')
	assert pwsh == 'Get-Ch' && pwsh_found
}

fn test_visible_window() {
	a, b := visible_window(3, -1, 8)
	assert a == 0 && b == 3
	c, d := visible_window(20, -1, 8)
	assert c == 0 && d == 8
	e, f := visible_window(20, 10, 8)
	assert e == 3 && f == 8
}
