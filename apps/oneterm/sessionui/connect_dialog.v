// The connect dialog: a saved session, Quick Connect or a duplicate (specs/26, decision 2).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui
import store

pub enum ConnectMode {
	saved
	quick
	duplicate
}

@[heap]
pub struct ConnectDialog {
pub mut:
	panel         &SessionPanel = unsafe { nil }
	ent           u32
	me            core.View[ConnectDialog]
	modal         int
	mode          ConnectMode
	session_id    ?u64
	session       store.Session
	duplicate     DuplicateSpec
	banner        string
	has_username  bool // the username input is shown
	username      core.View[input.TextInput]
	host          core.View[input.TextInput]
	port          core.View[input.TextInput]
	auth          core.View[AuthForm]
	hops          []HopForm
	picker        JumpPicker
	has_picker    bool
	picked        ?u64
	chain_error   string
	save_username bool
	save_session  bool
	connecting    bool
	error         string
	attempt       u64
}

fn title_of(mode ConnectMode, s store.Session) string {
	return match mode {
		.quick {
			'SSH Quick Connect'
		}
		.duplicate {
			'Duplicate SSH Session'
		}
		.saved {
			if u := s.username {
				'Connect to ${s.label} (${u}@${s.host}:${s.port})'
			} else {
				'Connect to ${s.label}'
			}
		}
	}
}

// open_connect_view creates the view and modal for any mode.
fn (mut p SessionPanel) open_connect_view(mut cx core.Cx, d_ ConnectDialog, specs []HopSpec) {
	v := cx.new_view(d_)
	cx.update_view_with(v, specs, fn (mut d ConnectDialog, specs []HopSpec, mut cx core.ViewCx[ConnectDialog]) {
		d.init(specs, mut cx)
	})
	mut d := cx.peek(v.model())
	d.modal = kit.open_modal(mut cx, kit.ModalOptions{
		title:       title_of(d.mode, d.session)
		width:       440
		body_with:   kit.view_body(v.any())
		footer_with: kit.build_with(ConnectRef{d}, fn (r ConnectRef, _ &kit.Theme) core.Element {
			return r.d.footer()
		})
	})
	p.track(d.modal, v.any())
	if d.mode == .saved {
		d.activate_saved(v.model(), mut cx)
	}
	d.focus_first(mut cx)
}

struct ConnectRef {
	d &ConnectDialog = unsafe { nil }
}

fn (mut p SessionPanel) open_saved_connect(mut cx core.Cx, id u64, s store.Session) {
	chain := p.sessions.jump_chain(s.jump_host, id) or {
		warn(mut cx, err.msg())
		return
	}
	mut specs := []HopSpec{}
	for e in chain {
		specs << hop_spec_of(e) or {
			warn(mut cx, err.msg())
			return
		}
	}
	banner := if u := s.username {
		'ssh://${u}@${s.host}:${s.port}'
	} else {
		'ssh://${s.host}:${s.port}'
	}
	p.open_connect_view(mut cx, ConnectDialog{
		panel:      unsafe { p }
		mode:       .saved
		session_id: id
		session:    s
		banner:     banner
	}, specs)
}

// open_quick_connect opens SSH Quick Connect: every field, connect now, optionally save.
pub fn (mut p SessionPanel) open_quick_connect(mut cx core.Cx) {
	if p.read_only {
		return
	}
	p.open_connect_view(mut cx, ConnectDialog{
		panel: unsafe { p }
		mode:  .quick
	}, [])
}

// open_duplicate opens the Duplicate SSH Session dialog, prefilled, for a running session.
pub fn (mut p SessionPanel) open_duplicate(mut cx core.Cx, spec DuplicateSpec) {
	if p.read_only {
		return
	}
	p.open_connect_view(mut cx, ConnectDialog{
		panel:     unsafe { p }
		mode:      .duplicate
		duplicate: spec
	}, spec.hops)
}

fn (mut d ConnectDialog) init(specs []HopSpec, mut cx core.ViewCx[ConnectDialog]) {
	d.ent = cx.entity
	d.me = cx.handle()
	mut p := d.panel
	s := d.session
	dup := d.duplicate
	method := match d.mode {
		.saved { s.auth_method }
		.quick { store.AuthMethod.password }
		.duplicate { dup.method }
	}

	key := match d.mode {
		.saved { s.key_path or { '' } }
		.quick { '' }
		.duplicate { dup.key_path }
	}

	d.auth = new_auth_form(mut cx.Cx, 'connect-auth', method, key, true, p.vault)
	d.watch_auth(d.auth, mut cx)
	mut auth := cx.peek(d.auth.model())
	d.hops = build_hop_forms(mut cx.Cx, specs, p)
	for h in d.hops {
		d.watch_auth(h.form, mut cx)
	}
	match d.mode {
		.saved {
			auth.saved = saved_of(s)
			auth.offers_save = method == .password
			if s.username == none {
				d.has_username = true
				d.username = input.new_input(mut cx.Cx,
					id:          'connect-username'
					placeholder: 'e.g. root, ubuntu, admin - or root@host:port'
				)
				d.watch_input(d.username, mut cx)
			}
		}
		else {
			d.host = input.new_input(mut cx.Cx,
				id:          'quick-host'
				placeholder: 'e.g. 192.168.1.10 or server.example.com'
				value:       dup.host
			)
			d.port = input.new_input(mut cx.Cx,
				id:          'quick-port'
				placeholder: '22'
				value:       if d.mode == .duplicate { dup.port.str() } else { '' }
			)
			d.username = input.new_input(mut cx.Cx,
				id:          'quick-username'
				placeholder: 'root  or  root@host:port'
				value:       dup.username
			)
			d.has_username = true
			for v in [d.host, d.port, d.username] {
				d.watch_input(v, mut cx)
			}
			if d.mode == .quick {
				d.picker = new_jump_picker(mut cx.Cx, p.sessions.sessions(), none, none)
				d.has_picker = true
				cx.peek(d.picker.select.model()).changed = core.listener_of[ConnectDialog, string](cx.entity, fn (mut d ConnectDialog, _ &string, mut cx core.ViewCx[ConnectDialog]) {
					d.rebuild_hops(mut cx)
				}).any
			}
		}
	}
}

fn (mut d ConnectDialog) watch_input(v core.View[input.TextInput], mut cx core.ViewCx[ConnectDialog]) {
	input.on_change(mut cx.Cx, v, cx.listener(fn (mut d ConnectDialog, _ &string, mut cx core.ViewCx[ConnectDialog]) {
		d.error = ''
		cx.notify()
	}))
}

fn (mut d ConnectDialog) watch_auth(v core.View[AuthForm], mut cx core.ViewCx[ConnectDialog]) {
	cx.peek(v.model()).changed = core.listener_of[ConnectDialog, bool](cx.entity, fn (mut d ConnectDialog, _ &bool, mut cx core.ViewCx[ConnectDialog]) {
		d.error = ''
		cx.notify()
	}).any
}

@[markused]
pub fn (mut d ConnectDialog) render(mut cx core.ViewCx[ConnectDialog]) core.Element {
	t := kit.theme(cx)
	mut rows := []core.Element{}
	if d.hops.len > 0 {
		rows << hops_block(d.hops, t)
	}
	if d.mode == .saved {
		rows << ui.div().w_full().px(12).py(8).rounded(t.radius_md).bg(t.p.raised).child(ui.text(d.banner).size(t.font_size(.sm)).color(t.p.text_2))
	} else {
		rows << labelled('Host', true, cx.child_view(d.host))
		rows << labelled('Port', false, cx.child_view(d.port))
	}
	if d.has_username {
		rows << labelled('Username', true, cx.child_view(d.username))
	}
	if d.has_picker {
		rows << labelled('Jump host', false, cx.child_view(d.picker.select))
		if d.chain_error != '' {
			rows << ui.text(d.chain_error).size(t.font_size(.sm)).color(t.p.error)
		}
	}
	rows << cx.child_view(d.auth)
	if d.mode == .saved && d.session.username == none {
		forced := cx.peek(d.auth.model()).wants_save()
		rows << kit.checkbox('connect-save-username', d.save_username || forced).label('Save username to session').disabled(forced).on_change(cx.listener(fn (mut d ConnectDialog, v &bool, mut cx core.ViewCx[ConnectDialog]) {
			d.save_username = *v
			cx.notify()
		}))
	}
	if d.mode == .quick {
		rows << kit.checkbox('quick-save-session', d.save_session).label('Save to SSH Sessions').on_change(cx.listener(fn (mut d ConnectDialog, v &bool, mut cx core.ViewCx[ConnectDialog]) {
			d.save_session = *v
			cx.notify()
		}))
	}
	if d.error != '' {
		note := if d.mode == .quick && d.save_session {
			'Not saved to SSH Sessions: a session is saved once its connection succeeds. The tick is still on - connect again and it will be saved.'
		} else {
			''
		}
		rows << inline_error(t, d.error, note)
	}
	return ui.div().w_full().on_key_down(cx.listener(fn (mut d ConnectDialog, e &core.KeyDownEvent, mut cx core.ViewCx[ConnectDialog]) {
		if e.key == .enter {
			d.connect(mut cx)
			cx.stop_propagation()
		}
	})).child(dialog_body(t, cx.win.size().height, rows))
}

fn (d &ConnectDialog) footer() core.Element {
	label := if d.connecting { 'Connecting' } else { 'Connect' }
	return ui.div().flex_row().gap(8).child(kit.button('dialog-cancel').label('Cancel').outline().on_click(core.handler_with(d.modal,
		close_modal_click))).child(kit.button('connect-button').label(label).primary().loading(d.connecting).disabled(d.connecting).on_click(core.listener_of[ConnectDialog, core.ClickEvent](d.ent, fn (mut d ConnectDialog, _ &core.ClickEvent, mut cx core.ViewCx[ConnectDialog]) {
		d.connect(mut cx)
	})))
}
