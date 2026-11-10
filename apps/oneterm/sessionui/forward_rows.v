// The "Port forwards" rows of the session dialog (specs/26): one editable row per forward.
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui
import store

const forward_kinds = ['Local', 'Remote', 'Dynamic']

@[heap]
pub struct ForwardRow {
pub mut:
	id          int
	kind        core.View[SearchSelect]
	bind        core.View[input.TextInput]
	bind_port   core.View[input.TextInput]
	target_host core.View[input.TextInput]
	target_port core.View[input.TextInput]
}

// is_ip accepts a dotted quad or a colon-hex address. No resolver call: a bind address never
// needs DNS. ponytail: IPv6 is shape-checked only (hex digits and colons).
fn is_ip(s string) bool {
	if s.contains(':') {
		return s.count(':') >= 2 && s.contains_only('0123456789abcdefABCDEF:.')
	}
	parts := s.split('.')
	return parts.len == 4 && parts.all(it != '' && it.len <= 3 && it.contains_only('0123456789')
		&& it.int() <= 255)
}

fn (r &ForwardRow) kind_index(cx core.Cx) int {
	return forward_kinds.index(cx.peek(r.kind.model()).value)
}

fn (r &ForwardRow) text(cx core.Cx, v core.View[input.TextInput]) string {
	return cx.peek(v.model()).value().trim_space()
}

fn forward_port(cx core.Cx, r &ForwardRow, v core.View[input.TextInput], name string) !int {
	t := r.text(cx, v)
	if t == '' || !t.contains_only('0123456789') || t.len > 5 || t.int() > 65535 {
		return error('Port forward: the ${name} port must be a number 0..65535.')
	}
	return t.int()
}

// parse reads one row; the error is the message shown to the user.
fn (r &ForwardRow) parse(cx core.Cx) !store.PortForward {
	kind := r.kind_index(cx)
	bind := r.text(cx, r.bind)
	if kind != 1 && !is_ip(bind) {
		return error('Port forward: the bind address must be an IP address.')
	}
	bind_port := forward_port(cx, r, r.bind_port, 'listening')!
	mut f := store.PortForward{
		kind:      unsafe { store.ForwardKind(kind) }
		bind:      bind
		bind_port: bind_port
	}
	if kind != 2 {
		f = store.PortForward{
			...f
			target_host: r.text(cx, r.target_host)
			target_port: forward_port(cx, r, r.target_port, 'target')!
		}
	}
	if problem := f.validate() {
		return error('Port forward ${f.summary()}: ${forward_problem(problem)}.')
	}
	return f
}

fn forward_problem(e store.ForwardError) string {
	return match e {
		.bind_port { 'the listening port cannot be 0' }
		.target_port { 'the target port cannot be 0' }
		.empty_host { 'the host is empty' }
		.whitespace { 'the host contains whitespace' }
	}
}

// ForwardError names the row a refused forward is about.
pub struct ForwardFailure {
pub:
	row     int
	message string
}

// new_forward_row builds one row, from `f` or empty.
fn new_forward_row(mut cx core.Cx, id int, f ?store.PortForward) &ForwardRow {
	mut value := store.PortForward{}
	if p := f {
		value = p
	}
	mut kind_index := int(value.kind)
	if f == none {
		kind_index = 0
	}
	mut r := &ForwardRow{
		id: id
	}
	r.kind = new_select(mut cx,
		id:         'forward-kind-${id}'
		options:    forward_kinds
		value:      forward_kinds[kind_index]
		searchable: false
		clearable:  false
	)
	r.bind = input.new_input(mut cx,
		id:          'forward-bind-${id}'
		placeholder: '127.0.0.1'
		value:       value.bind
	)
	r.bind_port = input.new_input(mut cx,
		id:          'forward-port-${id}'
		placeholder: 'port'
		value:       if value.bind_port == 0 { '' } else { value.bind_port.str() }
	)
	r.target_host = input.new_input(mut cx,
		id:          'forward-host-${id}'
		placeholder: 'target host'
		value:       value.target_host
	)
	r.target_port = input.new_input(mut cx,
		id:          'forward-target-port-${id}'
		placeholder: 'port'
		value:       if value.target_port == 0 { '' } else { value.target_port.str() }
	)
	return r
}

fn (d &SessionDialog) forward_block(t &kit.Theme, mut cx core.ViewCx[SessionDialog]) core.Element {
	mut rows := ui.div().flex_col().gap(8).w_full()
	for r in d.forwards {
		rows.child(forward_row_view(r, t, mut cx))
	}
	add := kit.button('forward-add').label('Add').icon(kit.icon_plus).ghost().size(.sm).on_click(cx.listener(fn (mut d SessionDialog, _ &core.ClickEvent, mut cx core.ViewCx[SessionDialog]) {
		d.next_forward++
		d.forwards << new_forward_row(mut cx.Cx, d.next_forward, none)
		cx.notify()
	}))
	return labelled('Port forwards', false,
		ui.div().flex_col().gap(8).w_full().child(rows).child(ui.div().flex_row().child(add)))
}

fn forward_row_view(r &ForwardRow, t &kit.Theme, mut cx core.ViewCx[SessionDialog]) core.Element {
	kind := r.kind_index(cx.Cx)
	has_target := kind != 2
	bind := cx.peek(r.bind.model()).value().trim_space()
	exposed := bind != '' && bind !in ['127.0.0.1', '::1', 'localhost'] && !(is_ip(bind)
		&& bind.starts_with('127.'))
	mut row := ui.div().flex_row().gap(4).w_full().items_center()
	row.child(ui.div().w(104).shrink_0().flex_col().child(core.view_element(r.kind.any())))
	row.child(ui.div().grow().min_w(0).flex_col().child(core.view_element(r.bind.any())))
	row.child(ui.div().w(64).shrink_0().flex_col().child(core.view_element(r.bind_port.any())))
	if has_target {
		row.child(ui.text('->').color(t.p.text_2))
		row.child(ui.div().grow().min_w(0).flex_col().child(core.view_element(r.target_host.any())))
		row.child(ui.div().w(64).shrink_0().flex_col().child(core.view_element(r.target_port.any())))
	} else {
		row.child(ui.text('SOCKS5').size(t.font_size(.sm)).color(t.p.text_2))
	}
	row.child(kit.icon_button('forward-remove-${r.id}', kit.icon_x).ghost().size(.xs).on_click(core.listener_with[SessionDialog, int, core.ClickEvent](cx.entity,
		r.id, fn (mut d SessionDialog, id int, _ &core.ClickEvent, mut cx core.ViewCx[SessionDialog]) {
		d.forwards = d.forwards.filter(it.id != id)
		cx.notify()
	})))
	mut col := ui.div().flex_col().gap(4).w_full().child(row)
	if exposed {
		col.child(ui.text('This listener is reachable from other machines.').size(t.font_size(.sm)).color(t.p.warn))
	}
	return col
}

// take_forwards parses every row; the first invalid row or duplicate listener is the error.
fn (d &SessionDialog) take_forwards(cx core.Cx) ![]store.PortForward {
	mut forwards := []store.PortForward{}
	for r in d.forwards {
		forwards << r.parse(cx)!
	}
	for i, f in forwards {
		if forwards[..i].any(it.bind_key() == f.bind_key()) {
			return error('Port forward ${f.summary()}: the same listener is used twice.')
		}
	}
	return forwards
}
