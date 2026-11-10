// Window state: placements saved per window key and restored, clamped by the platform to
// the monitors that are still connected.
module app

import quadra.core
import quadra.platform

pub struct WindowStates {
pub mut:
	placements map[string]platform.Placement
}

// load_window_states reads <config>/windows.json.
pub fn load_window_states(mut cx core.Cx, dir string) Settings[WindowStates] {
	return load_settings[WindowStates](mut cx, dir, 'windows', WindowStates{}, SettingsOptions[WindowStates]{})
}

// with_placement returns opts with the saved placement of key, if any.
pub fn with_placement(cx &core.Cx, s Settings[WindowStates], key string, opts platform.WindowOptions) platform.WindowOptions {
	st := cx.peek(s.model)
	p := st.placements[key] or { return opts }
	return platform.WindowOptions{
		...opts
		placement: p
	}
}

// remember_placement saves the current window's placement under key (call it from the
// window's close handler).
pub fn remember_placement(mut cx core.Cx, s Settings[WindowStates], key string) {
	p := cx.win.pw.placement()
	if !p.valid {
		return
	}
	update_with(mut cx, s, Placed{key, p}, fn (mut st WindowStates, d Placed, mut _ core.ViewCx[WindowStates]) {
		st.placements[d.key] = d.p
	})
}

struct Placed {
	key string
	p   platform.Placement
}
