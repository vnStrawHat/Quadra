// The status bar (specs/22): measured elision of the centre, and its indicators.
module main

import quadra.core
import quadra.kit
import quadra.text
import quadra.ui

const min_path_w = f32(80)
const min_branch_w = f32(40)
const bar_chrome = f32(116)
const toggle_chrome = f32(37)
const icon_chrome = f32(16)

// divide_centre splits what the bar has left between the path and the branch.
pub fn divide_centre(window_w f32, icons int, fixed f32, git f32) (f32, f32) {
	chrome := bar_chrome + icon_chrome * f32(icons)
	mut room := window_w - chrome - fixed
	if room < 0 {
		room = 0
	}
	if room - git >= min_path_w {
		return room - git, git
	}
	mut g := room - min_path_w
	if g < min_branch_w {
		g = min_branch_w
	}
	if g > git {
		g = git
	}
	if g > room {
		g = room
	}
	return room - g, g
}

// Measure gives the width of a label (the status bar's text system; tests count runes).
pub interface Measure {
	width(s string) f32
}

struct TextMeasure {
	ts    &text.TextSystem = unsafe { nil }
	style text.TextStyle
}

fn (m TextMeasure) width(s string) f32 {
	mut ts := m.ts
	return ts.layout_line(s, m.style).width
}

// elide_left keeps the tail of a path behind an ellipsis, cutting at a separator; only a single
// component longer than the budget is cut inside itself.
pub fn elide_left(path string, budget f32, measure Measure) string {
	if measure.width(path) <= budget {
		return path
	}
	mut i := 0
	for {
		next := path.index_after('\\', i + 1) or { -1 }
		next2 := path.index_after('/', i + 1) or { -1 }
		mut cut := next
		if cut < 0 || (next2 >= 0 && next2 < cut) {
			cut = next2
		}
		if cut < 0 {
			break
		}
		i = cut
		tail := '…' + path[i..]
		if measure.width(tail) <= budget {
			return tail
		}
	}
	// One component is longer than the budget: cut inside it.
	mut k := path.runes().len
	r := path.runes()
	for k > 1 {
		k--
		s := '…' + r[r.len - k..].string()
		if measure.width(s) <= budget {
			return s
		}
	}
	return '…'
}

// cwd_tooltip: a shortened label shows the whole value, an unshortened one only offers the copy.
pub fn cwd_tooltip(shown string, cwd string) string {
	return if shown != cwd { cwd } else { 'Click to copy' }
}

// elide_right keeps the head (a branch name is known by its start).
pub fn elide_right(s string, budget f32, measure Measure) string {
	if measure.width(s) <= budget {
		return s
	}
	r := s.runes()
	for k := r.len - 1; k > 0; k-- {
		t := r[..k].string() + '…'
		if measure.width(t) <= budget {
			return t
		}
	}
	return '…'
}

// item_el is an indicator: a 12 rem icon in the label's colour, gap 4.
fn item_el(glyph string, label core.Element, t &kit.Theme) &ui.Div {
	return ui.div().flex_row().items_center().gap(t.sp(4)).shrink_0().child(kit.icon(glyph).size(t.sp(12)).color(t.p.text)).child(label)
}

fn (mut o OneTerm) status_bar(t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	size := t.font_size(.xs)
	measure := TextMeasure{cx.app.text, text.TextStyle{
		family: t.font_family
		size:   size
	}}
	cwd := o.active_cwd(mut cx)
	git := o.git.label
	res := o.res.text
	mut fixed := measure.width(o.clock) + measure.width(o.net_text) + measure.width(res)
	if o.stats_on && o.stats_text != '' {
		fixed += measure.width(o.stats_text)
	}
	git_text := git.whole()
	mut icons := 1
	icons += if o.net_text != '' { 1 } else { 0 }
	icons += if res != '' { 1 } else { 0 }
	icons += if cwd != '' { 1 } else { 0 }
	icons += if git.head != '' { 1 } else { 0 }
	width := cx.win.size().width - if o.ui.vertical_tabs.enabled { toggle_chrome } else { f32(0) }
	path_w, git_w := divide_centre(width, icons, fixed, if git.head != '' {
		measure.width(git_text)
	} else {
		f32(0)
	})

	mut left := []core.Element{}
	if o.ui.vertical_tabs.enabled {
		collapsed := o.ui.vertical_tabs.collapsed
		glyph := if collapsed { icon_panel_left_open } else { icon_panel_left_close }
		label := if collapsed { 'Expand Tabs' } else { 'Collapse Tabs' }
		left << kit.icon_button('status-rail', glyph).ghost().size(.xs).tooltip(label).on_click(cx.listener(fn (mut o OneTerm, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
			o.toggle_rail_collapsed(mut cx)
		}))
		left << kit.status_divider()
	}
	left << item_el(kit.icon_clock, ui.text(o.clock), t)

	mut centre := ui.div().flex_row().items_center().gap(t.sp(8)).grow().min_w(0).overflow_hidden()
	if cwd != '' || git.head != '' {
		centre.child(kit.status_divider())
	}
	if cwd != '' {
		shown := elide_left(cwd, path_w, measure)
		crumb := item_el(icon_folder_open, ui.text(shown), t).test_id('status-cwd').cursor(.pointer).on_click(cx.listener(fn (mut o OneTerm, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
			cx.clipboard_write(o.active_cwd(mut cx))
		}))
		centre.child(kit.tooltip(crumb, cwd_tooltip(shown, cwd)))
	}
	if cwd != '' && git.head != '' {
		centre.child(kit.status_divider())
	}
	if git.head != '' {
		centre.child(item_el(icon_git_branch, git.element(git_w, measure, t), t).test_id('status-git'))
	}

	mut right := []core.Element{}
	if o.net_text != '' {
		right << item_el(icon_network, ui.text(o.net_text), t).test_id('status-net')
		right << kit.status_divider()
	}
	if res != '' {
		right << kit.hover_card_with(item_el(icon_cpu, ui.text(res), t).test_id('status-res'), BoxRef{o.res},
			resource_table)
		right << kit.status_divider()
	}
	if o.stats_on && o.stats_text != '' {
		right << ui.text(o.stats_text)
	}
	if !is_restricted() {
		right << kit.icon_button('status-dock', icon_panel_right).ghost().size(.xs).tooltip('Toggle Right Dock').on_click(cx.listener(fn (mut o OneTerm, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
			o.toggle_dock(mut cx)
		}))
	}
	return ui.div().test_id('statusbar').flex_row().items_center().gap(t.sp(8)).px(t.sp(8)).py(t.sp(4)).h(t.statusbar_h).shrink_0().bg(t.p.status_bg).border_t(1).border_color(t.p.line_strong).text_size(size).text_color(t.p.text).children(left).child(centre).children(right)
}
