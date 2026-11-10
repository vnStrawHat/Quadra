// The settings window (specs/22): every change applies and is saved at once. See settings_*.v.
module main

import json
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.platform
import quadra.ui
import sessionui

pub struct SettingsWin {
pub mut:
	one           core.Model[OneTerm]
	page          int
	numbers       map[string]core.View[input.NumberInput]
	texts         map[string]core.View[input.TextInput]
	capturing     string // the binding being captured
	capture_focus core.FocusHandle
	rejection     string
	notices       map[string]string
	font_select   core.View[sessionui.SearchSelect]
	search        core.View[input.TextInput]
	query         string
}

fn (mut o OneTerm) open_settings(mut cx core.ViewCx[OneTerm]) {
	o.themes = theme_list(o.dir)
	_ := cx.app.open_window(platform.WindowOptions{
		title:           'OneTerm Settings'
		custom_titlebar: true
		width:           1000
		height:          700
		min_width:       480
		min_height:      400
	}, SettingsWin{
		one: cx.self_model()
	}) or {
		kit.notify(mut cx.Cx, .error, 'Settings', err.msg())
		return
	}
}

fn (mut w SettingsWin) changed(mut cx core.ViewCx[SettingsWin]) {
	mut o := cx.peek(w.one)
	o.apply_and_persist(mut cx.Cx)
	if w.page >= 6 { // Network and About keep their fields in the updater's own file
		o.save_update()
	}
	cx.touch(w.one)
	cx.notify()
}

fn (w &SettingsWin) switch_row(key string, title string, desc string, mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	mut o := cx.peek(w.one)
	r := o.bool_ref(key)
	on := if r != unsafe { nil } { *r } else { false }
	return kit.SettingsRow{title, desc, kit.switch('set-${key}', on).on_change(core.listener_with[SettingsWin, string, bool](cx.entity,
		key, fn (mut w SettingsWin, key string, v &bool, mut cx core.ViewCx[SettingsWin]) {
		mut o := cx.peek(w.one)
		mut r := o.bool_ref(key)
		if r != unsafe { nil } {
			unsafe {
				*r = *v
			}
		}
		w.changed(mut cx)
	}))}
}

// ChoiceAt is a choice row's handler data: where it stores and the values it offers.
struct ChoiceAt {
	key    string
	values []string
}

fn (w &SettingsWin) choice_row(key string, title string, desc string, labels []string, values []string, mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	mut o := cx.peek(w.one)
	r := o.text_ref(key)
	at := values.index(*r)
	return kit.SettingsRow{title, desc, kit.select_box('set-${key}', labels, if at < 0 {
		0
	} else {
		at
	}).compact().size(.lg).on_change(core.listener_with[SettingsWin, ChoiceAt, int](cx.entity, ChoiceAt{key, values}, fn (mut w SettingsWin, c ChoiceAt, i &int, mut cx core.ViewCx[SettingsWin]) {
		mut o := cx.peek(w.one)
		mut r := o.text_ref(c.key)
		unsafe {
			*r = c.values[*i]
		}
		if c.key == 'kind' { // the program belonged to the previous kind
			o.cfg.shell.program = ''
			w.texts.delete('program')
		}
		w.changed(mut cx)
	}))}
}

fn (mut w SettingsWin) number_row(key string, title string, desc string, mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	w.make_number(key, mut cx)
	return kit.SettingsRow{title, desc, core.view_element(w.numbers[key].any())}
}

fn (mut w SettingsWin) text_row(key string, title string, desc string, placeholder string, mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	w.make_text(key, placeholder, mut cx)
	return kit.SettingsRow{title, desc, ui.div().flex_col().w(256).child(core.view_element(w.texts[key].any()))}
}

// fixed_row shows a value this release does not let the user change.
fn (w &SettingsWin) fixed_row(title string, desc string, value string, mut cx core.ViewCx[SettingsWin]) kit.SettingsRow {
	t := kit.theme(cx)
	return kit.SettingsRow{title, desc, ui.text(value).family(t.mono_family).color(t.p.text_3)}
}

fn (mut w SettingsWin) make_number(key string, mut cx core.ViewCx[SettingsWin]) {
	if key in w.numbers {
		return
	}
	mut o := cx.peek(w.one)
	r := number_ranges[key] or { [f64(0), 1000000, 1, 0] }
	n := input.new_number_input(mut cx.Cx,
		id:       'set-${key}'
		value:    o.number_of(key)
		min:      r[0]
		max:      r[1]
		step:     r[2]
		decimals: int(r[3])
		stepper:  true
	)
	input.on_number_change(mut cx.Cx, n, core.listener_with[SettingsWin, string, f64](cx.entity,
		key, fn (mut w SettingsWin, key string, v &f64, mut cx core.ViewCx[SettingsWin]) {
		mut o := cx.peek(w.one)
		o.set_number(key, *v)
		w.changed(mut cx)
	}))
	w.numbers[key] = n
}

fn (mut w SettingsWin) make_text(key string, placeholder string, mut cx core.ViewCx[SettingsWin]) {
	if key in w.texts {
		return
	}
	mut o := cx.peek(w.one)
	f := input.new_input(mut cx.Cx,
		id:          'set-${key}'
		value:       o.text_of(key)
		placeholder: placeholder
		height:      32
	)
	input.on_change(mut cx.Cx, f, core.listener_with[SettingsWin, string, string](cx.entity, key, fn (mut w SettingsWin, key string, v &string, mut cx core.ViewCx[SettingsWin]) {
		mut o := cx.peek(w.one)
		o.set_text(key, *v)
		w.changed(mut cx)
	}))
	w.texts[key] = f
}

// same_json is here because V generates broken C for json.encode inside a match expression.
fn same_json[T](a T, b T) bool {
	left := json.encode(a)
	right := json.encode(b)
	return left == right
}

// page_dirty: does the page differ from its defaults (Reset All shows only then)?
fn (w &SettingsWin) page_dirty(i int, o &OneTerm) bool {
	match i {
		0 {
			return o.ui.theme_name != '' || o.ui.ui_font_size != 16 || o.ui.show_fps
				|| o.ui.vertical_tabs.enabled || !same_json(o.cfg.shell, ShellConfig{})
		}
		1 {
			return without_defaults(o.ui.key_bindings).len > 0
		}
		2 {
			return !same_json(terminal_page_slice(o.cfg), terminal_page_slice(TerminalConfig{}))
		}
		3 {
			return !same_json(o.cfg.completion, CompletionConfig{})
		}
		4 {
			return !same_json(ssh_page_slice(o.cfg), ssh_page_slice(TerminalConfig{}))
		}
		5 {
			return o.ui.lock_after_idle_minutes != 0
		}
		6, 7 {
			return !same_json(o.update, UpdateConfig{}) // one block: Reset All restores both pages
		}
		else {
			return false
		}
	}
}

struct TermSlice {
	font   FontConfig
	cursor CursorConfig
	layout LayoutConfig
	scroll ScrollConfig
	bell   BellConfig
	mouse  MouseConfig
	secu   SecurityConfig
	log    LoggingConfig
}

fn terminal_page_slice(c TerminalConfig) TermSlice {
	mut log := c.logging
	log.directory = '' // a default of its own: not a change
	return TermSlice{c.font, c.cursor, c.layout, c.scroll, c.bell, c.mouse, c.security, log}
}

struct SshSlice {
	ssh  SshSettingsConfig
	sftp SftpConfig
}

fn ssh_page_slice(c TerminalConfig) SshSlice {
	return SshSlice{c.ssh, c.sftp}
}

fn (mut w SettingsWin) reset_page(mut cx core.ViewCx[SettingsWin]) {
	mut o := cx.peek(w.one)
	match w.page {
		0 {
			o.ui.theme_name, o.ui.ui_font_size, o.ui.show_fps = '', 16, false
			o.ui.vertical_tabs.enabled = false
			o.cfg.shell = ShellConfig{}
		}
		1 {
			o.ui.key_bindings = map[string]string{}
			w.notices = map[string]string{}
			cx.app.unbind[Do]()
			bind_keys(mut cx.app, o.ui.key_bindings) or {}
		}
		2 {
			d := TerminalConfig{}
			dir := o.cfg.logging.directory
			o.cfg.font, o.cfg.cursor, o.cfg.layout = d.font, d.cursor, d.layout
			o.cfg.scroll, o.cfg.bell, o.cfg.mouse = d.scroll, d.bell, d.mouse
			o.cfg.security, o.cfg.logging = d.security, d.logging
			o.cfg.logging.directory = dir
		}
		3 {
			o.cfg.completion = CompletionConfig{}
		}
		4 {
			o.cfg.ssh, o.cfg.sftp = SshSettingsConfig{}, SftpConfig{}
		}
		5 {
			o.ui.lock_after_idle_minutes = 0
		}
		6, 7 {
			o.update = UpdateConfig{}
		}
		else {}
	}

	w.numbers = map[string]core.View[input.NumberInput]{}
	w.texts = map[string]core.View[input.TextInput]{}
	w.font_select = core.View[sessionui.SearchSelect]{}
	w.changed(mut cx)
}

@[markused]
pub fn (mut w SettingsWin) render(mut cx core.ViewCx[SettingsWin]) core.Element {
	o := cx.peek(w.one)
	t := kit.theme(cx)
	if o.vault.is_screen_locked() {
		return ui.div().flex_col().size_full().center().bg(t.p.canvas).child(kit.icon(kit.icon_lock).size(32).color(t.p.text_2)).child(ui.text('OneTerm is locked'))
	}
	if w.capture_focus.id == 0 {
		w.capture_focus = cx.focus_handle()
		core.observe[SettingsWin, OneTerm](mut cx, w.one, fn (mut _ SettingsWin, _ core.Model[OneTerm], mut cx core.ViewCx[SettingsWin]) {
			cx.notify()
		})
	}
	if w.search.id == 0 {
		w.search = input.new_input(mut cx.Cx,
			id:          'settings-search'
			placeholder: 'Search...'
			prefix:      kit.icon_search
			height:      32
		)
		input.on_change(mut cx.Cx, w.search, cx.listener(fn (mut w SettingsWin, v &string, mut _ core.ViewCx[SettingsWin]) {
			w.query = *v
		}))
	}
	pages := w.pages(t, mut cx)
	mut header := []core.Element{}
	if w.page_dirty(w.page, o) { // as in the original: an undo icon at the header's right end
		header << kit.icon_button('reset-all', icon_undo_2).tooltip('Reset All').on_click(cx.listener(fn (mut w SettingsWin, _ &core.ClickEvent, mut cx core.ViewCx[SettingsWin]) {
			w.reset_page(mut cx)
		}))
	}
	view := kit.settings('settings', pages, w.page).cards().search(cx.child_view(w.search), w.query).header(header).on_select(cx.listener(fn (mut w SettingsWin, i &int, mut _ core.ViewCx[SettingsWin]) {
		w.page = *i
	}))
	return ui.div().flex_col().size_full().bg(t.p.canvas).child(kit.titlebar('Settings').title_size(16)).child(ui.div().grow().min_h(0).child(view))
}
