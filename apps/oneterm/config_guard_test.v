// The config files' read outcomes (specs/22, Config): what may be overwritten, and what not.
module main

import json
import os
import quadra.kit.term
import quadra.testing

fn fresh(name string) string {
	dir := '.tmp/oneterm-guard-${name}'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

// A directory in place of the file fails to read with something other than "not found", the
// stand-in for a permission failure the original's own test uses.
fn test_an_unreadable_config_is_never_written_over() {
	dir := fresh('unreadable')
	os.mkdir_all('${dir}/ui_config.json')!
	os.mkdir_all('${dir}/terminal.json')!
	ui_cfg := read_ui_config(dir, false)
	cfg := read_terminal_config(dir, false)
	assert ui_cfg.persist_blocked && cfg.persist_blocked
	assert os.is_dir('${dir}/ui_config.json') && os.is_dir('${dir}/terminal.json')
	o := OneTerm{
		ui:  ui_cfg
		cfg: cfg
		dir: dir
	}
	o.persist()!
	o.persist_ui()!
	assert os.is_dir('${dir}/ui_config.json') && os.is_dir('${dir}/terminal.json')
	assert !os.exists('${dir}/ui_config.json.bak') && !os.exists('${dir}/ui_config.json.tmp')
	assert !os.ls(dir)!.any(it.contains('corrupt'))
}

// Blocking is per document: the readable one still saves.
fn test_one_unreadable_document_does_not_block_the_other() {
	dir := fresh('one')
	os.mkdir_all('${dir}/terminal.json')!
	mut o := OneTerm{
		ui:  read_ui_config(dir, false)
		cfg: read_terminal_config(dir, false)
		dir: dir
	}
	assert !o.ui.persist_blocked && o.cfg.persist_blocked
	o.ui.show_fps = true
	o.persist()!
	assert read_ui_config(dir, false).show_fps
	assert os.is_dir('${dir}/terminal.json')
}

fn test_persist_blocked_is_never_saved() {
	assert !json.encode_pretty(UiConfig{ persist_blocked: true }).contains('persist_blocked')
	assert !json.encode_pretty(TerminalConfig{ persist_blocked: true }).contains('persist_blocked')
}

fn test_first_run_creates_the_files_with_the_defaults() {
	dir := fresh('first')
	assert read_terminal_config(dir, false).font.family == 'Lilex'
	ui_cfg := read_ui_config(dir, false)
	assert !ui_cfg.persist_blocked && ui_cfg.ui_font_size == 16
	assert os.is_file('${dir}/terminal.json') && os.is_file('${dir}/ui_config.json')
	assert json.decode(TerminalConfig, os.read_file('${dir}/terminal.json')!)!.font.size == 15
}

fn test_an_elevated_window_writes_nothing_and_is_blocked() {
	dir := fresh('elevated')
	assert read_ui_config(dir, true).persist_blocked
	assert !read_terminal_config(dir, true).persist_blocked
	assert os.ls(dir)!.len == 0
}

fn test_the_elevated_notice() {
	text := 'Settings could not be read for this account; this window is using the defaults.'
	assert elevated_settings_notice(true, true) == text
	assert elevated_settings_notice(true, false) == ''
	assert elevated_settings_notice(false, true) == ''
}

fn ui_from(doc string) UiConfig {
	dir := fresh('lenient')
	os.write_file('${dir}/ui_config.json', doc) or { panic(err) }
	c := read_ui_config(dir, false)
	assert os.exists('${dir}/ui_config.json'), 'quarantined: ${doc}'
	return c
}

fn minutes_of(value string) UiConfig {
	return ui_from('{"theme_name":"T","lock_after_idle_minutes":${value}}')
}

fn test_a_bad_idle_timeout_costs_only_itself() {
	assert minutes_of('60').lock_timeout_minutes() == 60
	assert minutes_of('0').lock_timeout_minutes() == 1
	assert minutes_of('100000').lock_timeout_minutes() == 480
	for wrong in ['"ten"', '-5', '2.5', 'true', 'null'] {
		c := minutes_of(wrong)
		assert c.lock_timeout_minutes() == 15, wrong
		assert c.theme_name == 'T', wrong
	}
}

fn tabs_of(value string) UiConfig {
	mut c := ui_from('{"theme_name":"Kept","vertical_tabs":${value}}')
	c.normalize()
	return c
}

fn test_a_bad_vertical_tabs_value_costs_only_itself() {
	def := VerticalTabs{}
	for wrong in ['"on"', '5', 'true', '[true]', 'null', '{"enabled":"yes"}', '{"enabled":1}',
		'{"enabled":null}'] {
		c := tabs_of(wrong)
		assert c.vertical_tabs.enabled == def.enabled && c.vertical_tabs.width == def.width, wrong
		assert c.theme_name == 'Kept', wrong
	}
	partial := tabs_of('{"enabled":true,"width":"wide","collapsed":"no"}').vertical_tabs
	assert partial.enabled && partial.width == 220 && !partial.collapsed
	assert tabs_of('{"width":260.4}').vertical_tabs.width == 260
	assert tabs_of('{"width":"100000"}').vertical_tabs.width == 220
	assert tabs_of('{"width":100000}').vertical_tabs.width == 400
	assert tabs_of('{"width":-30}').vertical_tabs.width == 140
}

// A clean document is decoded from its own text, not a re-encoded copy.
fn test_a_clean_ui_document_is_not_rewritten() {
	doc := '{"theme_name":"Café","lock_after_idle_minutes":30,"vertical_tabs":{"enabled":true,"width":300}}'
	assert lenient_ui_document(doc) == doc
	assert lenient_ui_document('not json') == 'not json'
}

// The Settings window's own writes go through the guard, end to end.
fn test_a_setting_change_leaves_an_unreadable_file_alone() {
	dir := fresh('rig')
	os.mkdir_all('${dir}/ui_config.json')!
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	assert one.ui.persist_blocked
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	_ := h.open(SettingsWin{ one: v.model() },
		width:  920
		height: 2400
	)!
	h.frame()
	h.click('set-vertical_tabs')!
	h.frame()
	assert os.is_dir('${dir}/ui_config.json')
	assert os.is_file('${dir}/terminal.json')
}
