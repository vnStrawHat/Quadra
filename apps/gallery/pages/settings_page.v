// Settings page: the settings component in a frame, with a working search and switches.
module pages

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub struct SettingsDemo {
mut:
	search     core.View[input.TextInput]
	query      string
	current    int
	scope      int
	accessible bool
	trust      bool = true
	blink      bool = true
	closing    int
	quit       int
	mode       int = 2
}

@[markused]
pub fn (mut p SettingsDemo) render(mut cx core.ViewCx[SettingsDemo]) core.Element {
	if p.search.id == 0 {
		p.search = input.new_input(mut cx.Cx,
			id:          'set-search'
			placeholder: kit.strings_of(cx.app).search_settings
			prefix:      kit.icon_search
		)
		input.on_change(mut cx.Cx, p.search, cx.listener(fn (mut p SettingsDemo, v &string, mut _ core.ViewCx[SettingsDemo]) {
			p.query = *v
		}))
	}
	general := kit.SettingsPage{'General', [
		kit.SettingsGroup{'General Settings', [
			kit.SettingsRow{'Accessible Mode', 'Keep collapsed controls expanded and reachable from the keyboard, for screen readers.', p.toggle('set-a11y',
				p.accessible, 0, mut cx)},
			kit.SettingsRow{'When Closing With No Tabs', 'What closing the active item does when no tab is open.', kit.select_box('set-closing', [
				'Platform Default',
				'Close Window',
				'Keep Window Open',
			], p.closing).on_change(cx.listener(fn (mut p SettingsDemo, i &int, mut _ core.ViewCx[SettingsDemo]) {
				p.closing = *i
			}))},
			kit.SettingsRow{'On Last Window Closed', 'What happens when the last window is closed.', kit.select_box('set-quit', [
				'Platform Default',
				'Quit App',
			], p.quit).on_change(cx.listener(fn (mut p SettingsDemo, i &int, mut _ core.ViewCx[SettingsDemo]) {
				p.quit = *i
			}))},
		]},
		kit.SettingsGroup{'Security', [
			kit.SettingsRow{'Trust All Projects', 'Run project tasks and language servers without asking first.', p.toggle('set-trust',
				p.trust, 1, mut cx)},
		]},
	]}
	appearance := kit.SettingsPage{'Appearance', [
		kit.SettingsGroup{'Theme', [
			kit.SettingsRow{'Theme Mode', 'Follow the system, or always use the light or the dark theme.', kit.segmented('set-mode', [
				'Light',
				'Dark',
				'System',
			], p.mode).on_change(cx.listener(fn (mut p SettingsDemo, i &int, mut _ core.ViewCx[SettingsDemo]) {
				p.mode = *i
			}))},
		]},
	]}
	editor := kit.SettingsPage{'Editor', [
		kit.SettingsGroup{'Cursor', [
			kit.SettingsRow{'Cursor Blink', 'Blink the text cursor while the editor has focus.', p.toggle('set-blink',
				p.blink, 2, mut cx)},
		]},
	]}
	mut pages := [general, appearance, editor]
	for name in ['Keymap', 'Languages & Tools', 'Window & Layout', 'Terminal'] {
		pages << kit.SettingsPage{name, []}
	}
	t := kit.theme(cx)
	mut header := []core.Element{}
	for i, s in ['User', 'Project'] {
		header << kit.toggle_button('set-scope-${i}', i == p.scope).label(s).size(.sm).on_toggle(core.listener_with[SettingsDemo, int, bool](cx.entity,
			i, fn (mut p SettingsDemo, i int, _ &bool, mut _ core.ViewCx[SettingsDemo]) {
			p.scope = i
		}))
	}
	header << ui.div().grow()
	json := '{\n  "accessible_mode": ${p.accessible},\n  "trust_all_projects": ${p.trust},\n  "cursor_blink": ${p.blink},\n  "theme_mode": "${[
		'light',
		'dark',
		'system',
	][p.mode]}"\n}'
	header << kit.button('set-json').label('Edit in settings.json').outline().size(.sm).on_click(core.handler_with(json, fn (json string, _ &core.ClickEvent, mut cx core.Cx) {
		kit.open_modal(mut cx, kit.ModalOptions{
			title: 'settings.json'
			body:  fn [json] () core.Element {
				return ui.text(json).family(kit.mono_face)
			}
		})
	}))
	view := kit.settings('settings', pages, p.current).search(cx.child_view(p.search), p.query).header(header).hint('ctrl-shift-e',
		'Focus Navbar').on_select(cx.listener(fn (mut p SettingsDemo, i &int, mut _ core.ViewCx[SettingsDemo]) {
		p.current = *i
	}))
	return page('Settings',
		'Preferences in pages: a searchable navigation, then rows of a title, a description and a control.', [
		demo('Window', ui.div().h(560).border(1).border_color(t.p.line_strong).child(view),
			"kit.settings('settings', pages, current).search(field, query).on_select(...)"),
	])
}

// toggle is a switch that flips one of the demo's booleans (k picks which).
fn (mut p SettingsDemo) toggle(id string, on bool, k int, mut cx core.ViewCx[SettingsDemo]) core.Element {
	return kit.switch(id, on).on_change(core.listener_with[SettingsDemo, int, bool](cx.entity, k, fn (mut p SettingsDemo, k int, v &bool, mut _ core.ViewCx[SettingsDemo]) {
		match k {
			0 { p.accessible = *v }
			1 { p.trust = *v }
			else { p.blink = *v }
		}
	}))
}
