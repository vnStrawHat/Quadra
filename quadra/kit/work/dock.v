// Docking (specs/20): a tab torn off its strip (dragged 12 px down or up) can be dropped on
// another strip (move it there, at that position) or on a pane: its middle moves the tab
// into that pane's group, an edge splits the pane there. The app owns the tabs and the
// tree, so docking only reports a DockEvent; PaneNode.dock_split builds the new tree.
module work

import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.layout
import quadra.ui

const dock_kind = 'dock-tab'

// tab_drag_kind is the drop kind of a dragged tab, for apps with their own drop targets.
pub const tab_drag_kind = dock_kind

// tab_drag_id is the id of the tab a drag carries.
pub fn tab_drag_id(p core.DragPayload) string {
	_, tab := dock_parts(p)
	return tab
}

// tab_drag_source is a drag of a tab for apps that lay tabs out themselves.
pub fn tab_drag_source(id string, title string, glyph string) core.DragSource {
	return core.DragSource{
		payload:    dock_payload('', id)
		preview_el: core.Element(&TabChip{
			title: title
			glyph: glyph
		})
	}
}

pub enum DockZone {
	center
	left
	right
	top
	bottom
}

// DockEvent asks to move tab from group `from` to group `to`: into it (center, at index,
// -1 = the end), or into a new group split off on the zone's side.
pub struct DockEvent {
pub:
	tab   string
	from  string
	to    string
	zone  DockZone
	index int = -1
}

// zone_at tells which part of a pane a point is over: the outer quarter on each side is
// an edge, the rest the middle.
pub fn zone_at(r prim.Rect, x f32, y f32) DockZone {
	fx := (x - r.x) / prim.max(1, r.width)
	fy := (y - r.y) / prim.max(1, r.height)
	edge := prim.min(prim.min(fx, 1 - fx), prim.min(fy, 1 - fy))
	if edge >= 0.25 {
		return .center
	}
	return if edge == fx {
		.left
	} else if edge == 1 - fx {
		.right
	} else if edge == fy {
		.top
	} else {
		.bottom
	}
}

// zone_rect is the area a drop on zone would take, for the highlight.
fn zone_rect(r prim.Rect, z DockZone) prim.Rect {
	return match z {
		.center { r }
		.left { prim.rect(r.x, r.y, r.width / 2, r.height) }
		.right { prim.rect(r.x + r.width / 2, r.y, r.width / 2, r.height) }
		.top { prim.rect(r.x, r.y, r.width, r.height / 2) }
		.bottom { prim.rect(r.x, r.y + r.height / 2, r.width, r.height / 2) }
	}
}

// dock_split puts new_group next to target on the zone's side (center: unchanged).
pub fn (n PaneNode) dock_split(target string, zone DockZone, new_group string) PaneNode {
	return match zone {
		.center { n }
		.left { n.split(target, .row, new_group, true) }
		.right { n.split(target, .row, new_group, false) }
		.top { n.split(target, .col, new_group, true) }
		.bottom { n.split(target, .col, new_group, false) }
	}
}

fn dock_payload(group string, tab string) core.DragPayload {
	return core.DragPayload{dock_kind, '${group}\n${tab}'}
}

fn dock_parts(p core.DragPayload) (string, string) {
	return p.value.all_before('\n'), p.value.all_after('\n')
}

// DockTarget wraps a pane's content: drops of tabs land on it, and the zone a drop would
// take is highlighted while a tab is over it.
@[heap]
struct DockTarget {
mut:
	child  core.Element
	group  string
	dock   ?core.AnyListener
	node   layout.NodeId
	hitbox int = -1
}

fn (mut d DockTarget) request_layout(mut cx core.LayoutCx) layout.NodeId {
	d.node = d.child.request_layout(mut cx)
	return d.node
}

fn (mut d DockTarget) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('dock:${d.group}')
	d.hitbox = cx.insert_hitbox(bounds, core.HitboxOptions{
		drop_kind: dock_kind
	})
	cx.on_mouse(d.hitbox, .drop, core.AnyListener{
		holder: voidptr(d)
		call:   dock_drop
	}, false)
	cx.leave()
	cx.push_hitbox(d.hitbox)
	d.child.prepaint(bounds, mut cx)
	cx.pop_hitbox()
}

fn (mut d DockTarget) paint(bounds prim.Rect, mut cx core.PaintCx) {
	d.child.paint(bounds, mut cx)
	if !cx.is_drop_target(d.hitbox) {
		return
	}
	t := kit.theme_of(cx.app)
	m := cx.win.mouse
	cx.quad(gpu.Quad{
		bounds:        zone_rect(bounds, zone_at(bounds, m.x, m.y)).inset(prim.edges(4))
		background:    drop_fill(t)
		border_widths: prim.edges(1)
		border_color:  t.p.border_focused
		radii:         prim.corners(t.radius_sm)
	})
}

fn dock_drop(holder voidptr, _ voidptr, event voidptr, mut cx core.Cx) {
	d := unsafe { &DockTarget(holder) }
	e := unsafe { &core.DropEvent(event) }
	from, tab := dock_parts(e.payload)
	kit.fire(d.dock, DockEvent{tab, from, d.group, zone_at(e.bounds, e.x, e.y), -1}, mut cx)
}

// TabChip builds the preview chip at layout time, when the theme is known.
@[heap]
struct TabChip {
	kit.Lazy
	title string
	glyph string
}

fn (mut c TabChip) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut c, mut cx)
	return c.layout_inner(el, mut cx)
}

fn (mut c TabChip) build_tree(mut cx core.LayoutCx) core.Element {
	t := kit.theme_of(cx.app)
	mut chip :=
		ui.div().flex_row().items_center().gap(4).h(t.tabs_h - 4).px(8).rounded(t.radius_md).bg(t.p.canvas).border(1).border_color(t.p.line_strong).shadow_level(1)
	if c.glyph != '' {
		chip.child(kit.icon(c.glyph).size(14).color(t.p.text_2))
	}
	return chip.child(ui.text(c.title).color(t.p.text))
}

// drop_target makes a docking strip accept tabs: one from this strip is moved to the slot
// under the pointer (TabMove), one from another group is reported as a DockEvent.
fn (s &TabStrip) drop_target(d &ui.Div) &ui.Div {
	if s.dock == none {
		return d
	}
	return d.on_drop(dock_kind, core.handler_with(StripTab{
		strip: unsafe { s }
	}, fn (sd StripTab, e &core.DropEvent, mut cx core.Cx) {
		strip := sd.strip
		from, tab := dock_parts(e.payload)
		rects := if strip.st != unsafe { nil } { strip.st.rects } else { []prim.Rect{} }
		mut index := rects.len
		for i, r in rects {
			if e.x < r.x + r.width / 2 {
				index = i
				break
			}
		}
		if from == strip.dock_group {
			at := strip.tabs.map(it.id).index(tab)
			kit.fire(strip.move, TabMove{tab, if at >= 0 && at < index { index - 1 } else { index }}, mut
				cx)
		} else {
			kit.fire(strip.dock, DockEvent{tab, from, strip.dock_group, .center, index}, mut cx)
		}
	}))
}

// drop_fill is the fill of a drop target (specs/15): a neutral grey at half opacity.
fn drop_fill(t &kit.Theme) prim.Color {
	return if t.mode == .light { prim.rgba(0x7E808780) } else { prim.rgba(0x83899480) }
}
