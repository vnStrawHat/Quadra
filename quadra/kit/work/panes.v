// Panes renders a pane tree: splits become kit.split dividers (1 px, 5 px grab area) and
// leaves show the element the caller built for that group.
module work

import quadra.core
import quadra.layout
import quadra.kit
import quadra.ui

// PaneResize reports a divider drag: the split at path (child indices) gets ratio.
pub struct PaneResize {
pub:
	path  []int
	ratio f32
}

@[heap]
pub struct Panes {
	kit.Lazy
pub mut:
	id       string
	root     PaneNode
	leaves   map[string]core.Element
	resize   ?core.AnyListener
	dock     ?core.AnyListener
	header_h f32 // height a collapsed leaf keeps; 0 = the theme's tab strip height
}

pub fn panes(id string, root PaneNode, leaves map[string]core.Element) &Panes {
	return &Panes{
		id:     id
		root:   root
		leaves: leaves
	}
}

// header sets the height a collapsed leaf keeps (its header strip).
pub fn (self_ &Panes) header(h f32) &Panes {
	mut p := unsafe { self_ }
	p.header_h = h
	return p
}

// on_dock makes every pane a drop target for tabs of docking strips (dock.v).
pub fn (self_ &Panes) on_dock(l core.Listener[DockEvent]) &Panes {
	mut p := unsafe { self_ }
	p.dock = l.any
	return p
}

pub fn (self_ &Panes) on_resize(l core.Listener[PaneResize]) &Panes {
	mut p := unsafe { self_ }
	p.resize = l.any
	return p
}

pub fn (mut p Panes) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut p, mut cx)
	return p.layout_inner(el, mut cx)
}

pub fn (mut p Panes) build_tree(mut cx core.LayoutCx) core.Element {
	t := kit.theme_of(cx.app)
	if p.header_h <= 0 {
		p.header_h = t.tabs_h
	}
	return ui.div().size_full().test_id(p.id).child(p.node(p.root, []int{}, t))
}

fn (p &Panes) node(n PaneNode, path []int, t &kit.Theme) core.Element {
	if n.is_leaf() {
		el := p.leaves[n.group] or { core.Element(ui.div()) }
		box := ui.div().flex_col().size_full().overflow_hidden().child(el)
		if p.dock == none {
			return box
		}
		return &DockTarget{
			child: box
			group: n.group
			dock:  p.dock
		}
	}
	mut a := path.clone()
	a << 0
	mut b := path.clone()
	b << 1
	fa, fb := n.folded()
	if fa || fb { // a folded side keeps its header height, the other takes the rest
		first := p.side(n.children[0], a, t, fa)
		second := p.side(n.children[1], b, t, fb)
		return ui.div().flex_col().size_full().child(first).child(ui.div().h(1).shrink_0().bg(t.p.line)).child(second)
	}
	key := path.map(it.str()).join('.')
	mut s := kit.split('${p.id}-split${key}', p.node(n.children[0], a, t),
		p.node(n.children[1], b, t), n.ratio).on_resize(core.handler_with(ResizeAt{p.resize, path}, fn (at ResizeAt, r &f32, mut cx core.Cx) {
		kit.fire(at.resize, PaneResize{at.path, *r}, mut cx)
	}))
	if n.axis == .col {
		s.vertical()
	}
	return s
}

fn (p &Panes) side(n PaneNode, path []int, t &kit.Theme, folded bool) core.Element {
	box := ui.div().flex_col().w_full().overflow_hidden().child(p.node(n, path, t))
	return if folded { box.h(p.header_h).shrink_0() } else { box.grow().min_h(0) }
}

// ResizeAt is a split's resize handler data: the listener and the split's path.
struct ResizeAt {
	resize ?core.AnyListener
	path   []int
}
