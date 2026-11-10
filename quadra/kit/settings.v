// Settings: a preferences page (specs/15, Settings). A navigation column of pages on the
// left; the chosen page on the right, one row per setting: the title and a description
// on the left, the control on the right. Two layouts: rules (groups under small mono
// headers, settings_rules.v) and cards (outlined group cards under a page header,
// settings_cards.v). The app owns the controls, the search text and the chosen page; the
// component keeps which pages are expanded in the column and where the page is scrolled.
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
	title       string
	rows        []SettingsRow
	description string // shown under the header when the group has one
}

pub struct SettingsPage {
pub:
	title       string
	groups      []SettingsGroup
	icon        string // a kit.icon_* body before the title in the column (cards)
	description string // under the page title (cards)
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
	header  []core.Element // rules: the row above the page title; cards: the title's right end
	hint    []string       // [keys, text]: a keyboard hint under the navigation
	cards   bool           // the outlined-card layout (cards())
	picked  ?core.AnyListener
mut:
	st    &SettingsState = unsafe { nil }
	heads map[int]&ui.Div // the shown groups by index (scroll targets)
	first int = -1 // the first shown group (its header has less space above)
}

// SettingsState outlives the builder: one per settings id, in the SettingsUi global.
@[heap]
pub struct SettingsState {
mut:
	open   map[int]bool // pages whose groups show under them in the column
	scroll ui.ScrollState
	target int = -1 // a group to bring to the top at the next prepaint
	group  int = -1 // the group picked in the column (cards: it is the chosen row)
	gpage  int = -1 // and its page
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
// (-1: the page row itself); `open` also expands the page (cards).
struct SettingsNav {
	st     &SettingsState
	picked ?core.AnyListener
	page   int
	group  int
	open   bool
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
	st.group, st.gpage = n.group, n.page
	if n.open {
		st.open[n.page] = true
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

// cards switches to the outlined-card layout (specs/15, Settings, cards).
pub fn (self_ &Settings) cards() &Settings {
	mut s := unsafe { self_ }
	s.cards = true
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

// shown is the page on the right: the chosen one, or while a search leaves it empty the
// first page that keeps a row.
fn (s &Settings) shown() int {
	if s.current >= 0 && s.current < s.pages.len
		&& (s.query == '' || s.count(s.pages[s.current]) > 0) {
		return s.current
	}
	for i, pg in s.pages {
		if s.count(pg) > 0 {
			return i
		}
	}
	return s.current
}

pub fn (mut s Settings) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

// prepaint brings a group picked in the column to the top of the page, now that the
// layout knows where it is.
pub fn (mut s Settings) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if s.st.target >= 0 {
		if head := s.heads[s.st.target] {
			y := cx.tree.rect(head.layout_node()).y
			if s.cards {
				s.st.scroll.y = y // the group's own padding stays above it
			} else {
				above := if s.st.target == s.first { f32(8) } else { 32 }
				s.st.scroll.y = y + above - 24 // text at the top padding
			}
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
	root :=
		ui.div().test_id(s.id).flex_row().size_full().bg(t.p.canvas).text_size(t.ui_text(.md)).text_color(t.p.text)
	if s.cards {
		return root.child(s.cards_nav(t)).child(s.cards_page(t))
	}
	return root.child(s.nav(t)).child(s.page(t))
}
