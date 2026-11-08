// The pane tree of a workbench (specs/16): nested two-way splits whose leaves are tab
// groups. Plain data, so apps keep it in their model and persist it with their settings;
// every operation returns a new tree.
module work

pub enum Axis {
	row // children side by side
	col // children stacked
}

pub struct PaneNode {
pub mut:
	group     string // leaf: the tab group shown here
	collapsed bool   // leaf in a stacked split: only its header shows
	axis      Axis
	ratio     f32 = 0.5 // share of the first child
	children  []PaneNode // none for a leaf, two for a split
}

pub fn leaf(group string) PaneNode {
	return PaneNode{
		group: group
	}
}

pub fn hsplit(a PaneNode, b PaneNode, ratio f32) PaneNode {
	return PaneNode{
		axis:     .row
		ratio:    ratio
		children: [a, b]
	}
}

pub fn vsplit(a PaneNode, b PaneNode, ratio f32) PaneNode {
	return PaneNode{
		axis:     .col
		ratio:    ratio
		children: [a, b]
	}
}

pub fn (n PaneNode) is_leaf() bool {
	return n.children.len == 0
}

// groups lists the leaves' groups in reading order.
pub fn (n PaneNode) groups() []string {
	if n.is_leaf() {
		return [n.group]
	}
	mut out := n.children[0].groups()
	out << n.children[1].groups()
	return out
}

// split puts new_group next to group (after it, or before) along axis, sharing its space.
pub fn (n PaneNode) split(group string, axis Axis, new_group string, before bool) PaneNode {
	if n.is_leaf() {
		if n.group != group {
			return n
		}
		fresh := leaf(new_group)
		return PaneNode{
			axis:     axis
			children: if before { [fresh, n] } else { [n, fresh] }
		}
	}
	return PaneNode{
		...n
		children: [n.children[0].split(group, axis, new_group, before),
			n.children[1].split(group, axis, new_group, before)]
	}
}

// remove drops a group's leaf; its sibling takes the parent's place. The last leaf stays.
pub fn (n PaneNode) remove(group string) PaneNode {
	if n.is_leaf() {
		return n
	}
	for i in 0 .. 2 {
		c := n.children[i]
		if c.is_leaf() && c.group == group {
			return n.children[1 - i]
		}
	}
	return PaneNode{
		...n
		children: [n.children[0].remove(group), n.children[1].remove(group)]
	}
}

// with_ratio sets the ratio of the split at path (child indices from the root), clamped
// so both sides keep at least 5 %.
pub fn (n PaneNode) with_ratio(path []int, ratio f32) PaneNode {
	if n.is_leaf() {
		return n
	}
	if path.len == 0 {
		return PaneNode{
			...n
			ratio: if ratio < 0.05 {
				f32(0.05)
			} else if ratio > 0.95 {
				f32(0.95)
			} else {
				ratio
			}
		}
	}
	mut kids := n.children.clone()
	kids[path[0]] = kids[path[0]].with_ratio(path[1..], ratio)
	return PaneNode{
		...n
		children: kids
	}
}

// toggle collapses a group's leaf to its header, or opens it again (stacked splits only).
pub fn (n PaneNode) toggle(group string) PaneNode {
	if n.is_leaf() {
		return if n.group == group {
			PaneNode{
				...n
				collapsed: !n.collapsed
			}
		} else {
			n
		}
	}
	if n.axis == .row && n.children.any(it.is_leaf() && it.group == group) {
		return n // side by side leaves never fold
	}
	return PaneNode{
		...n
		children: [n.children[0].toggle(group), n.children[1].toggle(group)]
	}
}

// folded tells which children of a split show only their header. Only stacked splits fold,
// and never both sides: the second one stays open.
fn (n PaneNode) folded() (bool, bool) {
	if n.axis != .col {
		return false, false
	}
	a := n.children[0].is_leaf() && n.children[0].collapsed
	b := n.children[1].is_leaf() && n.children[1].collapsed && !a
	return a, b
}

// is_collapsed tells whether a group's leaf is folded to its header.
pub fn (n PaneNode) is_collapsed(group string) bool {
	if n.is_leaf() {
		return n.group == group && n.collapsed
	}
	return n.children[0].is_collapsed(group) || n.children[1].is_collapsed(group)
}
