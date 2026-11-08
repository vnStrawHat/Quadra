// Accessibility glue: the semantic nodes of each prepaint go to the platform while
// assistive technology is listening, and its actions come back as A11yEvents.
module core

import quadra.prim
import quadra.platform

// a11y_poll prepaints once when assistive technology starts listening, to send the tree.
fn (mut w Window) a11y_poll() {
	if !w.a11y_sent && w.pw.a11y_active() {
		w.invalidate_prepaint()
	}
}

fn (mut w Window) send_a11y() {
	if !w.pw.a11y_active() {
		return
	}
	w.a11y_sent = true
	mut nodes := []platform.A11yNode{cap: w.semantics.len}
	for n in w.semantics {
		nodes << platform.A11yNode{
			id:            n.id
			parent:        n.parent
			role:          unsafe { platform.A11yRole(int(n.role)) }
			name:          n.label
			value:         n.value
			automation_id: n.test_id
			checked:       n.checked
			disabled:      n.disabled
			selected:      n.selected
			expanded:      n.expanded
			focused:       n.focused
			focusable:     n.focus_id != 0
			bounds:        n.bounds
		}
	}
	w.pw.update_a11y(nodes)
}

// on_a11y performs an action: focus moves focus; the others click the node's center,
// which is what the same control does for a pointer.
fn (mut w Window) on_a11y(e platform.A11yEvent) {
	for n in w.semantics {
		if n.id != e.node || n.disabled {
			continue
		}
		if e.action == .focus {
			if n.focus_id != 0 {
				w.set_focus(n.focus_id, true)
			}
			return
		}
		c := prim.pt(n.bounds.x + n.bounds.width / 2, n.bounds.y + n.bounds.height / 2)
		for action in [platform.MouseAction.down, .up] {
			w.on_mouse(platform.MouseEvent{
				action: action
				button: .left
				x:      c.x
				y:      c.y
			})
		}
		return
	}
}
