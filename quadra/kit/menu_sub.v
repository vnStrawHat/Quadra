// Menus, part two (specs/15): submenus, labelled separators and the height cap.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

const menu_cap_h = f32(450) // a menu scrolls past this, or half the window

// submenu is a row that opens `items`; `disabled` keeps its shape in the menu.
pub fn submenu(label string, items []MenuItem) MenuItem {
	return MenuItem{
		label: label
		items: items
	}
}

// menu_heading is a section header row.
pub fn menu_heading(label string) MenuItem {
	return MenuItem{
		label:   label
		heading: true
	}
}

// labelled_separator is a rule with a label in the middle.
pub fn labelled_separator(label string) MenuItem {
	return MenuItem{
		label:     label
		separator: true
	}
}

fn separator_row(item MenuItem, t &Theme) core.Element {
	if item.label == '' && t.ctl.roomy_menus {
		return ui.div().h(2).my(t.sp(2)).bg(t.p.line)
	}
	if item.label == '' {
		return ui.div().h(1).my(6).bg(t.p.line)
	}
	return ui.div().flex_row().items_center().gap(6).h(20).mx(10).child(separator_rule(t)).child(ui.text(item.label).size(t.ui_text(.sm)).color(t.p.text_2)).child(separator_rule(t))
}

fn separator_rule(t &Theme) &ui.Div {
	return ui.div().h(1).grow().bg(t.p.line)
}

fn item_height(item MenuItem, t &Theme, lk MenuLook) f32 {
	if item.separator {
		return if item.label != '' {
			f32(20)
		} else if lk.roomy {
			2 + t.sp(4) + lk.gap
		} else {
			13
		}
	}
	if item.heading && !lk.roomy {
		return 20
	}
	if lk.row_h > 0 && !item.heading {
		return lk.row_h + lk.gap
	}
	return t.ui_text(.md) * t.line_height + t.sp(8) + lk.gap // list rows and roomy headings
}

// cap_height makes the menu scroll when its rows pass the cap (half the window, at most
// 450; a select's list 320).
fn (mut m Menu) cap_height(mut cx core.LayoutCx, t &Theme) {
	lk := menu_look(t, m.list)
	mut total := f32(8)
	for it in m.items {
		total += item_height(it, t, lk)
	}
	half := if cx.win != unsafe { nil } { cx.win.size().height / 2 } else { menu_cap_h }
	cap := prim.min(half, if m.list { t.sp(320) } else { menu_cap_h })
	if total > cap {
		m.div.id('menu-list').test_id('menu-list').max_h(cap).overflow_y_scroll()
	}
}

// SubRow names a row for its handlers.
struct SubRow {
	menu  &Menu = unsafe { nil }
	index int
}

fn click_sub_row(s SubRow, _ &core.ClickEvent, mut cx core.Cx) {
	s.menu.open_sub(s.index, false, mut cx)
}

fn hover_sub_row(s SubRow, _ &core.MouseMoveEvent, mut cx core.Cx) {
	m := s.menu
	if m.state == unsafe { nil } {
		return
	}
	want := if m.items[s.index].items.len > 0 && !m.items[s.index].disabled { s.index } else { -1 }
	if m.state.sub != want {
		m.open_sub(want, false, mut cx)
	}
}

fn (m &Menu) open_sub(i int, by_key bool, mut cx core.Cx) {
	if m.state == unsafe { nil } {
		return
	}
	if i >= 0 && m.items[i].disabled {
		return
	}
	mut st := m.state
	st.sub, st.by_key = i, by_key
	cx.win.invalidate()
}

// close_self closes a submenu and gives the keyboard back to its parent.
fn (m &Menu) close_self(mut cx core.Cx) {
	if m.parent == unsafe { nil } {
		return
	}
	mut st := m.parent.state
	st.sub = -1
	cx.win.set_focus(m.parent.div.focus_id, true)
	cx.win.invalidate()
}

fn (mut m Menu) prepaint_sub(bounds prim.Rect, mut cx core.PrepaintCx) {
	i := m.state.sub
	if i < 0 || i >= m.items.len || m.items[i].items.len == 0 {
		return
	}
	anchor := m.anchors[i] or { return }
	child := &Menu{
		items:  m.items[i].items
		key:    '${m.key}-sub${i}'
		parent: unsafe { m }
		grab:   m.state.by_key
		width:  m.width
	}
	// One's submenus overlap their row by 2; roomy ones sit 8 into the parent and 4 up, so
	// their first row lines up with the parent's (the row is inset 4).
	mut a := ui.anchored(child).layer(3500 + m.key.len).placement(.right_start)
	if theme_of(cx.app).ctl.roomy_menus {
		a.at(prim.rect(anchor.x, anchor.y - 4, anchor.width, anchor.height)).gap(-4)
	} else {
		a.at(anchor).gap(-2)
	}
	a.prepaint(bounds, mut cx)
}

// SubAnchor records a submenu row's bounds, where the submenu opens next to it.
@[heap]
struct SubAnchor {
mut:
	el    core.Element
	menu  &Menu = unsafe { nil }
	index int
}

fn (mut a SubAnchor) request_layout(mut cx core.LayoutCx) layout.NodeId {
	return a.el.request_layout(mut cx)
}

fn (mut a SubAnchor) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	mut m := a.menu
	m.anchors[a.index] = bounds
	a.el.prepaint(bounds, mut cx)
}

fn (mut a SubAnchor) paint(bounds prim.Rect, mut cx core.PaintCx) {
	a.el.paint(bounds, mut cx)
}
