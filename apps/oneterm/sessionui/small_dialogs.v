// The rename-group form and the two danger confirmations (specs/26).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

fn kit_close(mut cx core.Cx, id int) {
	kit.close_modal(mut cx, id)
}

// RenameDialog: all sessions of a group move to the new name; an empty name is refused.
@[heap]
pub struct RenameDialog {
pub mut:
	panel &SessionPanel = unsafe { nil }
	old   string
	name  core.View[input.TextInput]
	modal int
	ent   u32
}

fn (mut p SessionPanel) open_rename_group(mut cx core.Cx, group string) {
	if p.read_only {
		return
	}
	v := cx.new_view(RenameDialog{
		panel: unsafe { p }
		old:   group
	})
	cx.update_view(v, fn [group] (mut d RenameDialog, mut cx core.ViewCx[RenameDialog]) {
		d.ent = cx.entity
		d.name = input.new_input(mut cx.Cx,
			id:          'group-name'
			placeholder: 'Group name'
			value:       group
		)
		cx.peek(d.name.model()).focus.focus(mut cx.Cx)
	})
	mut d := cx.peek(v.model())
	d.modal = kit.open_modal(mut cx, kit.ModalOptions{
		title:  'Rename Group'
		width:  440
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

@[markused]
pub fn (mut d RenameDialog) render(mut cx core.ViewCx[RenameDialog]) core.Element {
	t := kit.theme(cx)
	return ui.div().w_full().on_key_down(cx.listener(fn (mut d RenameDialog, e &core.KeyDownEvent, mut cx core.ViewCx[RenameDialog]) {
		if e.key == .enter {
			d.save(mut cx)
			cx.stop_propagation()
		}
	})).child(dialog_body(t, cx.win.size().height, [
		labelled('Group Name', true, cx.child_view(d.name)),
	]))
}

fn (d &RenameDialog) footer() core.Element {
	return ui.div().flex_row().gap(8).child(kit.button('dialog-cancel').label('Cancel').outline().on_click(core.handler_with(d.modal,
		close_modal_click))).child(kit.button('dialog-save').label('Save').primary().on_click(core.listener_of[RenameDialog, core.ClickEvent](d.ent, fn (mut d RenameDialog, _ &core.ClickEvent, mut cx core.ViewCx[RenameDialog]) {
		d.save(mut cx)
	})))
}

fn (mut d RenameDialog) save(mut cx core.ViewCx[RenameDialog]) {
	name := cx.peek(d.name.model()).value().trim_space()
	if name == '' {
		warn(mut cx.Cx, 'Group name cannot be empty.')
		return
	}
	mut p := d.panel
	p.sessions.rename_group(d.old, name)
	p.refresh(mut cx.Cx)
	success(mut cx.Cx, 'Group renamed.')
	kit_close(mut cx.Cx, d.modal)
}

// DangerConfirm is a Cancel plus danger-styled confirm alert (delete, trust host key).
pub struct DangerData {
pub:
	panel &SessionPanel = unsafe { nil }
	kind  int // 0 delete, 1 trust
	id    u64 // the session or the attempt
	modal &ModalRef = unsafe { nil }
}

fn (mut p SessionPanel) open_danger(mut cx core.Cx, title string, lines []string, ok string, kind int, id u64) int {
	mut modal := &ModalRef{}
	data := DangerData{unsafe { p }, kind, id, modal}
	// An alert: its lines are the muted description under the title, Cancel the default
	// button; no close button, outside clicks ignored.
	opened := kit.open_modal(mut cx, kit.ModalOptions{
		title:       title
		width:       448
		description: lines.join('\n')
		alert:       true
		footer:      fn [data, ok] () core.Element {
			return ui.div().flex_row().gap(8).child(kit.button('danger-cancel').label('Cancel').on_click(core.handler_with(data,
				danger_cancel))).child(kit.button('danger-ok').label(ok).danger().on_click(core.handler_with(data,
				danger_ok)))
		}
	})
	modal.id = opened
	p.track(opened, unsafe { nil })
	return opened
}

fn danger_cancel(d DangerData, _ &core.ClickEvent, mut cx core.Cx) {
	kit.close_modal(mut cx, d.modal.id)
}

fn danger_ok(d DangerData, _ &core.ClickEvent, mut cx core.Cx) {
	kit.close_modal(mut cx, d.modal.id)
	mut p := d.panel
	if d.kind == 0 {
		p.do_delete(mut cx, d.id)
	} else {
		p.trust_host_key(mut cx, d.id)
	}
}

fn delete_confirmation(label string) string {
	return 'Delete the saved SSH session "${label}"? This cannot be undone.'
}

fn (mut p SessionPanel) confirm_delete(mut cx core.Cx, id u64) {
	s := p.sessions.get(id) or { return }
	p.open_danger(mut cx, 'Delete SSH Session', [delete_confirmation(s.label)], 'Delete', 0, id)
}

fn (mut p SessionPanel) do_delete(mut cx core.Cx, id u64) {
	p.sessions.remove(id)
	p.refresh(mut cx)
	success(mut cx, 'SSH session deleted.')
}

// host_key_lines is the Unknown SSH Host Key text.
fn host_key_lines(host string, port int, via string, algorithm string, fingerprint string) []string {
	return [
		'The server is not present in your OpenSSH known_hosts file.',
		'',
		'Host: ${host}:${port}${via}',
		'Algorithm: ${algorithm}',
		'SHA-256 fingerprint: ${fingerprint}',
		'',
		'Verify this fingerprint through a trusted channel before continuing.',
	]
}

// ModalRef lets a dialog's buttons know its modal id (closures capture by value).
@[heap]
struct ModalRef {
mut:
	id int
}
