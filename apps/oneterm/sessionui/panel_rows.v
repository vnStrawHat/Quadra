// The session tree: flat rows, row rendering and keyboard navigation (specs/26).
module sessionui

import quadra.core
import quadra.kit
import quadra.prim
import quadra.ui
import store

// row_height is a leaf's content height (14 rem text at the line height, no padding).
// ponytail: one height for every row (uniform list); a group row in the original is a few px
// taller (16 rem label), a variable list would give it that.
fn row_height(t &kit.Theme) f32 {
	return f32(int(t.font_size(.sm) * t.line_height + 0.99))
}

pub struct Row {
pub:
	id       string
	label    string
	subtitle string
	color    prim.Color
	depth    int
	group    bool
	open     bool
	session  u64
	has_id   bool
	parent   string
}

// flat_rows lists the visible rows, a group's members below an open group.
fn (p &SessionPanel) flat_rows(entries []store.Entry) []Row {
	mut by_id := map[u64]store.Session{}
	for e in entries {
		by_id[e.id] = e.session
	}
	mut rows := []Row{}
	for item in store.build_tree(entries, p.query) {
		if item.children.len == 0 {
			rows << session_row(item, by_id, 0, '')
			continue
		}
		open := p.group_is_open(item.id)
		rows << Row{
			id:    item.id
			label: item.label
			group: true
			open:  open
		}
		if open {
			for child in item.children {
				rows << session_row(child, by_id, 1, item.id)
			}
		}
	}
	return rows
}

fn session_row(item store.TreeItem, by_id map[u64]store.Session, depth int, parent string) Row {
	id := store.parse_session_item(item.id) or { 0 }
	s := by_id[id] or { store.Session{} }
	return Row{
		id:       item.id
		label:    item.label
		subtitle: store.session_subtitle(s)
		color:    prim.parse_hex(store.session_color_hex(s)) or { prim.rgb(0x56B6C2) }
		depth:    depth
		session:  id
		has_id:   id in by_id
		parent:   parent
	}
}

pub struct RowsData {
pub:
	ent      u32
	rows     []Row
	selected string
	th       &kit.Theme = unsafe { nil }
}

fn (p &SessionPanel) tree_list(entries []store.Entry, t &kit.Theme, mut cx core.ViewCx[SessionPanel]) core.Element {
	rows := p.flat_rows(entries)
	mut cursor := -1
	for i, r in rows {
		if r.id == p.selected {
			cursor = i
		}
	}
	return ui.uniform_list_with('session-rows', rows.len, row_height(t), RowsData{cx.entity, rows, p.selected, unsafe { t }},
		session_tree_row).reveal(cursor)
}

fn session_tree_row(d RowsData, i int) core.Element {
	t := d.th
	r := d.rows[i]
	pl := 16 * f32(r.depth) + 12
	mut row :=
		ui.div().test_id('session-row-${r.id}').flex_row().items_center().gap(8).pl(pl).pr(12).h_full().cursor(.pointer).role(.treeitem).label(r.label).selected(r.id == d.selected)
	if r.group {
		row.expanded(r.open)
		row.child(kit.icon(if r.open { kit.icon_chevron_down } else { kit.icon_chevron_right }).size(t.sp(14)).color(t.p.text_2))
		row.child(ui.text(r.label).size(t.font_size(.md)))
	} else {
		row.child(ui.div().size(8).shrink_0().bg(r.color))
		row.child(ui.div().grow().min_w(0).overflow_hidden().child(ui.text(r.label).truncate().size(t.font_size(.sm))))
		row.child(ui.text(r.subtitle).size(t.font_size(.xs)).color(t.p.text_2))
	}
	if r.id == d.selected {
		row.bg(t.p.press)
	} else {
		row.hover(ui.Patch{
			background: t.p.panel_hover
		})
	}
	row.on_mouse_down(core.listener_with[SessionPanel, Row, core.MouseDownEvent](d.ent, r,
		row_mouse_down))
	row.on_click(core.listener_with[SessionPanel, Row, core.ClickEvent](d.ent, r, row_click))
	return row
}

fn row_mouse_down(mut p SessionPanel, r Row, e &core.MouseDownEvent, mut cx core.ViewCx[SessionPanel]) {
	p.focus.focus(mut cx.Cx)
	p.select_row(r.id, mut cx)
	if e.button == .right {
		if r.group {
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.group_menu(r))
		} else if r.has_id {
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.session_menu(r.session))
		}
		cx.stop_propagation() // the list's blank-area menu is for empty space
	}
}

fn row_click(mut p SessionPanel, r Row, e &core.ClickEvent, mut cx core.ViewCx[SessionPanel]) {
	if e.button != .left {
		return
	}
	if r.group {
		p.toggle_group(r.id, mut cx)
	} else if e.clicks == 2 && r.has_id {
		p.open_connect(mut cx.Cx, r.session)
	}
}

fn (mut p SessionPanel) toggle_group(id string, mut cx core.ViewCx[SessionPanel]) {
	p.expanded[id] = !p.group_is_open(id)
	cx.notify()
}

// key is the tree's keyboard: arrows move, Right/Left open/close, Enter toggles or opens.
fn (mut p SessionPanel) key(e &core.KeyDownEvent, mut cx core.ViewCx[SessionPanel]) {
	rows := p.flat_rows(p.sessions.sessions())
	if rows.len == 0 {
		return
	}
	mut i := -1
	for k, r in rows {
		if r.id == p.selected {
			i = k
		}
	}
	match e.key {
		.down {
			p.select_row(rows[if i + 1 < rows.len { i + 1 } else { i }].id, mut cx)
		}
		.up {
			p.select_row(rows[if i > 0 { i - 1 } else { 0 }].id, mut cx)
		}
		.right {
			if i >= 0 && rows[i].group {
				if !rows[i].open {
					p.toggle_group(rows[i].id, mut cx)
				} else if i + 1 < rows.len && rows[i + 1].parent == rows[i].id {
					p.select_row(rows[i + 1].id, mut cx)
				}
			}
		}
		.left {
			if i >= 0 && rows[i].group && rows[i].open {
				p.toggle_group(rows[i].id, mut cx)
			} else if i >= 0 && rows[i].parent != '' {
				p.select_row(rows[i].parent, mut cx)
			}
		}
		.enter {
			if i < 0 {
				return
			}
			if rows[i].group {
				p.toggle_group(rows[i].id, mut cx)
			} else if rows[i].has_id {
				p.open_connect(mut cx.Cx, rows[i].session)
			}
		}
		else {
			return
		}
	}

	cx.stop_propagation()
}

// selected_session is the selected row's session id.
fn (p &SessionPanel) selected_session() ?u64 {
	return store.parse_session_item(p.selected)
}
