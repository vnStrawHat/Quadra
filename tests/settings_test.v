// Settings: debounced atomic saves, corruption recovery, migration (specs/10).
module main

import json
import os
import quadra.app
import quadra.core
import quadra.platform
import quadra.testing

struct Prefs {
mut:
	theme string = 'system'
	size  int    = 14
}

struct PrefsV2 {
mut:
	theme     string = 'system'
	font_size int    = 14
}

fn scratch(name string) string {
	d := os.join_path(os.temp_dir(), 'quadra-tests', name)
	os.rmdir_all(d) or {}
	os.mkdir_all(d) or { panic(err) }
	return d
}

fn harness() (&testing.Harness, core.Cx) {
	mut h := testing.new_harness() or { panic(err) }
	cx := core.Cx{
		app: h.app
	}
	return h, cx
}

fn test_defaults_then_debounced_atomic_save() {
	dir := scratch('settings-save')
	mut h, mut cx := harness()
	s := app.load_settings[Prefs](mut cx, dir, 'prefs', Prefs{}, app.SettingsOptions[Prefs]{
		version: 1
	})
	assert s.get(&cx).theme == 'system'
	for i in 0 .. 3 {
		s.update(mut cx, fn [i] (mut p Prefs, mut _ core.ViewCx[Prefs]) {
			p.size = 14 + i
		})
		h.advance(100)
	}
	assert !os.exists(s.path) // still waiting for the debounce
	h.advance(app.save_delay_ms)
	assert s.state.saves == 1
	assert !os.exists(s.path + '.tmp')
	mut cx2 := core.Cx{
		app: h.app
	}
	again := app.load_settings[Prefs](mut cx2, dir, 'prefs', Prefs{}, app.SettingsOptions[Prefs]{
		version: 1
	})
	assert again.get(&cx2).size == 16
}

fn test_corrupt_file_is_moved_aside() {
	dir := scratch('settings-corrupt')
	os.write_file(os.join_path(dir, 'prefs.json'), '{"version":1,"data":{"theme": ')!
	_, mut cx := harness()
	mut log := app.new_logger('', 'test', .info)
	s := app.load_settings[Prefs](mut cx, dir, 'prefs', Prefs{}, app.SettingsOptions[Prefs]{
		version: 1
		log:     log
	})
	assert s.get(&cx).theme == 'system'
	assert !os.exists(s.path)
	assert os.ls(dir)!.any(it.starts_with('prefs.json.corrupt-'))
	assert log.recent(10).any(it.level == .warn && it.msg.contains('using defaults'))
}

fn test_migration_from_older_version() {
	dir := scratch('settings-migrate')
	os.write_file(os.join_path(dir, 'prefs.json'),
		'{"version":1,"data":{"theme":"dark","size":18}}')!
	_, mut cx := harness()
	s := app.load_settings[PrefsV2](mut cx, dir, 'prefs', PrefsV2{}, app.SettingsOptions[PrefsV2]{
		version: 2
		migrate: fn (from int, data string) !PrefsV2 {
			assert from == 1
			old := json.decode(Prefs, data)!
			return PrefsV2{
				theme:     old.theme
				font_size: old.size
			}
		}
	})
	assert s.get(&cx).font_size == 18
	assert s.get(&cx).theme == 'dark'
}

fn test_flush_writes_immediately() {
	dir := scratch('settings-flush')
	_, mut cx := harness()
	s := app.load_settings[Prefs](mut cx, dir, 'prefs', Prefs{}, app.SettingsOptions[Prefs]{})
	s.update(mut cx, fn (mut p Prefs, mut _ core.ViewCx[Prefs]) {
		p.theme = 'dark'
	})
	s.flush(mut cx)!
	assert os.read_file(s.path)!.contains('"theme":"dark"')
	// replace_file must overwrite an existing file (MSVCRT rename does not).
	os.write_file(s.path + '.new', 'replaced')!
	platform.replace_file(s.path + '.new', s.path)!
	assert os.read_file(s.path)! == 'replaced'
}
