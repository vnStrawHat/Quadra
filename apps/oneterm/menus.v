// The app menu, the + dropdown and the tab menu, in the original's order (specs/22).
module main

import quadra.core
import quadra.kit
import quadra.prim

const brand_accent = prim.rgb(0x58c4dc)

fn item(label string, cmd Cmd, arg string) kit.MenuItem {
	return kit.MenuItem{
		label:  label
		action: Do{cmd, arg}
	}
}

fn checked(label string, on bool, cmd Cmd, arg string) kit.MenuItem {
	return kit.MenuItem{
		label:   label
		checked: on
		action:  Do{cmd, arg}
	}
}

// PickTheme is a theme row's action in windows that do not handle Do (the settings window).
pub struct PickTheme {
pub:
	one  core.Model[OneTerm]
	name string
}

pub fn (p PickTheme) run(mut cx core.Cx) {
	mut o := cx.peek(p.one)
	o.ui.theme_name = p.name
	if i := theme_info(o.themes, p.name) {
		if i.light {
			o.last_light = p.name
		} else {
			o.last_dark = p.name
		}
	}
	o.apply_and_persist(mut cx)
	cx.touch(p.one)
}

// theme_items are the sectioned rows of a theme menu.
// theme_items: from the settings window `settings` is its OneTerm (no focus chain reaches Do).
fn (o &OneTerm) theme_items(current string, settings core.Model[OneTerm]) []kit.MenuItem {
	mut out := []kit.MenuItem{}
	for r in theme_rows(o.themes, current) {
		if r.heading != '' {
			out << kit.menu_heading(r.heading)
			continue
		}
		mut action := kit.MenuAction(Do{.theme, r.name})
		if settings.id != 0 {
			action = kit.MenuAction(PickTheme{settings, r.name})
		}
		out << kit.MenuItem{
			label:   r.name
			checked: r.name == current
			action:  action
		}
	}
	return out
}

fn (o &OneTerm) current_theme() string {
	return if o.ui.theme_name != '' { o.ui.theme_name } else { default_theme }
}

// OneRef lets a menu build its items when it opens (kit.menu_button_with), not at every
// render: the app menu with its theme list was about 50 KB per root render.
struct OneRef {
	o &OneTerm = unsafe { nil }
}

fn app_menu_items(r OneRef) []kit.MenuItem {
	return r.o.app_menu()
}

fn (o &OneTerm) app_menu() []kit.MenuItem {
	current := o.current_theme()
	light := if i := theme_info(o.themes, current) { i.light } else { false }
	return [item('About', .about, ''), kit.separator(),
		kit.submenu('Appearance', [checked('Light', light, .light, ''),
			checked('Dark', !light, .dark, '')]),
		kit.submenu('Theme', o.theme_items(current, core.Model[OneTerm]{})),
		checked('Show FPS Monitor', o.ui.show_fps, .toggle_fps, ''),
		kit.separator(), item('Settings...', .open_settings, ''),
		item('Lock OneTerm', .lock, ''), kit.separator(), item('Quit', .quit, '')]
}

fn shell_items(cmd Cmd) []kit.MenuItem {
	return shell_choices.map(item(it[1], cmd, it[0]))
}

fn (o &OneTerm) new_tab_items() []kit.MenuItem {
	mut items := shell_items(.new_shell)
	if is_restricted() {
		return items
	}
	items << kit.submenu('Run as administrator', shell_items(.new_admin))
	items << kit.labelled_separator('SSH Sessions')
	sessions := o.saved_sessions()
	if sessions.len == 0 {
		items << kit.MenuItem{
			label:    'No saved sessions'
			disabled: true
		}
	}
	mut groups := []string{}
	for s in sessions {
		if s.group == '' {
			items << session_row(s)
		} else if s.group !in groups {
			groups << s.group
		}
	}
	for g in groups {
		items << kit.labelled_separator(g)
		items << sessions.filter(it.group == g).map(session_row(it))
	}
	items << kit.separator()
	items << item('Quick Connect...', .quick_connect, '')
	items << item('New Saved Session...', .new_saved_session, '')
	return items
}

fn session_row(s SavedSession) kit.MenuItem {
	return kit.MenuItem{
		label:  s.title
		swatch: s.color
		action: Do{.open_saved, s.id}
	}
}

// new_tab_button is xsmall in the strip, small in the rail's header.
fn (o &OneTerm) new_tab_button(size kit.Size) core.Element {
	return kit.menu_button_with('new-tab', '', OneRef{o}, fn (r OneRef) []kit.MenuItem {
		return r.o.new_tab_items()
	}).icon(kit.icon_plus).ghost().size(size).tooltip('New Terminal')
}

// tab_menu acts on the right-clicked tab; `below` names the bulk row for the rail.
fn (o &OneTerm) tab_menu(id string, below bool) []kit.MenuItem {
	i := o.tabs.map(it.id).index(id)
	if i < 0 {
		return []
	}
	tab := o.tabs[i]
	right := kit.MenuItem{
		label:    if below { 'Close Tabs Below' } else { 'Close to the Right' }
		disabled: i == o.tabs.len - 1
		action:   Do{.close_right, id}
	}
	return [item('Rename...', .rename_tab, id), item('Duplicate', .duplicate_tab, id),
		kit.separator(), o.channel_menu(tab), kit.separator(),
		item('Close', .close_tab, id), kit.MenuItem{
			label:    'Close Others'
			disabled: o.tabs.len < 2
			action:   Do{.close_others, id}
		}, right]
}
