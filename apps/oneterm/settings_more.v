// The Key Bindings, Security, Network and About pages (specs/22).
module main

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.platform
import quadra.ui
import sessionui

const default_font_label = 'Default (theme)'

// font_value is what a Family row stores: the theme font is ''.
fn font_value(label string) string {
	return if label == default_font_label { '' } else { label }
}

fn font_label(family string) string {
	return if family == '' { default_font_label } else { family }
}

// family_row is a searchable list, because a machine can have hundreds of fonts.
fn (mut w SettingsWin) family_row(mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	if w.font_select.id == 0 {
		o := cx.peek(w.one)
		mut options := [default_font_label]
		options << cx.app.text.font_families()
		w.font_select = sessionui.new_select(mut cx.Cx,
			id:          'set-family'
			options:     options
			value:       font_label(o.cfg.font.family)
			search_hint: 'Search fonts...'
			clearable:   false
			list_h:      160
		)
		cx.peek(w.font_select.model()).changed = core.listener_of[SettingsWin, string](cx.entity, fn (mut w SettingsWin, v &string, mut cx core.ViewCx[SettingsWin]) {
			mut o := cx.peek(w.one)
			o.cfg.font.family = font_value(*v)
			w.changed(mut cx)
		}).any
	}
	return kit.SettingsRow{'Family', 'Use "Default" for the theme\'s monospace font.', ui.div().flex_col().w(256).child(core.view_element(w.font_select.any()))}
}

// log_folder_row: a click anywhere on the field opens the folder picker.
fn (mut w SettingsWin) log_folder_row(mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	w.make_text('log_dir', '', mut cx)
	pick := ui.div().test_id('log-folder-pick').absolute().top(0).right(0).bottom(0).left(0).cursor(.pointer).on_click(cx.listener(fn (mut w SettingsWin, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
		w.pick_log_folder(mut cx)
	}))
	return kit.SettingsRow{'Log Folder', 'Default: <user_home>/.OneTerm/logs.', ui.div().flex_col().w(256).child(core.view_element(w.texts['log_dir'].any())).child(pick)}
}

fn (mut w SettingsWin) pick_log_folder(mut cx core.ViewCx[SettingsWin]) {
	picked := cx.pick_files(platform.FileDialog{
		title:   'Select terminal log folder'
		folders: true
	})
	if picked.len == 0 {
		return
	}
	mut o := cx.peek(w.one)
	o.set_text('log_dir', picked[0])
	input.set_value(mut cx.Cx, w.texts['log_dir'], picked[0])
	w.changed(mut cx)
}

fn (mut w SettingsWin) key_groups(t &kit.Theme, mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	mut out := []kit.SettingsGroup{}
	for g in key_groups {
		mut rows := []kit.SettingsRow{}
		for b in bindables.filter(it.group == g) {
			rows << w.key_row(b, t, mut cx)
		}
		out << group(g, rows)
	}
	return out
}

fn (mut w SettingsWin) key_row(b Bindable, t &kit.Theme, mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	o := cx.peek(w.one)
	mut desc := show_default(o.ui.key_bindings, b.id)
	if n := w.notices[b.id] {
		desc = if desc == '' { n } else { '${desc}. ${n}' }
	}
	if w.capturing == b.id {
		prompt := if w.rejection != '' {
			'${w.rejection} - press another key (Esc to cancel)'
		} else {
			'Press keys... (Esc to cancel)'
		}
		box := ui.div().id('kbd-capture').test_id('kbd-capture').track_focus(w.capture_focus).on_key_down(cx.listener(fn (mut w SettingsWin, e &core.KeyDownEvent, mut cx core.ViewCx[SettingsWin]) {
			w.captured(e, mut cx)
			cx.stop_propagation()
		})).on_action(cx.listener(fn (mut w SettingsWin, _ &kit.Dismiss, mut cx core.ViewCx[SettingsWin]) {
			w.capturing, w.rejection = '', ''
			cx.notify()
		})).w(260).h(28).flex_row().items_center().px(8).rounded(t.radius_sm).border(1).border_color(t.p.border_focused).child(ui.text(prompt))
		return kit.SettingsRow{b.label, desc, box}
	}
	current := effective_key(o.ui.key_bindings, b.id)
	chip := if current != '' {
		core.Element(kit.kbd(current))
	} else {
		core.Element(ui.text('—').color(t.p.text_2))
	}
	edit := kit.button('edit-${b.id}').label('Edit').ghost().size(.sm).on_click(core.listener_with[SettingsWin, string, core.ClickEvent](cx.entity,
		b.id, fn (mut w SettingsWin, id string, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
		w.capturing, w.rejection = id, ''
		w.notices = map[string]string{}
		w.capture_focus.focus(mut cx.Cx)
		cx.notify()
	}))
	reset := kit.button('reset-${b.id}').label('Reset').ghost().size(.sm).on_click(core.listener_with[SettingsWin, string, core.ClickEvent](cx.entity,
		b.id, fn (mut w SettingsWin, id string, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
		w.reset_binding(id, mut cx)
	}))
	return kit.SettingsRow{b.label, desc, ui.div().flex_row().items_center().gap(4).child(chip).child(edit).child(reset)}
}

// captured is the next key while a row is being rebound: Esc cancels, a key held by another
// action is refused and says which.
fn (mut w SettingsWin) captured(e &core.KeyDownEvent, mut cx core.ViewCx[SettingsWin]) {
	if e.key == .escape {
		w.capturing, w.rejection = '', ''
		cx.notify()
		return
	}
	keys := core.Keystroke{e.key, e.mods}.str()
	id := w.capturing
	o := cx.peek(w.one)
	if taken := conflicting(o.ui.key_bindings, id, keys) {
		w.rejection = '${keys} is used by ${taken}'
		cx.notify()
		return
	}
	w.capturing, w.rejection = '', ''
	w.set_binding(id, keys, mut cx)
}

// reset_binding restores the default. When a user override holds that key the action stays
// unbound, and the row says so: otherwise Reset reads as a button that does nothing.
fn (mut w SettingsWin) reset_binding(id string, mut cx core.ViewCx[SettingsWin]) {
	w.notices = map[string]string{}
	w.capturing, w.rejection = '', ''
	w.set_binding(id, key_default(id), mut cx)
	o := cx.peek(w.one)
	if winner := displacing_action(o.ui.key_bindings, id) {
		w.notices[id] = 'Still unbound: ${winner} holds this key. Rebind either one to free it.'
		cx.notify()
	}
}

// set_binding stores one binding; a keystroke that does not parse is refused.
fn (mut w SettingsWin) set_binding(id string, keys string, mut cx core.ViewCx[SettingsWin]) {
	mut o := cx.peek(w.one)
	old := o.ui.key_bindings.clone()
	o.ui.key_bindings[id] = keys
	o.ui.key_bindings = without_defaults(resolve_default_collisions(o.ui.key_bindings))
	cx.app.unbind[Do]()
	bind_keys(mut cx.app, o.ui.key_bindings) or {
		o.ui.key_bindings = old.clone()
		cx.app.unbind[Do]()
		bind_keys(mut cx.app, old) or {}
		kit.notify(mut cx.Cx, .error, 'Not a key binding', keys)
		return
	}
	w.notices.delete(id)
	o.apply_and_persist(mut cx.Cx)
	cx.notify()
}
