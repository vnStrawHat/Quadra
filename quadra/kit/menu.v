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
}

@[heap]
pub struct Menu {
pub mut:
	items []MenuItem
	width f32 = 200 // the least width: long labels widen the menu instead of wrapping
mut:
	div   &ui.Div    = unsafe { nil }
	state &MenuState = unsafe { nil }
}

pub fn menu(items []MenuItem) &Menu {
	return &Menu{
		items: items
	}
}

fn run_item(item MenuItem, mut cx core.Cx) {
	if item.disabled || item.separator {
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

// row is one menu entry (specs/15): 23 tall, inset 4 with 6 padding, hover one step,
// the keyboard highlight two; the shortcut muted at the right, 16 from the label.
fn (m &Menu) row(i int, item MenuItem, t &Theme, highlighted bool, icons bool) core.Element {
	p := t.p
	if item.separator {
		return ui.div().h(1).my(6).bg(p.line)
	}
	fg := if item.disabled {
		p.text_disabled
	} else if item.danger {
		p.error
	} else {
		p.text
	}
	mut row :=
		ui.div().test_id('menu-${i}').flex_row().items_center().gap(6).mx(4).px(6).h(23).rounded(t.radius_md).role(.menuitem).label(item.label).disabled(item.disabled).text_color(fg)
	if !item.disabled {
		row.cursor(.pointer).hover(ui.Patch{
			background: p.panel_hover
		}).on_click(core.handler_with(item, click_item))
	}
	if highlighted {
		row.bg(p.press)
	}
	// The icon column is there only when some item has an icon, so labels line up either way.
	if item.glyph != '' {
		row.child(icon(item.glyph).size(14).color(if item.disabled {
			p.text_disabled
		} else {
			p.text_2
		}))
	} else if icons {
		row.child(ui.div().w(14))
	}
	row.child(ui.text(item.label)).child(ui.div().grow().min_w(16))
	if item.shortcut != '' {
		row.child(ui.text(key_label(item.shortcut)).color(if item.disabled {
			p.text_disabled
		} else {
			p.text_2
		}))
	}
	return row
}

pub fn (mut m Menu) request_layout(mut cx core.LayoutCx) layout.NodeId {
	t := theme_of(cx.app)
	m.div =
		ui.div().flex_col().py(4).min_w(m.width).bg(t.p.overlay_bg).rounded(t.radius_xl).border(1).border_color(t.p.line).shadow_level(1).text_size(t.font_size(.md)).role(.menu).focus_ring(ui.Patch{}) // the highlighted row shows focus
	hi := if m.state != unsafe { nil } { m.state.index } else { -1 }
	icons := m.items.any(it.glyph != '')
	for i, item in m.items {
		m.div.child(m.row(i, item, t, i == hi, icons))
	}
	return m.div.request_layout(mut cx)
}

fn menu_keys(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	mut m := unsafe { &Menu(holder) }
	e := unsafe { &core.KeyDownEvent(event) }
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
				run_item(m.items[m.state.index], mut cx)
			}
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
		if !m.items[i].separator && !m.items[i].disabled {
			break
		}
	}
	m.state.index = i
	cx.win.invalidate() // the highlight is part of the menu's element tree
	cx.stop_propagation()
}

pub fn (mut m Menu) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('menu')
	m.state = cx.state(MenuState{})
	cx.leave()
	m.div.focus_id = stable_focus(mut cx, 'menu')
	m.div.keys << ui.KeySpec{.key_down, '', core.AnyListener{
		holder: voidptr(m)
		call:   menu_keys
	}}
	if !m.state.focused {
		m.state.focused = true
		cx.win.set_focus(m.div.focus_id, true) // keyboard users land in the menu
	}
	m.div.prepaint(bounds, mut cx)
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

// context_menu opens `items` at the pointer on right click inside `child`.
pub fn context_menu(child core.Element, items []MenuItem) &ui.Div {
	return ui.div().child(child).on_mouse_down(core.handler_with(items, fn (items []MenuItem, e &core.MouseDownEvent, mut cx core.Cx) {
		if e.button == .right {
			open_popup_with(mut cx, prim.rect(e.x, e.y, 0, 0), .bottom_start, items, menu_popup)
			cx.stop_propagation()
		}
	}))
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
