// Lazy is embedded by kit components: they build their element tree at layout time from
// the current theme, so switching themes needs a relayout but never a re-render.
module kit

import quadra.core
import quadra.prim
import quadra.layout

pub struct Lazy {
mut:
	inner ?core.Element
}

pub fn (mut l Lazy) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if mut el := l.inner {
		el.prepaint(bounds, mut cx)
	}
}

pub fn (mut l Lazy) paint(bounds prim.Rect, mut cx core.PaintCx) {
	if mut el := l.inner {
		el.paint(bounds, mut cx)
	}
}

pub fn (mut l Lazy) layout_inner(el core.Element, mut cx core.LayoutCx) layout.NodeId {
	l.inner = el
	mut e := el
	return e.request_layout(mut cx)
}
