// Popovers and hover cards. A popover is a framed panel next to an anchor: a click opens
// it; an outside click or escape closes it (the overlay host's popups). A hover card opens
// after a delay over its trigger and stays while the pointer is over the trigger or the
// card, so its links can be clicked.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

const hover_card_delay_ms = 400.0
const hover_card_grace_ms = 150.0 // crossing the gap between trigger and card

// popover_frame is the popover surface: an overlay at level 1 with padding (specs/15).
pub fn popover_frame(t &Theme, content core.Element) &ui.Div {
	return ui.div().flex_col().gap(8).p(12).rounded(t.radius_xl).bg(t.p.overlay_bg).border(1).border_color(t.p.line).shadow_level(1).text_color(t.p.text).text_size(t.ui_text(.md)).role(.dialog).child(content)
}

// open_popover shows build() in a popover next to anchor and returns its popup id.
pub fn open_popover(mut cx core.Cx, anchor prim.Rect, placement ui.Placement, build fn () core.Element) int {
	return open_popup_with(mut cx, anchor, placement, Builder{build}, fn (b Builder, t &Theme) core.Element {
		return popover_frame(t, b.f())
	})
}

// popover_button is a button that opens build() in a popover below itself.
pub fn popover_button(id string, label string, build fn () core.Element) &Button {
	return button(id).label(label).on_click(core.handler_with(Builder{build}, fn (b Builder, _ &core.ClickEvent, mut cx core.Cx) {
		open_popover(mut cx, cx.target_bounds(), .bottom_start, b.f)
	}))
}

pub struct CardState {
mut:
	since f64 = -1 // when the pointer arrived; -1 = away
	left  f64 = -1 // when the pointer left both trigger and card; -1 = still over them
	card  u64 // the card's hitbox id, once shown
}

@[heap]
pub struct HoverCard {
pub mut:
	child core.Element
mut:
	build fn () core.Element = unsafe { nil }
	node  layout.NodeId
}

// hover_card shows build() in a card below child while the pointer rests on child.
pub fn hover_card(child core.Element, build fn () core.Element) &HoverCard {
	return &HoverCard{
		child: child
		build: build
	}
}

pub fn (mut h HoverCard) request_layout(mut cx core.LayoutCx) layout.NodeId {
	h.node = h.child.request_layout(mut cx)
	return h.node
}

pub fn (mut h HoverCard) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	id := cx.enter('hover-card')
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{
		hover_prepaint: true
	})
	mut st := cx.state(CardState{})
	cx.leave()
	cx.push_hitbox(hb)
	h.child.prepaint(bounds, mut cx)
	cx.pop_hitbox()
	now := cx.now_ms()
	over := cx.is_hovered_id(id) || (st.card != 0 && cx.is_hovered_id(st.card))
	if over {
		st.left = -1
		if st.since < 0 {
			st.since = now
		}
	} else if st.since >= 0 {
		if st.left < 0 {
			st.left = now
		}
		if now - st.left >= hover_card_grace_ms || st.card == 0 {
			st.since, st.left, st.card = -1, -1, 0
			return
		}
		cx.request_prepaint_after(int(hover_card_grace_ms - (now - st.left)) + 1)
	} else {
		return
	}
	if now - st.since < hover_card_delay_ms {
		cx.request_prepaint_after(int(hover_card_delay_ms - (now - st.since)) + 1)
		return
	}
	t := theme_of(cx.app)
	box := &CardBox{
		content: popover_frame(t, h.build())
		state:   st
	}
	mut a := ui.anchored(box).at(bounds).placement(.bottom_start).gap(6).layer(3900)
	a.prepaint(bounds, mut cx)
}

pub fn (mut h HoverCard) paint(bounds prim.Rect, mut cx core.PaintCx) {
	h.child.paint(bounds, mut cx)
}

// CardBox is the shown card: its hitbox keeps the card open while hovered.
@[heap]
struct CardBox {
mut:
	content core.Element
	state   &CardState = unsafe { nil }
	node    layout.NodeId
}

fn (mut b CardBox) request_layout(mut cx core.LayoutCx) layout.NodeId {
	b.node = b.content.request_layout(mut cx)
	return b.node
}

fn (mut b CardBox) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	b.state.card = cx.enter('hover-card-box')
	hb := cx.insert_hitbox(bounds, core.HitboxOptions{
		hover_prepaint: true
	})
	cx.leave()
	cx.push_hitbox(hb)
	b.content.prepaint(bounds, mut cx)
	cx.pop_hitbox()
}

fn (mut b CardBox) paint(bounds prim.Rect, mut cx core.PaintCx) {
	b.content.paint(bounds, mut cx)
}
