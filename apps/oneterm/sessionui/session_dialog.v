// The "New / Edit SSH Session" dialog (specs/26).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui
import store

@[heap]
pub struct SessionDialog {
pub mut:
	panel        &SessionPanel = unsafe { nil }
	ent          u32
	me           core.View[SessionDialog]
	modal        int
	edit_id      ?u64
	label        core.View[input.TextInput]
	host         core.View[input.TextInput]
	port         core.View[input.TextInput]
	username     core.View[input.TextInput]
	color        core.View[ColorField]
	auth         core.View[AuthForm]
	group        core.View[SearchSelect]
	jump         JumpPicker
	forwards     []&ForwardRow
	next_forward int
	advanced     bool
	agent        bool
	logging      store.LoggingOverride
	color_value  string
}

fn advanced_is_configured(jump ?u64, forwards []store.PortForward, agent bool) bool {
	return jump != none || forwards.len > 0 || agent
}

// open_session_dialog opens the dialog; `edit` is none for a new session.
fn (mut p SessionPanel) open_session_dialog(mut cx core.Cx, edit ?u64, s store.Session, focus_group bool) {
	title := if edit != none { 'Edit SSH Session' } else { 'New SSH Session' }
	v := cx.new_view(SessionDialog{
		panel:    unsafe { p }
		edit_id:  edit
		agent:    s.agent_forwarding
		logging:  s.logging
		advanced: advanced_is_configured(s.jump_host, s.forwards, s.agent_forwarding)
	})
	cx.update_view(v, fn [s, edit, focus_group] (mut d SessionDialog, mut cx core.ViewCx[SessionDialog]) {
		d.init(s, edit, focus_group, mut cx)
	})
	mut d := cx.peek(v.model())
	d.modal = kit.open_modal(mut cx, kit.ModalOptions{
		title:  title
		width:  560
		body:   fn [v] () core.Element {
			return core.view_element(v.any())
		}
		footer: fn [d] () core.Element {
			return d.footer()
		}
	})
	p.track(d.modal, fn [v] (mut cx core.Cx) {
		cx.release(v.model())
	})
}

fn (mut d SessionDialog) init(s store.Session, edit ?u64, focus_group bool, mut cx core.ViewCx[SessionDialog]) {
	d.ent = cx.entity
	d.me = cx.handle()
	entries := d.panel.sessions.sessions()
	d.label = input.new_input(mut cx.Cx,
		id:          'session-label'
		placeholder: 'e.g. Production Server'
		value:       s.label
	)
	d.host = input.new_input(mut cx.Cx,
		id:          'session-host'
		placeholder: 'e.g. 192.168.1.10'
		value:       s.host
	)
	d.port = input.new_input(mut cx.Cx,
		id:          'session-port'
		placeholder: '22'
		value:       if edit != none { s.port.str() } else { '' }
	)
	d.username = input.new_input(mut cx.Cx,
		id:          'session-username'
		placeholder: 'optional'
		value:       s.username or { '' }
	)
	d.color_value = s.color or { store.default_color_hex }
	d.color = new_color_field(mut cx.Cx, d.color_value)
	cx.peek(d.color.model()).changed = core.listener_of[SessionDialog, string](cx.entity, fn (mut d SessionDialog, v &string, mut _ core.ViewCx[SessionDialog]) {
		d.color_value = *v
	}).any
	d.auth = new_auth_form(mut cx.Cx, 'session-auth', s.auth_method, s.key_path or { '' }, false,
		d.panel.vault)
	mut auth := cx.peek(d.auth.model())
	auth.saved = saved_of(s)
	auth.use_saved(true)
	auth.offers_save = false
	d.group = new_select(mut cx.Cx,
		id:          'session-group'
		options:     existing_group_names(entries)
		value:       store.normalized_group(s.group) or { '' }
		placeholder: 'Select or type group...'
		search_hint: 'Search or type group name...'
		creatable:   true
	)
	d.jump = new_jump_picker(mut cx.Cx, entries, edit, s.jump_host)
	for f in s.forwards {
		d.next_forward++
		d.forwards << new_forward_row(mut cx.Cx, d.next_forward, f)
	}
	if focus_group {
		cx.peek(d.group.model()).focus.focus(mut cx.Cx)
	} else {
		cx.peek(d.label.model()).focus.focus(mut cx.Cx)
	}
}

@[markused]
pub fn (mut d SessionDialog) render(mut cx core.ViewCx[SessionDialog]) core.Element {
	t := kit.theme(cx)
	mut rows := []core.Element{}
	rows << labelled('Label', true, cx.child_view(d.label))
	rows << labelled('Color', false, cx.child_view(d.color))
	rows << labelled('Host', true, cx.child_view(d.host))
	rows << labelled('Port', false, cx.child_view(d.port))
	rows << labelled('Username', false, cx.child_view(d.username))
	rows << cx.child_view(d.auth)
	chevron := if d.advanced { kit.icon_chevron_down } else { kit.icon_chevron_right }
	rows << ui.div().flex_row().child(kit.button('session-advanced').label('Advanced').icon(chevron).ghost().muted().size(.sm).on_click(cx.listener(fn (mut d SessionDialog, _ &core.ClickEvent, mut cx core.ViewCx[SessionDialog]) {
		d.advanced = !d.advanced
		cx.notify()
	})))
	if d.advanced {
		rows << labelled('Jump host', false, cx.child_view(d.jump.select))
		rows << kit.checkbox('session-agent', d.agent).label('Forward the SSH agent to the remote host').on_change(cx.listener(fn (mut d SessionDialog, v &bool, mut cx core.ViewCx[SessionDialog]) {
			d.agent = *v
			cx.notify()
		}))
		rows << d.forward_block(t, mut cx)
	}
	rows << labelled('Group', false, cx.child_view(d.group))
	rows << labelled('Logging', false, kit.radio_group('session-logging', [
		'Use global',
		'On',
		'Off',
	], int(d.logging)).horizontal().on_change(cx.listener(fn (mut d SessionDialog, v &int, mut cx core.ViewCx[SessionDialog]) {
		d.logging = logging_from(*v)
		cx.notify()
	})))
	return ui.div().w_full().on_key_down(cx.listener(fn (mut d SessionDialog, e &core.KeyDownEvent, mut cx core.ViewCx[SessionDialog]) {
		if e.key == .enter {
			d.save(mut cx)
			cx.stop_propagation()
		}
	})).child(dialog_body(t, cx.win.size().height, rows))
}

fn logging_from(i int) store.LoggingOverride {
	return match i {
		1 { .on }
		2 { .off }
		else { .inherit }
	}
}

// footer is rebuilt by the overlay host, so it reads the dialog's current state.
fn (d &SessionDialog) footer() core.Element {
	return ui.div().flex_row().gap(8).child(kit.button('dialog-cancel').label('Cancel').outline().on_click(core.handler_with(d.modal,
		close_modal_click))).child(kit.button('dialog-save').label('Save').primary().on_click(core.listener_of[SessionDialog, core.ClickEvent](d.ent, fn (mut d SessionDialog, _ &core.ClickEvent, mut cx core.ViewCx[SessionDialog]) {
		d.save(mut cx)
	})))
}
