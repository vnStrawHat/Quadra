// SessionPanel: the search box, the session tree and everything it opens (specs/26).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui
import store
import vault

@[params]
pub struct PanelOptions {
pub:
	store      &store.SessionStore = unsafe { nil }
	vault      &vault.VaultState   = unsafe { nil }
	read_only  bool
	on_connect core.Listener[ConnectRequest]
	on_cancel  core.Listener[u64]
}

// DialogEntry is an open dialog: its modal and how to release its view once it is closed.
struct DialogEntry {
	modal   int
	release fn (mut cx core.Cx) = unsafe { nil }
}

@[heap]
pub struct SessionPanel {
pub mut:
	sessions   &store.SessionStore = unsafe { nil }
	vault      &vault.VaultState   = unsafe { nil }
	read_only  bool
	on_connect core.AnyListener
	on_cancel  core.AnyListener
	search     core.View[input.TextInput]
	query      string
	expanded   map[string]bool
	selected   string
	focus      core.FocusHandle
	view       core.View[SessionPanel]
	// connection attempts, keyed by the id the app reports back
	attempts     map[u64]Attempt
	next_attempt u64
	rejected     map[u64]string // session id -> the sealed value the server rejected
	dialogs      []DialogEntry
	reconciling  bool
}

// new_panel creates the panel; the app mounts it with core.view_element(panel.any()).
pub fn new_panel(mut cx core.Cx, opts PanelOptions) core.View[SessionPanel] {
	v := cx.new_view(SessionPanel{
		sessions:   opts.store
		vault:      opts.vault
		read_only:  opts.read_only
		on_connect: opts.on_connect.any
		on_cancel:  opts.on_cancel.any
		focus:      cx.focus_handle()
	})
	cx.update_view(v, fn (mut p SessionPanel, mut cx core.ViewCx[SessionPanel]) {
		p.view = cx.handle()
		p.search = input.new_input(mut cx.Cx,
			id:          'session-search'
			placeholder: 'Search sessions...'
			bare:        true
		)
		input.on_change(mut cx.Cx, p.search, cx.listener(fn (mut p SessionPanel, v &string, mut cx core.ViewCx[SessionPanel]) {
			p.query = *v
			cx.notify()
		}))
	})
	return v
}

// refresh redraws after the store changed behind the panel's back.
pub fn (mut p SessionPanel) refresh(mut cx core.Cx) {
	cx.touch(p.view.model())
}

// watch_dialogs schedules the release of dialogs closed by the kit (specs/26 decision 9).
fn (mut p SessionPanel) watch_dialogs(mut cx core.ViewCx[SessionPanel]) {
	ov := cx.global[kit.Overlays]() or { return }
	mut stale := false
	for d in p.dialogs {
		if !ov.modals.any(it.id == d.modal) {
			stale = true
		}
	}
	if p.reconciling || !stale {
		return
	}
	p.reconciling = true
	cx.set_timeout(0, fn (mut p SessionPanel, mut cx core.ViewCx[SessionPanel]) {
		p.reconciling = false
		p.reconcile(mut cx.Cx)
	})
}

// reconcile releases dialogs whose modal is gone and cancels the attempts they carried.
fn (mut p SessionPanel) reconcile(mut cx core.Cx) {
	ov := cx.peek_global[kit.Overlays]() or { return }
	mut keep := []DialogEntry{cap: p.dialogs.len}
	for d in p.dialogs {
		if ov.modals.any(it.id == d.modal) {
			keep << d
			continue
		}
		mut gone := []u64{}
		for id, a in p.attempts {
			if a.modal == d.modal {
				gone << id
			}
		}
		for id in gone {
			a := p.attempts[id]
			p.attempts.delete(id)
			if !a.alert {
				cx.app.invoke(p.on_cancel, &id, cx.win, true)
			}
		}
		if d.release != unsafe { nil } {
			d.release(mut cx)
		}
	}
	p.dialogs = keep
}

// track remembers an open dialog so reconcile releases it.
fn (mut p SessionPanel) track(modal int, release fn (mut cx core.Cx)) {
	p.dialogs << DialogEntry{modal, release}
}

// select_row highlights a tree item (any mouse button).
fn (mut p SessionPanel) select_row(id string, mut cx core.ViewCx[SessionPanel]) {
	p.selected = id
	cx.notify()
}

fn (p &SessionPanel) group_is_open(id string) bool {
	return p.expanded[id] or { true } // Rust builds folders expanded
}

@[markused]
pub fn (mut p SessionPanel) render(mut cx core.ViewCx[SessionPanel]) core.Element {
	t := kit.theme(cx)
	entries := p.sessions.sessions()
	q := p.query.trim_space().to_lower()
	has_sessions := entries.len > 0
	has_results := if q == '' {
		has_sessions
	} else {
		entries.any(store.session_matches(it.session, q))
	}
	p.watch_dialogs(mut cx)
	header :=
		ui.div().flex_row().items_center().px(t.sp(12)).py(t.sp(8)).child(ui.div().grow().min_w(0).h(t.height(.sm)).flex_col().justify_center().px(t.sp(8)).border_b(1).border_color(t.p.line).child(cx.child_view(p.search)))
	mut list := ui.div().id('session-list').test_id('session-list').flex_col().grow().min_h(0).overflow_hidden().on_mouse_down(cx.listener(fn (mut p SessionPanel, e &core.MouseDownEvent, mut cx core.ViewCx[SessionPanel]) {
		if e.button == .right {
			kit.open_context_menu(mut cx.Cx, e.x, e.y, p.blank_menu())
			cx.stop_propagation()
		}
	}))
	if !has_sessions {
		list.child(center_note('No SSH session yet. Right-click → New Session.', t))
	} else if !has_results {
		list.child(center_note('No sessions found for "${p.query.trim_space()}".', t))
	} else {
		list.child(p.tree_list(entries, t, mut cx))
	}
	return ui.div().id('session-panel').test_id('session-panel').flex_col().size_full().role(.group).label('Sessions').bg(t.p.canvas).text_size(t.font_size(.md)).text_color(t.p.text).track_focus(p.focus).on_key_down(cx.listener(fn (mut p SessionPanel, e &core.KeyDownEvent, mut cx core.ViewCx[SessionPanel]) {
		p.key(e, mut cx)
	})).child(header).child(list)
}

fn center_note(text string, t &kit.Theme) core.Element {
	return ui.div().grow().center().w_full().child(ui.text(text).size(t.font_size(.sm)).color(t.p.text_2))
}
