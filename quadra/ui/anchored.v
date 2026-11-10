// Anchored overlays: content laid out as its own root and painted above everything,
// positioned next to an anchor and kept inside the window (flip, then shift).
module ui

import quadra.core
import quadra.prim
import quadra.layout

pub enum Placement {
	bottom_start
	bottom_end
	bottom
	top_start
	top_end
	top
	right_start
	right // vertically centered on the anchor (tooltips of a vertical bar)
	left_start
	center // centered in the window (modals)
}

@[heap]
pub struct Anchored {
pub mut:
	content   core.Element
	placement Placement
	gap       f32 = 4
	anchor    ?prim.Rect // explicit anchor; default: where this element sits in the layout
	priority  int
	fill      bool // stretch to the whole window (modal backdrops)
mut:
	node layout.NodeId
}

// anchored places `content` in an overlay layer next to this element's position.
pub fn anchored(content core.Element) &Anchored {
	return &Anchored{
		content: content
	}
}

pub fn (self_ &Anchored) placement(p Placement) &Anchored {
	mut a := unsafe { self_ }
	a.placement = p
	return a
}

pub fn (self_ &Anchored) at(r prim.Rect) &Anchored {
	mut a := unsafe { self_ }
	a.anchor = r
	return a
}

pub fn (self_ &Anchored) gap(v f32) &Anchored {
	mut a := unsafe { self_ }
	a.gap = v
	return a
}

pub fn (self_ &Anchored) layer(priority int) &Anchored {
	mut a := unsafe { self_ }
	a.priority = priority
	return a
}

pub fn (self_ &Anchored) fill_window() &Anchored {
	mut a := unsafe { self_ }
	a.fill = true
	a.placement = .center
	return a
}

pub fn (mut a Anchored) request_layout(mut cx core.LayoutCx) layout.NodeId {
	// Takes no space in the parent; the content is laid out separately during prepaint.
	a.node = cx.tree.node(layout.Style{
		width:  layout.px(0)
		height: layout.px(0)
	}, [])
	return a.node
}

pub fn (mut a Anchored) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	win := cx.win.size()
	mut lcx := cx.layout_cx()
	node := a.content.request_layout(mut lcx)
	if a.fill {
		cx.tree.compute_new(node, win) // new nodes: the window's own keep their results
	} else {
		cx.tree.compute_fit(node, win)
	}
	sz := cx.tree.rect(node).size()
	anchor := a.anchor or { bounds }
	r := if a.fill {
		prim.rect(0, 0, win.width, win.height)
	} else {
		place(anchor, sz, a.placement, a.gap, win)
	}
	cx.defer_draw(&OverlayContent{
		el:   a.content
		node: node
	}, r, a.priority)
}

pub fn (mut a Anchored) paint(bounds prim.Rect, mut cx core.PaintCx) {}

// place positions a box of size sz next to anchor, flipping to the other side when it
// would overflow and finally shifting it inside the window.
pub fn place(anchor prim.Rect, sz prim.Size, p Placement, gap f32, win prim.Size) prim.Rect {
	mut x, mut y := anchor.x, anchor.bottom() + gap
	match p {
		.bottom_start {}
		.bottom_end {
			x = anchor.right() - sz.width
		}
		.bottom {
			x = anchor.x + (anchor.width - sz.width) / 2
		}
		.top_start {
			y = anchor.y - gap - sz.height
		}
		.top_end {
			x, y = anchor.right() - sz.width, anchor.y - gap - sz.height
		}
		.top {
			x, y = anchor.x + (anchor.width - sz.width) / 2, anchor.y - gap - sz.height
		}
		.right_start {
			x, y = anchor.right() + gap, anchor.y
		}
		.right {
			x, y = anchor.right() + gap, anchor.y + (anchor.height - sz.height) / 2
		}
		.left_start {
			x, y = anchor.x - gap - sz.width, anchor.y
		}
		.center {
			x, y = (win.width - sz.width) / 2, (win.height - sz.height) / 2
		}
	}

	// Flip vertically when the preferred side overflows and the other side fits better.
	below := p in [.bottom_start, .bottom_end, .bottom]
	above := p in [.top_start, .top_end, .top]
	if below && y + sz.height > win.height && anchor.y - gap - sz.height >= 0 {
		y = anchor.y - gap - sz.height
	} else if above && y < 0 && anchor.bottom() + gap + sz.height <= win.height {
		y = anchor.bottom() + gap
	}
	if p in [.right_start, .right] && x + sz.width > win.width {
		x = anchor.x - gap - sz.width
	}
	if p == .left_start && x < 0 {
		x = anchor.right() + gap
	}
	x = prim.clamp(x, 0, prim.max(0, win.width - sz.width))
	y = prim.clamp(y, 0, prim.max(0, win.height - sz.height))
	return prim.rect(x, y, sz.width, sz.height)
}

// OverlayContent paints an overlay's content at its placed bounds.
@[heap]
struct OverlayContent {
mut:
	el   core.Element
	node layout.NodeId
}

fn (mut o OverlayContent) request_layout(mut _ core.LayoutCx) layout.NodeId {
	return o.node
}

fn (mut o OverlayContent) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter('overlay')
	o.el.prepaint(bounds, mut cx)
	cx.leave()
}

fn (mut o OverlayContent) paint(bounds prim.Rect, mut cx core.PaintCx) {
	o.el.paint(bounds, mut cx)
}
