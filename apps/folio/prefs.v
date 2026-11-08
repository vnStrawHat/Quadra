// Preferences (saved as JSON in the app's config folder) and the theme.
module main

import quadra.core
import quadra.kit

pub enum Layout {
	raw
	split
	preview
}

pub struct Prefs {
pub mut:
	layout    string = 'split' // raw | split | preview
	outline   bool   = true
	theme     string = 'system' // system | dark | light
	scale     f32    = 1        // preview text size
	split     f32    = 0.5      // the editor's share in Split
	outline_w f32    = 220
	recent    []string // most recent first
	session   []string // the files open at the last close, in tab order
	shown     string   // and the one that was shown
}

pub const layouts = ['raw', 'split', 'preview']
pub const max_recent = 10

pub fn layout_of(s string) Layout {
	return match s {
		'raw' { .raw }
		'preview' { .preview }
		else { .split }
	}
}

// with_recent puts path first in the recent list.
pub fn with_recent(list []string, path string) []string {
	mut out := [path]
	for p in list {
		if p != path && out.len < max_recent {
			out << p
		}
	}
	return out
}

// apply_theme sets the theme p.theme names, or follows the system.
pub fn apply_theme(mut a core.App, p Prefs) {
	a.appearance_hooks.clear()
	match p.theme {
		'dark' { kit.set_theme(mut a, kit.dark()) }
		'light' { kit.set_theme(mut a, kit.light()) }
		else { kit.follow_system(mut a, kit.light(), kit.dark()) }
	}
}
