// Tooltip: shows a label above an element (or at `placement`) after 500 ms of hover; the
// label may carry a shortcut after it and a line of detail under it.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

const tooltip_delay_ms = 500.0

pub struct TipState {
mut:
	since f64 = -1 // when hovering started; -1 = not hovering (the clock may start at 0)
}

@[heap]
pub struct Tooltip {
pub mut:
	child     core.Element
	text      string
	keys      string // a shortcut after the text, muted
	meta      string // a line under the text, 12 px muted
	placement ui.Placement = .top
mut:
	node   layout.NodeId
	bubble ?&ui.Anchored
}

pub fn tooltip(child core.Element, text string) &Tooltip {
	return &Tooltip{
		child: child
		text:  text
	}
}

pub fn (self_ &Tooltip) keys(s string) &Tooltip {
	mut tp := unsafe { self_ }
	tp.keys = s
	return tp
}

pub fn (self_ &Tooltip) meta(s string) &Tooltip {
	mut tp := unsafe { self_ }
	tp.meta = s
	return tp
}

pub fn (self_ &Tooltip) placement(p ui.Placement) &Tooltip {
	mut tp := unsafe { self_ }
	tp.placement = p
	return tp
}

pub fn (mut tp Tooltip) request_layout(mut cx core.LayoutCx) layout.NodeId {
	tp.node = tp.child.request_layout(mut cx)
	return tp.node
}

pub fn (mut tp Tooltip) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	id := cx.enter('tooltip:${tp.text}') // siblings without ids each keep their own hover
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{
		hover_prepaint: true
	})
	mut st := cx.state(TipState{})
	cx.leave()
	// The child's hitboxes nest under ours, so hovering the child hovers the tooltip.
	cx.push_hitbox(hb)
	tp.child.prepaint(bounds, mut cx)
	cx.pop_hitbox()
	if !cx.is_hovered_id(id) {
		st.since = -1
		return
	}
	now := cx.now_ms()
	if st.since < 0 {
		st.since = now
	}
	if now - st.since < tooltip_delay_ms {
		cx.request_prepaint_after(int(tooltip_delay_ms - (now - st.since)) + 1)
	} else {
		t := theme_of(cx.app)
		// Controls.roomy_menus: padding 8 x 2, radius_md, a 12 shortcut 12 after the text,
		// placed at the pointer + 1 (specs/15).
		roomy := t.ctl.roomy_menus
		mut line :=
			ui.div().flex_row().items_center().gap(t.sp(if roomy { 12 } else { 16 })).child(ui.text(tp.text))
		if tp.keys != '' {
			mut keys := ui.text(key_text(tp.keys, t)).color(t.p.text_2)
			if roomy {
				keys.size(t.ui_text(.sm))
			}
			line.child(keys)
		}
		mut bubble := ui.div().flex_col().px(t.sp(8)).py(t.sp(if roomy { 2 } else { 4 })).max_w(288).rounded(if roomy {
			t.radius_md
		} else {
			t.radius_xl
		}).bg(t.p.overlay_bg).border(1).border_color(t.p.line).shadow_level(1).text_color(t.p.text).text_size(t.ui_text(.md)).role(.tooltip).label(tp.text).child(line)
		if tp.meta != '' {
			bubble.child(ui.text(tp.meta).size(t.ui_text(.sm)).color(t.p.text_2))
		}
		mut a := ui.anchored(bubble).layer(4000)
		if roomy {
			m := cx.win.mouse
			a.at(prim.rect(m.x + 1, m.y + 1, 0, 0)).placement(.bottom_start).gap(0)
		} else {
			a.at(bounds).placement(tp.placement).gap(6)
		}
		a.prepaint(bounds, mut cx)
	}
}

pub fn (mut tp Tooltip) paint(bounds prim.Rect, mut cx core.PaintCx) {
	tp.child.paint(bounds, mut cx)
}
