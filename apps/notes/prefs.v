// User preferences (persisted settings) and how they apply to the app.
module main

import quadra.app
import quadra.core
import quadra.kit

pub struct Prefs {
pub mut:
	theme    string = 'system'  // system | light | dark
	density  string = 'compact' // compact | comfortable
	language string = 'en'
	split    f32    = 0.38 // sidebar+list share of the window width
}

pub const themes = ['system', 'light', 'dark']
pub const densities = ['compact', 'comfortable']
pub const languages = ['en', 'vi']

// apply_prefs sets the theme, density and language from prefs.
pub fn apply_prefs(mut a core.App, p Prefs) {
	density := if p.density == 'comfortable' { kit.Density.comfortable } else { kit.Density.compact }
	a.appearance_hooks.clear()
	match p.theme {
		'light' {
			kit.set_theme(mut a, kit.Theme{
				...kit.light()
				density: density
			})
		}
		'dark' {
			kit.set_theme(mut a, kit.Theme{
				...kit.dark()
				density: density
			})
		}
		else {
			kit.follow_system(mut a, kit.Theme{
				...kit.light()
				density: density
			}, kit.Theme{
				...kit.dark()
				density: density
			})
		}
	}

	mut cx := core.Cx{
		app: a
	}
	if app.locale(cx) != p.language {
		app.set_locale(mut cx, p.language)
	}
	words := if p.language == 'vi' { kit.strings_vi() } else { kit.Strings{} }
	kit.set_strings(mut a, words)
}

// update_prefs changes the preferences, saves them (debounced) and applies them.
pub fn update_prefs(mut cx core.Cx, s app.Settings[Prefs], f fn (mut p Prefs, mut cx core.ViewCx[Prefs])) {
	s.update(mut cx, f)
	apply_prefs(mut cx.app, *cx.peek(s.model))
}

// update_prefs_with is update_prefs for a callback that needs data (no closure, specs/14).
pub fn update_prefs_with[D](mut cx core.Cx, s app.Settings[Prefs], data D, f fn (mut p Prefs, d D, mut cx core.ViewCx[Prefs])) {
	app.update_with(mut cx, s, data, f)
	apply_prefs(mut cx.app, *cx.peek(s.model))
}
