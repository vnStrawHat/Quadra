module sessionui

import os
import quadra.core
import quadra.kit
import quadra.platform
import quadra.testing
import quadra.ui
import store
import vault

const test_root = '.tmp/oneterm-tests/sessionui'

@[heap]
pub struct Rig {
mut:
	sessions &store.SessionStore = unsafe { nil }
	vault    &vault.VaultState   = unsafe { nil }
	panel    core.View[SessionPanel]
	requests []ConnectRequest
	cancels  []u64
}

@[markused]
pub fn (mut r Rig) render(mut cx core.ViewCx[Rig]) core.Element {
	if r.panel.id == 0 {
		return ui.div()
	}
	return ui.div().size_full().flex_col().child(cx.child_view(r.panel))
}

fn rig(name string, seed fn (mut s store.SessionStore)) !(&testing.Harness, &Rig, core.View[Rig]) {
	dir := os.join_path(test_root, name)
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	kit.set_theme(mut h.app, kit.dark())
	mut sessions := store.open(dir, false)
	seed(mut sessions)
	st := vault.new_state(dir, false)
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	v := h.open(Rig{ sessions: sessions, vault: st },
		width:  1000
		height: 800
	)!
	cx.update_view(v, fn (mut r Rig, mut cx core.ViewCx[Rig]) {
		r.panel = new_panel(mut cx.Cx, PanelOptions{
			store:      r.sessions
			vault:      r.vault
			on_connect: cx.listener(fn (mut r Rig, q &ConnectRequest, mut _ core.ViewCx[Rig]) {
				r.requests << *q
			})
			on_cancel:  cx.listener(fn (mut r Rig, a &u64, mut _ core.ViewCx[Rig]) {
				r.cancels << *a
			})
		})
	})
	h.frame()
	return h, cx.peek(v.model()), v
}

fn cx_of(h &testing.Harness) core.Cx {
	return core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
}

fn model(h &testing.Harness, r &Rig) &SessionPanel {
	return cx_of(h).peek(r.panel.model())
}

fn prod() store.Session {
	return store.Session{
		label:    'Prod'
		host:     '10.0.0.1'
		username: 'root'
		port:     2222
		group:    'PROD'
		color:    '#FF0000'
	}
}

fn test_tree_lists_groups_rows_and_subtitles() {
	mut h, _, _ := rig('tree', fn (mut s store.SessionStore) {
		s.add(prod())
		s.add(store.Session{ label: 'Alpha', host: 'a.test' })
	})!
	assert h.has_text('Alpha') && h.has_text('Prod') && h.has_text('PROD')
	assert h.has_text('root@10.0.0.1:2222') && h.has_text('a.test:22')
	alpha := h.text_bounds('Alpha') or { panic('no row') }
	group := h.text_bounds('PROD') or { panic('no group') }
	assert alpha.y < group.y // ungrouped first
}

fn test_group_click_collapses_it() {
	mut h, r, _ := rig('collapse', fn (mut s store.SessionStore) {
		s.add(prod())
	})!
	assert h.has_text('Prod')
	h.click('session-row-group:PROD')!
	h.frame()
	assert !h.has_text('Prod')
	h.click('session-row-group:PROD')!
	h.frame()
	assert h.has_text('Prod')
	_ = r
}

fn test_search_filters_and_shows_no_results() {
	mut h, _, _ := rig('search', fn (mut s store.SessionStore) {
		s.add(prod())
		s.add(store.Session{ label: 'Alpha', host: 'a.test' })
	})!
	h.click('session-search')!
	h.type_text('alph')
	h.frame()
	assert h.has_text('Alpha') && !h.has_text('Prod')
	h.type_text('zzz')
	h.frame()
	assert h.has_text('No sessions found for "alphzzz".')
}

fn test_empty_store_says_how_to_start() {
	mut h, _, _ := rig('empty', fn (mut _ store.SessionStore) {})!
	assert h.has_text('No SSH session yet. Right-click → New Session.')
	r := h.find('session-list') or { panic('no list') }
	h.inject(platform.MouseEvent{
		action: .down
		button: .right
		x:      r.x + 20
		y:      r.y + 20
	})
	h.inject(platform.MouseEvent{
		action: .up
		button: .right
		x:      r.x + 20
		y:      r.y + 20
	})
	h.frame()
	assert h.has_text('New Session')
}

fn right_click(mut h testing.Harness, id string) ! {
	b := h.find(id) or { return error('no ${id}') }
	x, y := b.x + b.width / 2, b.y + b.height / 2
	h.inject(platform.MouseEvent{
		action: .down
		button: .right
		x:      x
		y:      y
	})
	h.inject(platform.MouseEvent{
		action: .up
		button: .right
		x:      x
		y:      y
	})
	h.frame()
}

fn click_text(mut h testing.Harness, text string) ! {
	b := h.text_bounds(text) or { return error('no text ${text}') }
	h.click_at(b.x + 2, b.y + 2)
	h.frame()
}

fn test_session_menu_rows_and_delete_confirm() {
	mut h, r, _ := rig('menu', fn (mut s store.SessionStore) {
		s.add(prod())
	})!
	id := r.sessions.sessions()[0].id
	right_click(mut h, 'session-row-session:${id}')!
	for label in ['Open', 'Properties', 'Duplicate Saved Session', 'Move to Group', 'New Session',
		'Delete'] {
		assert h.has_text(label), label
	}
	click_text(mut h, 'Delete')!
	assert h.has_text('Delete the saved SSH session "Prod"? This cannot be undone.')
	h.click('danger-ok')!
	h.frame()
	assert r.sessions.sessions().len == 0
	assert h.has_text('SSH session deleted.')
	assert h.has_text('No SSH session yet. Right-click → New Session.')
}

fn test_group_menu_renames_for_every_member() {
	mut h, r, _ := rig('rename', fn (mut s store.SessionStore) {
		s.add(prod())
	})!
	right_click(mut h, 'session-row-group:PROD')!
	assert h.has_text('Rename Group...')
	click_text(mut h, 'Rename Group...')!
	assert h.has_text('Group Name')
	h.click('group-name')!
	h.press('ctrl-a backspace')!
	h.type_text('Live')
	h.click('dialog-save')!
	h.frame()
	assert r.sessions.sessions()[0].session.group or { '' } == 'Live'
	assert h.has_text('Group renamed.')
}
