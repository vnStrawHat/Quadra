// Element state that survives frames (scroll offsets, animation state), keyed by the
// element's stable id. State not touched during a prepaint is dropped.
module core

import quadra.prim

// state returns this element's persistent state, creating it from init on first use.
pub fn (mut cx PrepaintCx) state[T](init T) &T {
	id := cx.current_id()
	mut w := cx.win
	w.state_used[id] = true
	if h := w.state[id] {
		return prim.held[T](h) // two element types with one id panic outside -prod
	}
	h := heap_hold(init)
	w.state[id] = h
	return prim.held[T](h)
}

fn (mut w Window) drop_unused_state() {
	mut gone := []u64{}
	for id, _ in w.state {
		if !w.state_used[id] {
			gone << id
		}
	}
	for id in gone {
		w.state.delete(id)
	}
}

pub enum Role {
	none
	button
	link
	checkbox
	radio
	switch
	slider
	textbox
	listbox
	listitem
	menu
	menuitem
	tab
	tablist
	dialog
	heading
	image
	progressbar
	tree
	treeitem
	table
	row
	cell
	group
	tooltip
	alert
}

// SemanticNode is what accessibility providers and the test harness see.
pub struct SemanticNode {
pub:
	role     Role
	label    string
	value    string
	checked  bool
	disabled bool
	selected bool
	expanded bool
	focused  bool
	test_id  string
	bounds   prim.Rect
	focus_id u64 // focus target, 0 = not focusable
pub mut:
	id     u64 // set by register_semantics: the element id, stable across frames
	parent int = -1 // index of the enclosing node (push_semantics), -1 = top level
}

// register_semantics adds a node under the innermost pushed one and returns its index;
// push it while prepainting the children it contains.
pub fn (mut cx PrepaintCx) register_semantics(n SemanticNode) int {
	mut node := n
	node.id = cx.current_id()
	node.parent = if cx.sems.len > 0 { cx.sems.last() } else { -1 }
	cx.win.semantics << node
	return cx.win.semantics.len - 1
}

pub fn (mut cx PrepaintCx) push_semantics(i int) {
	cx.sems << i
}

pub fn (mut cx PrepaintCx) pop_semantics() {
	cx.sems.pop()
}
