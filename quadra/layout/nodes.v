// Node constructors, and the recycling of a rebuilt subtree's nodes (relayout.v).
module layout

import quadra.prim

pub fn (mut t Tree) node(style Style, children []NodeId) NodeId {
	if t.recycle.len > 0 {
		return t.reuse(&style, children, unsafe { nil }, prim.Holder{})
	}
	id := t.nodes.len
	t.nodes << Node{
		style:    style
		children: children
		owner:    t.owner
	}
	return id
}

pub fn (mut t Tree) leaf(style Style, measure MeasureFn, ctx prim.Holder) NodeId {
	if t.recycle.len > 0 {
		return t.reuse(&style, []NodeId{}, measure, ctx)
	}
	t.nodes << Node{
		style:   style
		measure: measure
		ctx:     ctx
		owner:   t.owner
	}
	return t.nodes.len - 1
}

// leaf_same is leaf for contexts that can tell when they measure alike (SameFn).
pub fn (mut t Tree) leaf_same(style Style, measure MeasureFn, ctx prim.Holder, same SameFn) NodeId {
	if voidptr(t.same_last) != voidptr(measure) {
		t.register_same(measure, same)
	}
	if t.recycle.len > 0 {
		return t.reuse(&style, []NodeId{}, measure, ctx)
	}
	t.nodes << Node{
		style:   style
		measure: measure
		ctx:     ctx
		owner:   t.owner
	}
	return t.nodes.len - 1
}

pub fn (mut t Tree) add_child(parent NodeId, child NodeId) {
	t.nodes[parent].children << child
	t.nodes[child].parent = parent
}

pub fn (mut t Tree) set_children(parent NodeId, children []NodeId) {
	t.nodes[parent].children = children
	for c in children {
		t.nodes[c].parent = parent
	}
}

// reuse fills the next node of the subtree being rebuilt (recycle_below) with what a new
// one would hold. The node keeps its layout when nothing it depends on changed: no garbage,
// and most of the comparison with the old tree is done while the new one is built.
@[noinline]
fn (mut t Tree) reuse(style &Style, children []NodeId, measure MeasureFn, ctx prim.Holder) NodeId {
	id := t.recycle.pop()
	mut n := unsafe { &t.nodes[id] }
	mut keep := n.done && !n.lost && voidptr(n.measure) == voidptr(measure)
		&& same_style(&n.style, style)
	if measure != unsafe { nil } {
		same := t.same_of(measure)
		keep = keep && same != unsafe { nil } && same(n.ctx, ctx)
	} else {
		keep = keep && n.children.len == children.len
		for i, c in children {
			keep = keep && n.children[i] == c && t.nodes[c].done
		}
	}
	n.style, n.children, n.measure, n.ctx = *style, children, measure, ctx
	n.owner = t.owner
	if !keep {
		t.forget(id)
	}
	for c in children {
		t.nodes[c].parent = id
	}
	return id
}

// recycle_below hands the nodes under root that owner made (not its nested views') to the
// next adds, in the order a build makes them: children before their parent.
pub fn (mut t Tree) recycle_below(root NodeId, owner u32) {
	t.recycle.clear()
	t.collect(root, owner)
	t.recycle.reverse_in_place()
}

fn (mut t Tree) collect(id NodeId, owner u32) {
	for c in t.nodes[id].children {
		if t.nodes[c].owner == owner && t.nodes[c].parent == id {
			t.collect(c, owner)
			t.recycle << c
		}
	}
}

// end_recycle forgets the nodes a rebuild did not use (its new tree is smaller).
pub fn (mut t Tree) end_recycle() {
	t.recycle.clear()
}

struct SamePair {
	measure MeasureFn = unsafe { nil }
	same    SameFn    = unsafe { nil }
}

fn (mut t Tree) register_same(measure MeasureFn, same SameFn) {
	if voidptr(t.same_of(measure)) == unsafe { nil } {
		t.sames << SamePair{measure, same}
	}
	t.same_last = measure
}

fn (t &Tree) same_of(m MeasureFn) SameFn {
	for p in t.sames {
		if voidptr(p.measure) == voidptr(m) {
			return p.same
		}
	}
	return SamePair{}.same
}
