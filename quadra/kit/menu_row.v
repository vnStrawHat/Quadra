// Menu rows (specs/15, Menu): One's 23 px rows, or with Controls.roomy_menus 26 px rows 2
// apart; a select's option list (Menu.list) has padded rows with a trailing check.
module kit

import quadra.core
import quadra.ui

// MenuLook is the metrics of a menu's rows for a theme.
struct MenuLook {
	roomy  bool
	list   bool // a select's options
	row_h  f32  // 0: from the content (list rows)
	pad    f32  // the row's side padding
	gap    f32  // between rows
	icon   f32
	spacer f32 // the least space between the label and the shortcut
}

fn menu_look(t &Theme, list bool) MenuLook {
	if list {
		return MenuLook{
			roomy:  true
			list:   true
			pad:    t.sp(7)
			gap:    0
			icon:   12
			spacer: t.sp(8)
		}
	}
	if t.ctl.roomy_menus {
		return MenuLook{
			roomy:  true
			row_h:  26
			pad:    t.sp(8)
			gap:    t.sp(2)
			icon:   12
			spacer: t.sp(4)
		}
	}
	return MenuLook{
		row_h:  23
		pad:    6
		icon:   14
		spacer: 16
	}
}

// row is one menu entry (specs/15): 23 tall, inset 4 with 6 padding, hover one step,
// the keyboard highlight two; the shortcut muted at the right, 16 from the label.
fn (m &Menu) row(i int, item MenuItem, t &Theme, highlighted bool, icons bool) core.Element {
	p := t.p
	lk := menu_look(t, m.list)
	if item.separator {
		return separator_row(item, t)
	}
	if item.heading {
		if lk.roomy {
			return ui.div().mx(4).px(lk.pad).child(ui.text(item.label).color(p.text_2))
		}
		return ui.div().flex_row().items_center().h(20).mx(4).px(6).child(ui.text(item.label).size(t.ui_text(.sm)).color(p.text_2))
	}
	fg := if item.disabled && lk.roomy {
		p.text_2
	} else if item.disabled {
		p.text_disabled
	} else if item.danger {
		p.error
	} else {
		p.text
	}
	mut row := ui.div().test_id('menu-${i}').flex_row().items_center().gap(if lk.roomy {
		t.sp(4)
	} else {
		6
	}).mx(4).px(lk.pad).rounded(t.radius_md).role(.menuitem).label(item.label).disabled(item.disabled).text_color(fg)
	if lk.row_h > 0 {
		row.h(lk.row_h)
	} else {
		row.py(t.sp(4))
	}
	if !item.disabled {
		row.cursor(.pointer).hover(ui.Patch{
			background: if lk.list { p.panel_hover.alpha(0.7) } else { p.panel_hover }
		})
		if item.items.len > 0 {
			row.on_click(core.handler_with(SubRow{unsafe { m }, i}, click_sub_row))
		} else {
			row.on_click(core.handler_with(item, click_item))
		}
	}
	if m.items.any(it.items.len > 0) { // hovering any row opens or closes the submenu
		row.on_mouse_move(core.handler_with(SubRow{unsafe { m }, i}, hover_sub_row))
	}
	if lk.list && item.checked {
		row.bg(p.panel_hover)
	}
	if highlighted || (m.state != unsafe { nil } && m.state.sub == i) {
		row.bg(p.press)
	}
	// The icon column is there only when some item has an icon, so labels line up either way.
	if item.checked && !lk.list {
		row.child(icon(icon_check).size(lk.icon).color(if lk.roomy { fg } else { p.accent_text }))
	} else if item.glyph != '' {
		row.child(icon(item.glyph).size(lk.icon).color(if item.disabled {
			p.text_disabled
		} else {
			p.text_2
		}))
	} else if icons {
		row.child(ui.div().w(lk.icon))
	}
	if item.swatch.a > 0 {
		row.child(ui.div().size(8).shrink_0().bg(item.swatch))
	}
	row.child(ui.text(item.label)).child(ui.div().grow().min_w(lk.spacer))
	if item.shortcut != '' {
		mut keys := ui.text(key_text(item.shortcut, t)).color(if item.disabled && !lk.roomy {
			p.text_disabled
		} else {
			p.text_2
		})
		if lk.roomy {
			keys.size(t.ui_text(.sm))
		}
		row.child(keys)
	}
	if lk.list && item.checked {
		row.child(icon(icon_check).size(12).color(p.text))
	}
	if item.items.len > 0 {
		row.child(icon(icon_chevron_right).size(12).color(p.text_2))
		return &SubAnchor{
			el:    row
			menu:  unsafe { m }
			index: i
		}
	}
	return row
}
