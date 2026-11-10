// Themes (specs/22, Config): the bundled files, parsed only when applied, and <config>/themes.
module main

import os
import quadra.kit

struct BundledTheme {
	name string
	mode string
	text string
}

pub struct ThemeInfo {
pub:
	name  string
	light bool
	text  string // a bundled theme's JSON
	path  string // a user theme's file
}

const default_theme = 'One Dark'

// theme_list lists the bundled themes, then the files in <config>/themes; a bad file is skipped.
pub fn theme_list(dir string) []ThemeInfo {
	mut out := []ThemeInfo{cap: bundled_themes.len}
	for b in bundled_themes {
		out << ThemeInfo{b.name, b.mode == 'light', b.text, ''}
	}
	folder := os.join_path(dir, 'themes')
	mut files := os.ls(folder) or { return out }
	files.sort()
	for f in files {
		if f.ends_with('.json') {
			path := os.join_path(folder, f)
			t := kit.load_theme(path) or { continue }
			out << ThemeInfo{t.name, t.mode == .light, '', path}
		}
	}
	return out
}

fn theme_info(list []ThemeInfo, name string) ?ThemeInfo {
	for i in list {
		if i.name == name {
			return i
		}
	}
	return none
}

fn parse_theme(i ThemeInfo) !kit.Theme {
	return if i.path != '' { kit.load_theme(i.path)! } else { kit.theme_from_json(i.text)! }
}

// theme_for picks the configured theme (the default when the name is unknown or unset).
pub fn theme_for(dir string, ui UiConfig, colors ColorsConfig) kit.Theme {
	all := theme_list(dir)
	name := if ui.theme_name != '' { ui.theme_name } else { default_theme }
	if i := theme_info(all, name) {
		if t := parse_theme(i) {
			return themed(t, colors, ui.ui_font_size)
		}
	}
	if i := theme_info(all, default_theme) {
		if t := parse_theme(i) {
			return themed(t, colors, ui.ui_font_size)
		}
	}
	return themed(kit.dark(), colors, ui.ui_font_size)
}

// ThemeRow is a row of the theme picker: a section heading, or a theme.
pub struct ThemeRow {
pub:
	heading string
	name    string
}

// theme_rows sections the list Light / Dark. The section holding the current theme comes
// first and the current theme leads it; the rest keep case-insensitive name order.
pub fn theme_rows(list []ThemeInfo, current string) []ThemeRow {
	mine := theme_info(list, current)
	first_light := if m := mine { m.light } else { true }
	mut out := []ThemeRow{}
	for light in [first_light, !first_light] {
		mut names := list.filter(it.light == light).map(it.name)
		if names.len == 0 {
			continue
		}
		names.sort_with_compare(fn (a &string, b &string) int {
			return a.to_lower().compare(b.to_lower())
		})
		if mine != none && light == first_light {
			mut lead := [current]
			lead << names.filter(it != current)
			names = lead.clone()
		}
		out << ThemeRow{
			heading: if light { 'Light themes' } else { 'Dark themes' }
		}
		out << names.map(ThemeRow{ name: it })
	}
	return out
}
