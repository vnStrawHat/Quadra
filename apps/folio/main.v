// Folio: a Markdown reader and editor (specs/21). Built only with public framework APIs.
//
//   folio [file.md] [--data <folder>]
//
// --data keeps the settings and logs in one folder (tests, portable use).
module main

import os
import quadra.app
import quadra.core
import quadra.kit
import quadra.native
import quadra.platform

const app_id = 'quadra-folio'

fn main() {
	root := arg_value('--data')
	files := os.args[1..].filter(!it.starts_with('--') && it != root)
	mut a := core.new_app(native.backends()!, core.AppOptions{})!
	opts, folio := prepare(mut a, root, files)!
	a.open_window(opts, folio)!
	a.run()
}

fn arg_value(flag string) string {
	i := os.args.index(flag)
	return if i >= 0 && i + 1 < os.args.len { os.args[i + 1] } else { '' }
}

// prepare sets up services, settings, theme and shortcuts and returns the first
// window's options and root view (the tests open it in a harness).
pub fn prepare(mut a core.App, root string, files []string) !(platform.WindowOptions, Folio) {
	s := app.init(mut a, app.AppInfo{
		id:   app_id
		name: 'Folio'
		root: root
	})!
	kit.install(mut a)!
	mut cx := core.Cx{
		app: a
	}
	prefs := app.load_settings[Prefs](mut cx, s.dirs.config, 'prefs', Prefs{}, app.SettingsOptions[Prefs]{
		version: 1
		log:     s.log
	})
	apply_theme(mut a, *cx.peek(prefs.model))
	bind_keys(mut a)!
	windows := app.load_window_states(mut cx, s.dirs.config)
	opts := app.with_placement(cx, windows, 'main', new_window_options())
	return opts, Folio{
		prefs:   prefs
		windows: windows
		pending: files
	}
}

fn new_window_options() platform.WindowOptions {
	return platform.WindowOptions{
		title:           'Folio'
		custom_titlebar: true
		width:           1280
		height:          820
		min_width:       560
		min_height:      360
	}
}
