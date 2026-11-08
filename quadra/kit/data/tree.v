// Tree: virtualized hierarchical list with keyboard navigation.
module data

import quadra.core
import quadra.kit
import quadra.ui

pub struct TreeNode {
pub:
	id       string
	label    string
	glyph    string
	children []TreeNode
}

struct FlatNode {
	node   TreeNode
	depth  int
	parent string
}

@[heap]
pub struct Tree {
pub mut:
	id       string
	roots    []TreeNode
	expanded map[string]bool
	selected string
	focus    core.FocusHandle
	on_sel   ?core.AnyListener // string id
	row_h    f32 = 26
}

pub fn new_tree(mut cx core.Cx, id string, roots []TreeNode) core.View[Tree] {
	return cx.new_view(Tree{
		id:    id
		roots: roots
		focus: cx.focus_handle()
	})
}

pub fn on_tree_select(mut cx core.Cx, t core.View[Tree], l core.Listener[string]) {
	mut tr := cx.peek(t.model())
	tr.on_sel = l.any
}

fn (t &Tree) flatten() []FlatNode {
	mut out := []FlatNode{}
	mut stack := [][]FlatNode{}
	_ = stack
	t.walk(t.roots, 0, '', mut out)
	return out
}

fn (t &Tree) walk(nodes []TreeNode, depth int, parent string, mut out []FlatNode) {
	for n in nodes {
		out << FlatNode{n, depth, parent}
		if n.children.len > 0 && t.expanded[n.id] {
			t.walk(n.children, depth + 1, n.id, mut out)
		}
	}
}

fn (mut t Tree) choose(id string, mut cx core.ViewCx[Tree]) {
	t.selected = id
	if l := t.on_sel {
		v := id
		cx.app.invoke(l, &v, cx.win, true)
	}
}

fn (mut t Tree) key(e &core.KeyDownEvent, mut cx core.ViewCx[Tree]) {
	flat := t.flatten()
	mut i := 0
	for k, f in flat {
		if f.node.id == t.selected {
			i = k
		}
	}
	if flat.len == 0 {
		return
	}
	cur := flat[i]
	match e.key {
		.down {
			t.choose(flat[if i + 1 < flat.len { i + 1 } else { i }].node.id, mut cx)
		}
		.up {
			t.choose(flat[if i > 0 { i - 1 } else { 0 }].node.id, mut cx)
		}
		.right {
			if cur.node.children.len > 0 && !t.expanded[cur.node.id] {
				t.expanded[cur.node.id] = true
			} else if cur.node.children.len > 0 {
				t.choose(cur.node.children[0].id, mut cx)
			}
		}
		.left {
			if t.expanded[cur.node.id] {
				t.expanded[cur.node.id] = false
			} else if cur.parent != '' {
				t.choose(cur.parent, mut cx)
			}
		}
		.enter, .space {
			if cur.node.children.len > 0 {
				t.expanded[cur.node.id] = !t.expanded[cur.node.id]
			}
		}
		else {
			return
		}
	}

	cx.stop_propagation()
}

@[markused]
pub fn (mut t Tree) render(mut cx core.ViewCx[Tree]) core.Element {
	th := kit.theme(cx)
	flat := t.flatten()
	selected := t.selected
	focused := t.focus.is_focused(cx.Cx)
	expanded := t.expanded.clone()
	id := t.id
	ent := cx.entity
	mut cursor := -1
	for k, f in flat {
		if f.node.id == selected {
			cursor = k
		}
	}
	list := ui.uniform_list_with('${t.id}-rows', flat.len, t.row_h, TreeRows{ent, flat, selected, focused, expanded, id, unsafe { th }},
		tree_row).reveal(cursor)
	return ui.div().test_id(t.id).flex_col().size_full().py(4).role(.tree).track_focus(t.focus).on_key_down(cx.listener(fn (mut t Tree, e &core.KeyDownEvent, mut cx core.ViewCx[Tree]) {
		t.key(e, mut cx)
	})).child(list)
}

// TreeRows is what the visible rows are built from.
struct TreeRows {
	ent      u32
	flat     []FlatNode
	selected string
	focused  bool
	expanded map[string]bool
	id       string
	th       &kit.Theme = unsafe { nil }
}

fn tree_row(d TreeRows, i int) core.Element {
	ent, flat, selected, focused, id, th := d.ent, d.flat, d.selected, d.focused, d.id, d.th
	p := th.p
	f := flat[i]
	n := f.node
	on := n.id == selected
	nid := n.id
	has_kids := n.children.len > 0
	// A panel row (specs/15): 26 tall, indent 20 with 1 px guides, hover one step, selected
	// two; the selected row of a focused tree is outlined in the focus color.
	mut row := ui.div().test_id('${id}-${n.id}').flex_row().items_center().gap(2).pl(8 +
		f32(f.depth) * indent).pr(8).h_full().cursor(.pointer).role(.treeitem).label(n.label).selected(on).expanded(d.expanded[n.id]).text_size(th.font_size(.md)).text_color(p.text)
	for k in 0 .. f.depth {
		row.child(ui.div().absolute().left(8 + f32(k) * indent + 7).top(0).w(1).h_full().bg(p.text.alpha(0.1)))
	}
	if on {
		row.bg(p.press)
		if focused {
			row.border(1).border_color(p.border_focused)
		}
	} else {
		row.hover(ui.Patch{
			background: p.raised_hover
		})
	}
	row.on_click(core.listener_with[Tree, TreeClick, core.ClickEvent](ent, TreeClick{nid, has_kids}, fn (mut t Tree, k TreeClick, e &core.ClickEvent, mut cx core.ViewCx[Tree]) {
		nid, has_kids := k.nid, k.has_kids
		if has_kids && (e.clicks == 2 || t.selected == nid) {
			t.expanded[nid] = !t.expanded[nid]
		}
		t.choose(nid, mut cx)
	}))
	row.child(if has_kids {
		core.Element(kit.icon(if d.expanded[n.id] {
			kit.icon_chevron_down
		} else {
			kit.icon_chevron_right
		}).size(16).color(p.text_2))
	} else {
		core.Element(ui.div().w(16))
	})
	if n.glyph != '' {
		row.child(kit.icon(n.glyph).size(16).color(p.text_2))
	}
	return row.child(ui.div().pl(4).child(ui.text(n.label)))
}

const indent = f32(20)

struct TreeClick {
	nid      string
	has_kids bool
}
