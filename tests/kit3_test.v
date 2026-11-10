// Settings, notifications, auto-hiding scroll thumbs, match highlights and tooltips.
module main

import quadra.core
import quadra.gpu
import quadra.kit
import quadra.kit.input
import quadra.testing
import quadra.ui

struct Prefs {
mut:
	page   int
	query  string
	cards  bool
	scroll &ui.ScrollState = &ui.ScrollState{}
}

@[markused]
pub fn (mut p Prefs) render(mut cx core.ViewCx[Prefs]) core.Element {
	row := fn (title string) kit.SettingsRow {
		return kit.SettingsRow{title, 'About ${title}', ui.div().w(20).h(20)}
	}
	pages := [
		kit.SettingsPage{'General', [
			kit.SettingsGroup{
				title: 'General Settings'
				rows:  [row('Autosave'), row('Restore Session')]
			},
		], '', ''},
		kit.SettingsPage{'Editor', [
			kit.SettingsGroup{
				title: 'Cursor'
				rows:  [row('Cursor Blink')]
			},
			kit.SettingsGroup{
				title: 'Font'
				rows:  [row('Font Size')]
			},
		], '', ''},
	]
	mut settings := kit.settings('set', pages, p.page)
	if p.cards {
		settings.cards()
	}
	settings.search(ui.div(), p.query).on_select(cx.listener(fn (mut p Prefs, i &int, mut _ core.ViewCx[Prefs]) {
		p.page = *i
	}))
	tall := ui.div().h(1000).shrink_0()
	return ui.div().flex_row().size_full().child(ui.div().w(600).h_full().child(settings)).child(ui.div().w(200).h(100).flex_col().overflow_y_scroll().scroll_with(p.scroll).child(tall))
}

fn test_settings_pages_and_search() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Prefs{}, width: 800, height: 400)!
	assert h.has_text('Autosave') && h.has_text('General Settings')
	assert h.find('set-group-1-0') == none
	h.click('set-expand-1')! // the chevron shows the page's groups, without picking it
	assert h.find('set-group-1-0') != none && !h.has_text('Cursor Blink')
	h.click('set-expand-1')!
	assert h.find('set-group-1-0') == none
	h.click('set-expand-1')!
	h.click('set-group-1-0')! // a group picks its page
	assert h.has_text('Cursor Blink')
	h.click('set-page-0')!
	h.click('set-page-1')!
	assert h.has_text('Cursor Blink') && !h.has_text('Autosave')
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	mut p := cx.peek(v.model())
	p.page, p.query = 0, 'restore'
	cx.touch(v.model())
	h.frame()
	assert h.has_text('Restore Session') && !h.has_text('Autosave')
	assert h.find('set-page-1') == none // a page without matches leaves the navigation
}

fn test_settings_cards_navigation() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Prefs{ cards: true },
		width:  800
		height: 400
	)!
	assert h.has_text('Autosave') && h.has_text('General Settings')
	assert h.find('set-expand-0') == none // one group: no chevron
	h.click('set-page-1')! // picks the page and opens its groups
	assert h.has_text('Cursor Blink') && h.find('set-group-1-1') != none
	h.click('set-expand-1')! // the chevron alone closes them
	assert h.find('set-group-1-0') == none && h.has_text('Cursor Blink')
	h.click('set-expand-1')!
	h.click('set-group-1-1')!
	assert h.has_text('Font Size')
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	mut p := cx.peek(v.model())
	p.page, p.query = 0, 'font size'
	cx.touch(v.model())
	h.frame()
	// the chosen page has no match: the first page that has one shows, the choice stays
	assert h.find('set-page-0') == none && h.has_text('Font Size') && !h.has_text('Autosave')
	assert p.page == 0
}

// thumb finds the scroll thumb of the scroll box (x 600..800): a 6 px wide pill; it
// returns its opacity and its top.
fn thumb_at(h &testing.Harness) ?(f32, f32) {
	for inst in h.scene().instances {
		if inst.kind == u32(gpu.Kind.quad) && inst.bounds[2] == 6 && inst.bounds[0] > 600 {
			return inst.color[3], inst.bounds[1]
		}
	}
	return none
}

fn thumb(h &testing.Harness) ?f32 {
	a, _ := thumb_at(h)?
	return a
}

fn test_scroll_thumbs_drag_and_show_when_moved() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Prefs{}, width: 800, height: 400)!
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	mut p := cx.peek(v.model())
	p.scroll.y = 300 // the app scrolls: the thumb shows without pointer or wheel
	cx.win.invalidate_prepaint()
	h.frame()
	_, top := thumb_at(h) or { panic('a moved container shows its thumb') }
	h.drag(793, top + 5, 793, top + 35, true)
	_, moved := thumb_at(h) or { panic('the dragged thumb stays shown') }
	assert moved > top + 25 && p.scroll.y > 300 // the content followed the pointer
}

fn test_scroll_thumbs_show_on_activity_and_fade() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	h.open(Prefs{}, width: 800, height: 400)!
	assert thumb(h) == none // hidden until the pointer or the wheel is there
	h.scroll_at(700, 50, -40)
	full := thumb(h) or { panic('no thumb after scrolling') }
	h.advance(1200)
	fading := thumb(h) or { panic('the thumb should be fading') }
	assert fading < full
	h.advance(400)
	assert thumb(h) == none
	h.move_to(700, 60)
	assert thumb(h) != none // hovering brings it back
}

fn test_notifications_stack_until_closed() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	h.open(Prefs{}, width: 800, height: 400)!
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	id := kit.notify(mut cx, .info, 'Indexed', 'The project is ready')
	h.advance(20000)
	card := h.find('notice-${id}') or { panic('no notification') }
	assert card.width == 448 && card.right() == 800 - 12 // bottom right, 448 wide
	h.click('notice-close-${id}')!
	assert h.find('notice-${id}') == none
	brief := kit.notify(mut cx, .success, 'Saved', '', timeout_ms: 5000)
	h.advance(100)
	assert h.find('notice-${brief}') != none
	h.advance(5000)
	assert h.find('notice-${brief}') == none // closed by its timeout
	mut ran := &Ran{}
	asked := kit.notify(mut cx, .warning, 'Update ready', 'Restart to apply it.',
		actions: [
			kit.NoticeAction{
				label:   'Restart'
				primary: true
				run:     fn [mut ran] (mut _ core.Cx) {
					ran.n++
				}
			},
		]
	)
	h.frame()
	h.click('notice-${asked}-0')!
	assert ran.n == 1 && h.find('notice-${asked}') == none // the action ran and closed it
}

@[heap]
struct Ran {
mut:
	n int
}

fn test_match_marks_and_key_labels() {
	hits := input.fuzzy_hits('gd', 'Go to Definition') or { panic('no match') }
	assert hits == [0, 6]
	spans := input.marked('Go to Definition', hits, kit.dark().p.accent_text)
	assert spans.map(it.text) == ['G', 'o to ', 'D', 'efinition']
	assert kit.key_label('Ctrl-Shift-F12') == 'Ctrl-Shift-F12'
	assert kit.key_label('ctrl-k ctrl-f') == 'Ctrl-K Ctrl-F'
	mut t := kit.dark()
	assert kit.key_text('shift-ctrl-c', t) == 'Ctrl-Shift-C'
	t.ctl.plus_keys = true
	assert kit.key_text('shift-ctrl-c', t) == 'Ctrl+Shift+C'
	assert kit.key_text('win-shift-alt-pageup ctrl-k', t) == 'Alt+Shift+Win+Page Up Ctrl+K'
}

fn test_theme_sizes_and_split_roles() {
	mut t := kit.dark()
	assert t.height(.md) == 28 && t.ui_text(.md) == 14 && t.ui_text(.sm) == 12
	t.text_xs, t.text_sm, t.text_md, t.text_lg = 12, 14, 16, 18
	t.text_step, t.ui_scale, t.h_md = 1, 1.125, 32
	assert t.ui_text(.md) == 14 && t.ui_text(.lg) == 16 && t.ui_text(.sm) == 12
	assert t.height(.md) == 36 && t.sp(8) == 9
	pro := kit.dark()
	assert pro.p.tab_fg == pro.p.text_2 && pro.p.error_hover.a > 0 && pro.p.list_even.a == 0
	night :=
		kit.theme_from_json('{"mode": "dark", "palette": {"text_2": "#808080", "tab_active_fg": "#FFFFFF"}}')!
	assert night.p.tab_fg.hex() == '#808080ff' && night.p.tab_active_fg.hex() == '#ffffffff' // derived roles follow the file
	hc := kit.high_contrast()
	assert hc.p.tab_fg == hc.p.text_2 && hc.p.text_on_status == hc.p.text_on_accent
}

// theme_error is the error a theme file gives, or '' when it loads.
fn theme_error(text string) string {
	kit.theme_from_json(text) or { return err.msg() }
	return ''
}

fn test_themes_load_from_json() {
	pro := kit.dark()
	assert kit.theme_from_json(pro.to_json())! == pro // the file format round-trips
	assert pro.name == 'One Dark Pro' && pro.p.canvas.hex() == '#23272eff'
	assert pro.code.terminal.ansi[1].hex() == '#e05561ff' && pro.series.len == 8
	night :=
		kit.theme_from_json('{"name": "Night", "mode": "dark", "palette": {"canvas": "#000000"}}')!
	assert night.name == 'Night' && night.p.canvas.hex() == '#000000ff'
	assert night.p.panel == pro.p.panel && night.colors.background == night.p.canvas // the rest is One Dark Pro
	assert kit.theme_from_json('{"mode": "light"}')!.p.canvas == kit.light().p.canvas
	assert night.p.input_bg == night.p.canvas && !pro.ctl.plain_toggles // fields follow the canvas
	plain := kit.theme_from_json('{"mode": "dark", "controls": {"plain_toggles": true}}')!
	assert plain.ctl.plain_toggles && kit.theme_from_json(plain.to_json())! == plain
	assert theme_error('{"mode": "dark", "controls": {"plain": true}}').contains('controls.plain')
	assert theme_error('{"mode": "dark", "palette": {"canvass": "#000000"}}').contains('palette.canvass')
	assert theme_error('{"mode": "dim"}').contains('mode')
	assert theme_error('{"mode": "dark", "editor": {"cursor": "#12"}}').contains('editor.cursor')
	assert theme_error('{"mode": "dark", "terminal": {"ansi": ["#000000"]}}').contains('16 colors')
}
