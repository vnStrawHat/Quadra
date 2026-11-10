// Menus: menu (popup content), menu_button, context_menu and menubar.
// Arrow keys move the highlight, enter activates, escape closes.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

pub struct MenuItem {
pub:
	label     string
	glyph     string
	shortcut  string
	danger    bool
	disabled  bool
	separator bool
	on_select ?fn (mut cx core.Cx)
	action    ?MenuAction // what on_select does, as an object: no closure per item (specs/14)
	checked   bool        // a check mark in the icon column
	items     []MenuItem  // a submenu (menu_sub.v)
	heading   bool        // a section header; never activates
	swatch    prim.Color  // a leading 8 px color square; transparent: none
}

// MenuAction is a menu item's action as an object.
pub interface MenuAction {
	run(mut cx core.Cx)
}

// PickIndex fires change with index: the action of an item that picks an option.
pub struct PickIndex {
pub:
	change ?core.AnyListener
	index  int
}

pub fn (p PickIndex) run(mut cx core.Cx) {
	fire(p.change, p.index, mut cx)
}

pub fn separator() MenuItem {
	return MenuItem{
		separator: true
	}
}

pub struct MenuState {
pub mut:
	index   int = -1
	focused bool
	sub     int = -1 // the open submenu's row (menu_sub.v)
	by_key  bool // the submenu was opened from the keyboard: it takes the focus
}

@[heap]
pub struct Menu {
pub mut:
	items []MenuItem
	width f32  // the least width (0: 200, roomy menus 128): long labels widen the menu instead of wrapping
	list  bool // a select's options (menu_row.v)
mut:
	div     &ui.Div    = unsafe { nil }
	state   &MenuState = unsafe { nil }
	key     string // keeps a submenu's state apart from its parent's
	parent  &Menu = unsafe { nil } // a submenu's owner
	grab    bool  = true           // takes the focus when it opens
	anchors map[int]prim.Rect // submenu rows, for placing the submenu
}

pub fn menu(items []MenuItem) &Menu {
	return &Menu{
		items: items
	}
}

fn run_item(item MenuItem, mut cx core.Cx) {
	if item.disabled || item.separator || item.heading || item.items.len > 0 {
		return
	}
	close_popups(mut cx)
	if f := item.on_select {
		f(mut cx)
	}
	if a := item.action {
		a.run(mut cx)
	}
}

fn click_item(item MenuItem, _ &core.ClickEvent, mut cx core.Cx) {
	run_item(item, mut cx)
}

fn menu_popup(items []MenuItem, _ &Theme) core.Element {
	return menu(items)
}

pub fn (mut m Menu) request_layout(mut cx core.LayoutCx) layout.NodeId {
	t := theme_of(cx.app)
	roomy := t.ctl.roomy_menus || m.list
	mut least := m.width
	if least == 0 {
		least = if roomy { t.sp(128) } else { 200 }
	}
	m.div =
		ui.div().flex_col().py(4).min_w(least).bg(t.p.overlay_bg).shadow_level(1).text_size(t.ui_text(.md)).role(.menu).focus_ring(ui.Patch{}) // the highlighted row shows focus
	if roomy { // a 1 px ring of the text color at 10 % instead of the border
		m.div.max_w(t.sp(500)).gap(menu_look(t, m.list).gap).rounded(t.radius_md).border(1).border_color(t.p.text.alpha(0.1))
	} else {
		m.div.rounded(t.radius_xl).border(1).border_color(t.p.line)
	}
	hi := if m.state != unsafe { nil } { m.state.index } else { -1 }
	icons := m.items.any(it.glyph != '' || (it.checked && !m.list))
	m.cap_height(mut cx, t)
	for i, item in m.items {
		m.div.child(m.row(i, item, t, i == hi, icons))
	}
	return m.div.request_layout(mut cx)
}

fn menu_keys(holder prim.Holder, _ prim.Holder, event prim.Holder, mut cx core.Cx) {
	mut m := prim.held[Menu](holder)
	e := prim.held[core.KeyDownEvent](event)
	n := m.items.len
	mut step := 0
	match e.key {
		.down {
			step = 1
		}
		.up {
			step = -1
		}
		.enter, .space {
			if m.state.index >= 0 && m.state.index < n {
				if m.items[m.state.index].items.len > 0 {
					m.open_sub(m.state.index, true, mut cx)
				} else {
					run_item(m.items[m.state.index], mut cx)
				}
			}
			cx.stop_propagation()
			return
		}
		.right {
			if m.state.index >= 0 && m.state.index < n && m.items[m.state.index].items.len > 0 {
				m.open_sub(m.state.index, true, mut cx)
			}
			cx.stop_propagation()
			return
		}
		.left {
			m.close_self(mut cx)
			cx.stop_propagation()
			return
		}
		else {
			return
		}
	}

	mut i := m.state.index
	for _ in 0 .. n {
		i = (i + step + n) % n
		if !m.items[i].separator && !m.items[i].disabled && !m.items[i].heading {
			break
		}
	}
	m.state.index = i
	cx.win.invalidate() // the highlight is part of the menu's element tree
	cx.stop_propagation()
}

pub fn (mut m Menu) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('menu${m.key}')
	m.state = cx.state(MenuState{})
	cx.leave()
	m.div.focus_id = stable_focus(mut cx, 'menu${m.key}')
	m.div.keys << ui.KeySpec{.key_down, '', core.AnyListener{
		holder: prim.hold(m)
		call:   menu_keys
	}}
	if !m.state.focused && m.grab {
		m.state.focused = true
		cx.win.set_focus(m.div.focus_id, true) // keyboard users land in the menu
	}
	m.div.prepaint(bounds, mut cx)
	m.prepaint_sub(bounds, mut cx)
}

pub fn (mut m Menu) paint(bounds prim.Rect, mut cx core.PaintCx) {
	m.div.paint(bounds, mut cx)
}

// menu_button is a button that opens a menu below itself.
pub fn menu_button(id string, text string, items []MenuItem) &Button {
	return button(id).label(text).ghost().on_click(core.handler_with(items, fn (items []MenuItem, _ &core.ClickEvent, mut cx core.Cx) {
		open_popup_with(mut cx, cx.target_bounds(), .bottom_start, items, menu_popup)
	}))
}

// menu_button_with builds its items only when the menu opens, from data (no closure,
// specs/14): a render showing the button pays nothing for a long menu (a theme list). D must
// not be a pointer type.
pub fn menu_button_with[D](id string, text string, data D, items fn (d D) []MenuItem) &Button {
	return button(id).label(text).ghost().on_click(core.handler_with(LazyMenu[D]{data, items},
		open_lazy_menu[D]))
}

struct LazyMenu[D] {
	data  D
	items fn (d D) []MenuItem = unsafe { nil }
}

fn open_lazy_menu[D](m LazyMenu[D], _ &core.ClickEvent, mut cx core.Cx) {
	open_popup_with(mut cx, cx.target_bounds(), .bottom_start, m.items(m.data), menu_popup)
}

// caret marks a button as a dropdown trigger (specs/15): a 16 chevron-down after the label
// at 75 %, gap 8, the input fill.
pub fn (self_ &Button) caret() &Button {
	mut b := unsafe { self_ }
	b.caret = true
	return b
}

// context_menu opens `items` at the pointer on right click inside `child`.
pub fn context_menu(child core.Element, items []MenuItem) &ui.Div {
	return ui.div().child(child).on_mouse_down(core.handler_with(items, fn (items []MenuItem, e &core.MouseDownEvent, mut cx core.Cx) {
		if e.button == .right {
			open_popup_with(mut cx, prim.rect(e.x, e.y, 0, 0), .bottom_start, items, menu_popup)
			cx.stop_propagation()
		}
	}))
}

// open_context_menu shows `items` at a point (for elements with their own mouse handling).
pub fn open_context_menu(mut cx core.Cx, x f32, y f32, items []MenuItem) {
	open_popup_with(mut cx, prim.rect(x, y, 0, 0), .bottom_start, items, menu_popup)
}

pub struct MenuDef {
pub:
	title string
	items []MenuItem
}

// menubar is a row of menu buttons (File, Edit, View...).
pub fn menubar(menus []MenuDef) &Styled {
	return themed_with(menus, fn (menus []MenuDef, t &Theme) core.Element {
		mut row :=
			ui.div().flex_row().items_center().gap(4).px(4).h(t.titlebar_h).bg(t.p.chrome).border_b(1).border_color(t.p.line_strong)
		for i, m in menus {
			row.child(menu_button('menubar-${i}', m.title, m.items).size(.sm))
		}
		return row
	})
}
