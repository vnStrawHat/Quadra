// Incremental layout (specs/05): a subtree built again replaces the old one in place. Its
// parts that are equal to the old ones take their layout over, then it and its ancestors are
// laid out again from the bottom until one answers its parent exactly as before: that node is
// the relayout boundary, and nothing above it runs.
module layout

// answers is what a node told its parent in the last layout: its measure cache and its last
// perform request (a copy of the node; a struct of its own with the cache array trips V's C
// ordering). A node that gives the same answers leaves its parent's layout unchanged.
fn (t &Tree) answers(id NodeId) Node {
	return t.nodes[id]
}

// forget drops a node's layout results (its subtree changed).
fn (mut t Tree) forget(id NodeId) {
	mut n := unsafe { &t.nodes[id] }
	n.valid, n.next, n.done, n.probed, n.lost = 0, 0, false, false, false
}

// replay asks id every request of a again, the perform one last, and reports whether every
// answer is the same.
fn (mut t Tree) replay(id NodeId, a Node) bool {
	mut same := a.done && !a.lost
	for i, e in a.cache {
		if has(a.valid, i) && (!a.done || i != int(a.pslot))
			&& t.layout_node(id, e.kw, e.kh, e.aw, e.ah, false) != e.out {
			same = false
		}
	}
	if a.done {
		e := a.cache[a.pslot]
		if t.layout_node(id, e.kw, e.kh, e.aw, e.ah, true) != e.out {
			same = false
		}
	}
	return same
}

fn same_style(a &Style, b &Style) bool {
	if a.grid == unsafe { nil } || b.grid == unsafe { nil } {
		return *a == *b
	}
	mut c := *a
	c.grid = b.grid
	return c == *b && *a.grid == *b.grid
}

// adopt gives the fresh subtree b the layout of the old subtree a where they are equal (same
// styles and child counts, leaves that answer the same), and reports whether all of b is.
fn (mut t Tree) adopt(a NodeId, b NodeId) bool {
	if a == b {
		return t.nodes[a].done // a node reuse() kept, or a clean view's subtree placed again
	}
	na := unsafe { &t.nodes[a] }
	nb := unsafe { &t.nodes[b] }
	mut same := na.done && !na.lost && same_style(&na.style, &nb.style)
		&& voidptr(na.measure) == voidptr(nb.measure)
	if nb.measure != unsafe { nil } {
		// A leaf answers whatever its element measures: ask it what a was asked, unless the
		// elements can tell they measure alike (a's element is intact until the build ends).
		eq := t.same_of(nb.measure)
		if same && eq != unsafe { nil } {
			same = eq(na.ctx, nb.ctx)
		} else {
			same = same && t.replay(b, t.answers(a))
		}
	} else if na.children.len != nb.children.len {
		same = false // ponytail: no matching by key, the children after an insertion relayout
	} else {
		for i in 0 .. na.children.len {
			t.nodes[nb.children[i]].parent = b // a node gets its parent when laid out: maybe never
			if !t.adopt(na.children[i], nb.children[i]) {
				same = false
			}
		}
	}
	if same {
		mut m := unsafe { &t.nodes[b] }
		m.rect, m.content, m.cache, m.valid, m.next = na.rect, na.content, na.cache, na.valid, na.next
		m.pslot, m.done, m.probed, m.lost = na.pslot, true, false, false
	}
	return same
}

// replace puts the fresh subtree where old is and lays out what that changes. The caller
// moves its own reference to the root when old was the root. It returns how many ancestors
// of fresh were laid out again (0: the change stayed inside it).
pub fn (mut t Tree) replace(old NodeId, fresh NodeId) int {
	if old == fresh {
		return 0 // a view whose tree is a clean nested view's, placed again
	}
	same := t.adopt(old, fresh)
	parent := t.nodes[old].parent
	if parent >= 0 {
		for i in 0 .. t.nodes[parent].children.len {
			if t.nodes[parent].children[i] == old {
				t.nodes[parent].children[i] = fresh
			}
		}
	}
	t.nodes[fresh].parent = parent
	t.nodes[fresh].rect = t.nodes[old].rect
	t.nodes[old].parent = -2 - fresh
	if same {
		return 0
	}
	mut ok := t.replay(fresh, t.answers(old))
		&& same_style(&t.nodes[old].style, &t.nodes[fresh].style)
	mut cur := fresh
	mut climbed := 0
	for !ok {
		p := t.nodes[cur].parent
		if p < 0 {
			// The root fills the window: its request follows from its own style.
			t.layout_root(cur, t.avail)
			break
		}
		a := t.answers(p)
		t.forget(p)
		ok = t.replay(p, a)
		cur = p
		climbed++
		t.climb_work += t.nodes[p].children.len + 1
	}
	return climbed
}

// current follows replace() from a node id that may have been replaced.
pub fn (t &Tree) current(id NodeId) NodeId {
	mut i := id
	for i >= 0 && i < t.nodes.len && t.nodes[i].parent <= -2 {
		i = -2 - t.nodes[i].parent
	}
	return i
}

// attached reports whether id is still in the tree under root (a node left behind by a
// rebuild may still point at a parent that no longer lists it).
pub fn (t &Tree) attached(id NodeId, root NodeId) bool {
	mut i := id
	for i >= 0 && i < t.nodes.len {
		if i == root {
			return true
		}
		p := t.nodes[i].parent
		if p < 0 || !t.nodes[p].children.any(it == i) {
			return false
		}
		i = p
	}
	return false
}

// verify lays a copy of the tree out from scratch and returns the first node under root whose
// rect or content differs from this tree's (tests, QUADRA_VERIFY_LAYOUT), or none.
pub fn (t &Tree) verify(root NodeId) ?string {
	mut c := Tree{
		nodes: t.nodes.clone()
	}
	c.compute(root, t.avail)
	return t.differs(&c, root)
}

fn (t &Tree) differs(c &Tree, id NodeId) ?string {
	a, b := t.nodes[id], c.nodes[id]
	if a.rect != b.rect || a.content != b.content {
		return 'node ${id}: rect ${a.rect} content ${a.content}, from scratch ${b.rect} ${b.content}'
	}
	for k in a.children {
		if d := t.differs(c, k) {
			return d
		}
	}
	return none
}
