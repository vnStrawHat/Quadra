// The context menus and the entry points they and the app share (specs/26).
module sessionui

import quadra.core
import quadra.kit
import store

enum MenuKind {
	open
	properties
	duplicate
	new_session
	delete_session
	move_none
	move_to
	new_group
	rename_group
}

// PanelMenu is a menu row's action: an object, so no closure per item (specs/14).
struct PanelMenu {
	panel &SessionPanel = unsafe { nil }
	kind  MenuKind
	id    u64
	group string
}

fn (m PanelMenu) run(mut cx core.Cx) {
	mut p := unsafe { m.panel }
	match m.kind {
		.open {
			p.open_connect(mut cx, m.id)
		}
		.properties {
			p.open_properties(mut cx, m.id, false)
		}
		.duplicate {
			if id := p.sessions.duplicate(m.id) {
				p.refresh(mut cx)
				p.open_properties(mut cx, id, false)
			}
		}
		.new_session {
			p.open_new_session(mut cx)
		}
		.delete_session {
			p.confirm_delete(mut cx, m.id)
		}
		.move_none {
			p.sessions.set_group(m.id, ?string(none))
			p.refresh(mut cx)
		}
		.move_to {
			p.sessions.set_group(m.id, ?string(m.group))
			p.refresh(mut cx)
		}
		.new_group {
			p.open_properties(mut cx, m.id, true)
		}
		.rename_group {
			p.open_rename_group(mut cx, m.group)
		}
	}
}

fn (p &SessionPanel) act(kind MenuKind, id u64, group string) kit.MenuAction {
	return PanelMenu{
		panel: unsafe { p }
		kind:  kind
		id:    id
		group: group
	}
}

fn (p &SessionPanel) item(label string, kind MenuKind, id u64) kit.MenuItem {
	return kit.MenuItem{
		label:  label
		action: p.act(kind, id, '')
	}
}

// existing_group_names is the combobox choices and the Move to Group submenu.
fn existing_group_names(entries []store.Entry) []string {
	mut names := []string{}
	for e in entries {
		if g := store.normalized_group(e.session.group) {
			if g !in names {
				names << g
			}
		}
	}
	names.sort()
	return names
}

// session_menu is the rows of a session's context menu: the ones acting on the session,
// then the global New Session, then the destructive Delete behind its own separator.
fn (p &SessionPanel) session_menu(id u64) []kit.MenuItem {
	entries := p.sessions.sessions()
	s := p.sessions.get(id) or { store.Session{} }
	current := store.normalized_group(s.group) or { '' }
	mut groups := [
		kit.MenuItem{
			label:   'No group'
			checked: current == ''
			action:  p.act(.move_none, id, '')
		},
	]
	for g in existing_group_names(entries) {
		groups << kit.MenuItem{
			label:   g
			checked: g == current
			action:  p.act(.move_to, id, g)
		}
	}
	groups << kit.separator()
	groups << p.item('New group...', .new_group, id)
	return [p.item('Open', .open, id), p.item('Properties', .properties, id),
		p.item('Duplicate Saved Session', .duplicate, id), kit.submenu('Move to Group', groups),
		kit.separator(), p.item('New Session', .new_session, 0),
		kit.separator(), kit.MenuItem{
			label:  'Delete'
			danger: true
			action: p.act(.delete_session, id, '')
		}]
}

fn (p &SessionPanel) group_menu(r Row) []kit.MenuItem {
	name := store.parse_group_item(r.id) or { r.label }
	return [
		kit.MenuItem{
			label:  'Rename Group...'
			action: p.act(.rename_group, 0, name)
		},
		kit.separator(),
		p.item('New Session', .new_session, 0),
	]
}

fn (p &SessionPanel) blank_menu() []kit.MenuItem {
	return [p.item('New Session', .new_session, 0)]
}

// open_new_session opens the New SSH Session dialog; the dock header's "+" and the menus.
pub fn (mut p SessionPanel) open_new_session(mut cx core.Cx) {
	if p.read_only {
		return
	}
	p.open_session_dialog(mut cx, none, store.Session{}, false)
}

// open_properties opens the Edit SSH Session dialog; focus_group for "New group...".
fn (mut p SessionPanel) open_properties(mut cx core.Cx, id u64, focus_group bool) {
	if s := p.sessions.get(id) {
		p.open_session_dialog(mut cx, id, s, focus_group)
	}
}

// open_connect opens the connect dialog for a saved session (double click, Open, Enter).
pub fn (mut p SessionPanel) open_connect(mut cx core.Cx, id u64) {
	if p.read_only {
		return
	}
	s := p.sessions.get(id) or { return }
	p.open_saved_connect(mut cx, id, s)
}

// open_selected, properties_selected and delete_selected are the rebindable actions.
pub fn (mut p SessionPanel) open_selected(mut cx core.Cx) {
	if id := p.selected_session() {
		p.open_connect(mut cx, id)
	}
}

pub fn (mut p SessionPanel) properties_selected(mut cx core.Cx) {
	if id := p.selected_session() {
		if !p.read_only {
			p.open_properties(mut cx, id, false)
		}
	}
}

pub fn (mut p SessionPanel) delete_selected(mut cx core.Cx) {
	if id := p.selected_session() {
		if !p.read_only {
			p.confirm_delete(mut cx, id)
		}
	}
}
