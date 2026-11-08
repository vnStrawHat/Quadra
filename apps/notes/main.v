// Quadra Notes: the reference application (specs/13). Built only with public framework APIs.
//
//   quadra-notes [--data <folder>] [files to import...]
//
// --data keeps every file (notes, settings, logs, crash dumps) in one folder.
module main

import os
import quadra.app
import quadra.core
import quadra.kit
import quadra.native
import quadra.platform

const app_id = 'quadra-notes'

fn main() {
	root := arg_value('--data')
	files := os.args[1..].filter(!it.starts_with('--') && it != root)
	mut a := core.new_app(native.backends()!, core.AppOptions{})!
	opts, shell := prepare(mut a, root, files) or {
		if err.code() == app.already_running {
			return
		}
		panic(err)
	}
	v := a.open_window(opts, shell)!
	if files.len > 0 {
		mut cx := core.Cx{
			app: a
		}
		import_paths(mut cx, cx.peek(v.model()).notes, files, '')
	}
	a.run()
}

fn arg_value(flag string) string {
	i := os.args.index(flag)
	return if i >= 0 && i + 1 < os.args.len { os.args[i + 1] } else { '' }
}

// prepare sets up services, settings, theme and shortcuts, starts loading the notes and
// returns the main window's options and root view (the tests open it in a harness).
pub fn prepare(mut a core.App, root string, args []string) !(platform.WindowOptions, Shell) {
	s := app.init(mut a, app.AppInfo{
		id:              app_id
		name:            'Quadra Notes'
		root:            root
		single_instance: true
		args:            args
		locales:         {
			'en': $embed_file('i18n/en.json').to_string()
			'vi': $embed_file('i18n/vi.json').to_string()
		}
	})!
	kit.install(mut a)!
	mut cx := core.Cx{
		app: a
	}
	prefs := app.load_settings[Prefs](mut cx, s.dirs.config, 'prefs', Prefs{}, app.SettingsOptions[Prefs]{
		version: 1
		log:     s.log
	})
	apply_prefs(mut a, *cx.peek(prefs.model))
	windows := app.load_window_states(mut cx, s.dirs.config)
	bind_keys(mut a)!
	notes := cx.new_model(Notes{
		root: os.join_path(s.dirs.data, 'notes')
	})
	load_notes(mut cx, notes)
	opts := app.with_placement(cx, windows, 'main', platform.WindowOptions{
		title:           'Quadra Notes'
		custom_titlebar: true
		width:           1100
		height:          720
		min_width:       640
		min_height:      420
	})
	return opts, Shell{
		notes:   notes
		prefs:   prefs
		windows: windows
	}
}
