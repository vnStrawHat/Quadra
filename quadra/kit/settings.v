// Settings: a preferences page (specs/15, Settings). A navigation column of pages on the
// left; the chosen page on the right, its settings grouped under small mono headers, one
// row per setting: the title and a description on the left, the control on the right.
// The app owns the controls, the search text and the chosen page; the component keeps
// which pages are expanded in the column and where the page is scrolled.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

pub struct SettingsRow {
pub:
	title       string
	description string
	control     core.Element
}

pub struct SettingsGroup {
pub:
	title string
	rows  []SettingsRow
}

pub struct SettingsPage {
pub:
	title  string
	groups []SettingsGroup
}

@[heap]
pub struct Settings {
	Lazy
pub mut:
	id      string
	pages   []SettingsPage
	current int
	query   string         // shows only the rows whose title or description contain it
	search  ?core.Element  // the search field above the navigation (an input the app owns)
	header  []core.Element // the row above the page title: scope chips, then actions
	hint    []string       // [keys, text]: a keyboard hint under the navigation
	picked  ?core.AnyListener
mut:
	st    &SettingsState = unsafe { nil }
	heads map[int]&ui.Div // the shown group headers by group index (scroll targets)
	first int = -1 // the first shown group (its header has less space above)
}

// SettingsState outlives the builder: one per settings id, in the SettingsUi global.
@[heap]
pub struct SettingsState {
mut:
	open   map[int]bool // pages whose groups show under them in the column
	scroll ui.ScrollState
	target int = -1 // a group to bring to the top at the next prepaint
}

pub struct SettingsUi {
mut:
	states map[string]&SettingsState
}

fn settings_state(app &core.App, id string) &SettingsState {
	cx := core.Cx{
		app: unsafe { app }
	}
	mut reg := cx.peek_global[SettingsUi]() or { return &SettingsState{} }
	if id !in reg.states {
		reg.states[id] = &SettingsState{}
	}
	return reg.states[id] or { &SettingsState{} }
}

// SettingsNav is what a click in the column needs: the state, the page and the group
// (-1: the page row itself).
struct SettingsNav {
	st     &SettingsState
	picked ?core.AnyListener
	page   int
	group  int
}

fn settings_expand(n SettingsNav, _ &core.ClickEvent, mut cx core.Cx) {
	mut st := n.st
	st.open[n.page] = !st.open[n.page]
	cx.win.invalidate()
	cx.stop_propagation() // the chevron toggles; it does not pick the page
}

fn settings_pick(n SettingsNav, _ &core.ClickEvent, mut cx core.Cx) {
	mut st := n.st
	if n.group >= 0 {
		st.target = n.group
	} else {
		st.scroll.y = 0
	}
	fire(n.picked, n.page, mut cx)
	cx.win.invalidate()
}

pub fn settings(id string, pages []SettingsPage, current int) &Settings {
	return &Settings{
		id:      id
		pages:   pages
		current: current
	}
}

pub fn (self_ &Settings) search(el core.Element, query string) &Settings {
	mut s := unsafe { self_ }
	s.search, s.query = el, query
	return s
}

pub fn (self_ &Settings) header(els []core.Element) &Settings {
	mut s := unsafe { self_ }
	s.header = els
	return s
}

pub fn (self_ &Settings) hint(keys string, text string) &Settings {
	mut s := unsafe { self_ }
	s.hint = [keys, text]
	return s
}

pub fn (self_ &Settings) on_select(l core.Listener[int]) &Settings {
	mut s := unsafe { self_ }
	s.picked = l.any
	return s
}

// matches reports whether a row passes the search: a case-insensitive substring of its
// title or description.
fn (s &Settings) matches(r SettingsRow) bool {
	if s.query == '' {
		return true
	}
	q := s.query.to_lower()
	return r.title.to_lower().contains(q) || r.description.to_lower().contains(q)
}

fn (s &Settings) count(p SettingsPage) int {
	mut n := 0
	for g in p.groups {
		n += g.rows.filter(s.matches(it)).len
	}
	return n
}

// nav is the 226 px page column: the search field, 28 px rows (hover one step, the
// chosen one two) whose chevron shows the page's groups under it, the keyboard hint at
// the bottom over a hairline.
fn (s &Settings) nav(t &Theme) &ui.Div {
	p := t.p
	mut col :=
		ui.div().test_id('${s.id}-nav').flex_col().w(226).h_full().shrink_0().p(10).gap(2).border_r(1).border_color(p.line_strong).role(.tree)
	if el := s.search {
		col.child(ui.div().flex_col().pb(10).child(el))
	}
	mut list := ui.div().flex_col().gap(2).grow().min_h(0).overflow_y_scroll()
	for i, pg in s.pages {
		if s.query != '' && s.count(pg) == 0 {
			continue
		}
		on := i == s.current
		open := s.st.open[i] && pg.groups.len > 0
		chevron := ui.div().test_id('${s.id}-expand-${i}').size(16).center().shrink_0().rounded(t.radius_sm).hover(ui.Patch{
			background: p.press
		}).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, -1}, settings_expand)).child(icon(if open {
			icon_chevron_down
		} else {
			icon_chevron_right
		}).size(12).color(p.text_2))
		mut row := ui.div().test_id('${s.id}-page-${i}').flex_row().items_center().gap(6).h(28).px(6).shrink_0().rounded(t.radius_md).cursor(.pointer).role(.treeitem).label(pg.title).selected(on).expanded(open).text_size(t.font_size(.md)).text_color(p.text).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, -1},
			settings_pick)).child(chevron).child(ui.text(pg.title))
		s.nav_style(mut row, on, t)
		list.child(row)
		if !open {
			continue
		}
		for g, grp in pg.groups {
			if s.query != '' && grp.rows.filter(s.matches(it)).len == 0 {
				continue
			}
			mut sub := ui.div().test_id('${s.id}-group-${i}-${g}').flex_row().items_center().h(26).pl(28).pr(6).shrink_0().rounded(t.radius_md).cursor(.pointer).role(.treeitem).label(grp.title).text_color(p.text_2).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, g},
				settings_pick)).child(ui.text(grp.title))
			s.nav_style(mut sub, false, t)
			list.child(sub)
		}
	}
	col.child(list)
	if s.hint.len == 2 {
		col.child(ui.div().pt(10).border_t(1).border_color(p.line).child(key_hint(s.hint[0],
			s.hint[1])))
	}
	return col
}

fn (s &Settings) nav_style(mut row ui.Div, on bool, t &Theme) {
	if on {
		row.bg(t.p.raised_hover).border(1).border_color(t.p.line)
	} else {
		row.hover(ui.Patch{
			background: t.p.panel_hover
		})
	}
}

// row is one setting: 14 px title over a 12 px muted description that wraps, the control
// right, 16 px above and below, a hairline under.
fn settings_row(r SettingsRow, t &Theme) &ui.Div {
	mut left := ui.div().flex_col().gap(2).grow().min_w(0).child(ui.text(r.title))
	if r.description != '' {
		left.child(ui.text(r.description).size(t.font_size(.sm)).color(t.p.text_2))
	}
	return ui.div().flex_row().items_center().gap(16).py(16).border_b(1).border_color(t.p.line).role(.group).label(r.title).child(left).child(ui.div().shrink_0().child(r.control))
}

fn (mut s Settings) page(t &Theme) &ui.Div {
	p := t.p
	mut col :=
		ui.div().test_id('${s.id}-page').flex_col().grow().min_w(0).h_full().overflow_y_scroll().scroll_with(&s.st.scroll).pl(40).pr(32).py(24)
	if s.header.len > 0 {
		col.child(ui.div().flex_row().items_center().gap(4).children(s.header))
	}
	if s.current < 0 || s.current >= s.pages.len {
		return col
	}
	pg := s.pages[s.current]
	col.child(ui.div().pt(if s.header.len > 0 { 24 } else { 0 }).role(.heading).label(pg.title).child(ui.text(pg.title).size(t.scaled(t.text_xl))))
	mut first := true
	for gi, g in pg.groups {
		rows := g.rows.filter(s.matches(it))
		if rows.len == 0 {
			continue
		}
		head :=
			ui.div().pt(if first { 8 } else { 32 }).pb(6).border_b(1).border_color(p.line).child(ui.text(g.title).family(t.mono_family).size(t.font_size(.sm)).color(p.text_2))
		s.heads[gi] = head
		if first {
			s.first = gi
		}
		col.child(head)
		for r in rows {
			col.child(settings_row(r, t))
		}
		first = false
	}
	return col
}

pub fn (mut s Settings) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

// prepaint brings a group picked in the column to the top of the page, now that the
// layout knows where its header is.
pub fn (mut s Settings) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if s.st.target >= 0 {
		if head := s.heads[s.st.target] {
			above := if s.st.target == s.first { f32(8) } else { 32 }
			s.st.scroll.y = cx.tree.rect(head.layout_node()).y + above - 24 // text at the top padding
		}
		s.st.target = -1
	}
	s.Lazy.prepaint(bounds, mut cx)
}

pub fn (mut s Settings) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	s.st = settings_state(cx.app, s.id)
	s.heads = map[int]&ui.Div{}
	s.first = -1
	return ui.div().test_id(s.id).flex_row().size_full().bg(t.p.canvas).text_size(t.font_size(.md)).text_color(t.p.text).child(s.nav(t)).child(s.page(t))
}
