// Jump-host UI shared by the connect, Quick Connect and session dialogs (specs/26): the
// picker over the other saved sessions and one credential block per hop.
module sessionui

import quadra.core
import quadra.kit
import quadra.ui
import store

// HopSpec is the non-secret description of one hop; `saved` is never set for a duplicate.
pub struct HopSpec {
pub:
	label    string
	host     string
	port     int
	username string
	method   store.AuthMethod
	key_path string
	saved    ?SavedPassword
}

fn hop_address(s HopSpec) string {
	return '${s.username}@${s.host}:${s.port}'
}

// hop_spec_of: a saved session as a hop; a hop must carry its username.
fn hop_spec_of(e store.Entry) !HopSpec {
	s := e.session
	username := s.username or { '' }
	if username == '' {
		return error('Jump host "${s.label}" has no username; edit that session first.')
	}
	return HopSpec{
		label:    s.label
		host:     s.host
		port:     s.port
		username: username
		method:   s.auth_method
		key_path: s.key_path or { '' }
		saved:    saved_of(s)
	}
}

// HopForm is one hop's credential block.
pub struct HopForm {
pub:
	spec HopSpec
	form core.View[AuthForm]
}

// build_hop_forms makes one credential form per hop, outermost first.
fn build_hop_forms(mut cx core.Cx, specs []HopSpec, p &SessionPanel) []HopForm {
	mut forms := []HopForm{cap: specs.len}
	for i, spec in specs {
		f := new_auth_form(mut cx, 'hop-${i}', spec.method, spec.key_path, true, p.vault)
		mut a := cx.peek(f.model())
		a.saved = spec.saved
		forms << HopForm{spec, f}
	}
	return forms
}

// hop_for_error: the backend starts a hop's failure with `jump host user@host:port:`. The
// longest matching prefix wins, because an IPv6 address can be another's plus `:<group>`.
fn hop_for_error(hops []HopForm, message string) ?HopForm {
	mut best := -1
	mut found := ?HopForm(none)
	for h in hops {
		prefix := 'jump host ${hop_address(h.spec)}:'
		if message.starts_with(prefix) && prefix.len > best {
			best = prefix.len
			found = h
		}
	}
	return found
}

fn hops_block(hops []HopForm, t &kit.Theme) core.Element {
	mut col := ui.div().flex_col().gap(12).w_full()
	for h in hops {
		block :=
			ui.div().flex_col().gap(8).w_full().pb(12).border_b(1).border_color(t.p.line).child(ui.div().flex_col().child(ui.text('Jump host ${h.spec.label}').size(t.font_size(.sm))).child(ui.text(hop_address(h.spec)).size(t.font_size(.sm)).color(t.p.text_2))).child(core.view_element(h.form.any()))
		col.child(block)
	}
	return col
}

// take_hops collects every hop's credential, outermost first; fields keep their values.
fn take_hops(hops []HopForm, mut cx core.Cx) ![]HopRequest {
	mut out := []HopRequest{cap: hops.len}
	for h in hops {
		mut form := cx.peek(h.form.model())
		cred := form.take_credential(mut cx) or {
			return error('Jump host "${h.spec.label}": ${err.msg()}')
		}
		out << HopRequest{
			label:      h.spec.label
			host:       h.spec.host
			port:       h.spec.port
			username:   h.spec.username
			credential: cred
		}
	}
	return out
}

// JumpPicker is the "Jump host" field: `None` or one of the other saved sessions.
pub struct JumpPicker {
pub mut:
	select core.View[SearchSelect]
	ids    []u64
}

// new_jump_picker: `exclude` is the session being edited; `selected` is ignored when gone.
fn new_jump_picker(mut cx core.Cx, entries []store.Entry, exclude ?u64, selected ?u64) JumpPicker {
	mut ids := []u64{}
	mut labels := []string{}
	for e in entries {
		if exclude != none && exclude or { 0 } == e.id {
			continue
		}
		s := e.session
		ids << e.id
		labels << '${s.label}  (${s.username or { '?' }}@${s.host}:${s.port})'
	}
	mut value := ''
	if sel := selected {
		i := ids.index(sel)
		if i >= 0 {
			value = labels[i]
		}
	}
	return JumpPicker{
		select: new_select(mut cx,
			id:          'jump-host'
			options:     labels
			value:       value
			placeholder: 'None'
			search_hint: 'Search sessions...'
		)
		ids:    ids
	}
}

fn (j JumpPicker) selected(cx core.Cx) ?u64 {
	s := cx.peek(j.select.model())
	i := s.options.index(s.value)
	return if i >= 0 && i < j.ids.len { j.ids[i] } else { none }
}
