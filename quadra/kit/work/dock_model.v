module work

// Dock is the state of a docking workbench, for apps that keep tabs per group: tab groups
// laid out by a pane tree. apply and move update it from the strips' events.
pub struct Dock {
pub mut:
	root   PaneNode
	tabs   map[string][]Tab
	active map[string]string
	next   int // numbers the groups made by splits
}

// apply moves a tab as a DockEvent asks; a group left without tabs closes its pane.
pub fn (mut d Dock) apply(e DockEvent) {
	from := d.tabs[e.from] or { return }
	i := from.map(it.id).index(e.tab)
	if i < 0 || (e.from == e.to && e.zone == .center) {
		return
	}
	tab := from[i]
	if e.from == e.to && from.len == 1 {
		return
	}
	mut rest := from.clone()
	rest.delete(i)
	d.tabs[e.from] = rest
	if d.active[e.from] == e.tab && rest.len > 0 {
		d.active[e.from] = rest[if i < rest.len { i } else { rest.len - 1 }].id
	}
	mut to := e.to
	if e.zone != .center {
		d.next++
		to = 'group-${d.next}'
		d.root = d.root.dock_split(e.to, e.zone, to)
		d.tabs[to] = []Tab{}
	}
	mut dst := d.tabs[to] or { []Tab{} }
	at := if e.index < 0 || e.index > dst.len { dst.len } else { e.index }
	dst.insert(at, tab)
	d.tabs[to] = dst
	d.active[to] = tab.id
	if rest.len == 0 {
		d.root = d.root.remove(e.from)
		d.tabs.delete(e.from)
		d.active.delete(e.from)
	}
}

// move reorders a tab inside its group (TabMove from the group's strip).
pub fn (mut d Dock) move(group string, m TabMove) {
	mut list := d.tabs[group] or { return }
	i := list.map(it.id).index(m.id)
	if i < 0 {
		return
	}
	tab := list[i]
	list.delete(i)
	list.insert(if m.to > list.len { list.len } else { m.to }, tab)
	d.tabs[group] = list
}
