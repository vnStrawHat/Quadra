// The master-password prompts saved SSH passwords trigger: set up and unlock (specs/26).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui
import vault

const read_only_note = 'Managed from the normal OneTerm window.'
const irreversible_warning = 'OneTerm cannot recover this password. If you forget it, the only way back is Reset, which deletes every saved password. Your sessions are kept.'

enum VaultKind {
	set_up
	unlock
}

fn (k VaultKind) title() string {
	return match k {
		.set_up { 'Create a master password' }
		.unlock { 'Unlock saved passwords' }
	}
}

fn (k VaultKind) intro() string {
	return match k {
		.set_up { 'OneTerm encrypts the passwords you save with it. Choose a long one: anyone with a copy of your OneTerm folder can try to guess it.' }
		.unlock { 'Enter your master password to use your saved passwords.' }
	}
}

@[heap]
pub struct VaultDialog {
pub mut:
	kind    VaultKind
	inputs  []core.View[input.TextInput]
	error   string
	busy    bool
	ticking bool
	vault   &vault.VaultState = unsafe { nil }
	panel   &SessionPanel     = unsafe { nil }
	ready   core.Listener[bool] // what ensure_unlocked runs once unlocked
	ent     u32
	modal   int
}

// ensure_unlocked runs `ready` (a listener, invoked with true) once the vault is unlocked: at
// once when it is, after the Unlock or Set up dialog when not. An unavailable vault does
// nothing; the caller says why.
fn (mut p SessionPanel) ensure_unlocked(mut cx core.Cx, ready core.Listener[bool]) {
	if p.vault.is_screen_locked() {
		return
	}
	match p.vault.phase() {
		.unlocked {
			run_ready(mut cx, ready)
		}
		.locked {
			p.open_vault_dialog(mut cx, .unlock, ready)
		}
		.uninitialised {
			if p.vault.is_read_only() {
				warn(mut cx, 'No master password is set up. ${read_only_note}')
				return
			}
			p.open_vault_dialog(mut cx, .set_up, ready)
		}
		.unavailable {}
	}
}

fn run_ready(mut cx core.Cx, ready core.Listener[bool]) {
	yes := true
	cx.app.invoke(ready.any, &yes, cx.win, true)
}

fn (mut p SessionPanel) open_vault_dialog(mut cx core.Cx, kind VaultKind, ready core.Listener[bool]) {
	v := cx.new_view(VaultDialog{
		kind:  kind
		vault: p.vault
		panel: unsafe { p }
		ready: ready
	})
	cx.update_view(v, fn (mut d VaultDialog, mut cx core.ViewCx[VaultDialog]) {
		d.init(mut cx)
	})
	mut d := cx.peek(v.model())
	d.modal = kit.open_modal(mut cx, kit.ModalOptions{
		title:       kind.title()
		width:       440
		body_with:   kit.view_body(v.any())
		footer_with: kit.build_with(VaultRef{d}, fn (r VaultRef, _ &kit.Theme) core.Element {
			return r.d.footer()
		})
	})
	p.track(d.modal, v.any())
}

struct VaultRef {
	d &VaultDialog = unsafe { nil }
}

fn (mut d VaultDialog) init(mut cx core.ViewCx[VaultDialog]) {
	d.ent = cx.entity
	labels := if d.kind == .set_up {
		['Master password', 'Confirm master password']
	} else {
		['Master password']
	}
	for i, _ in labels {
		mut v := input.new_input(mut cx.Cx, id: 'vault-input-${i}', password: true)
		input.on_change(mut cx.Cx, v, cx.listener(fn (mut d VaultDialog, _ &string, mut cx core.ViewCx[VaultDialog]) {
			d.error = ''
			cx.notify()
		}))
		d.inputs << v
	}
	cx.peek(d.inputs[0].model()).focus.focus(mut cx.Cx)
	d.schedule_tick(mut cx)
}

fn (d &VaultDialog) retry_after() i64 {
	return if d.kind == .unlock { d.vault.retry_after_ms() } else { 0 }
}

fn (mut d VaultDialog) schedule_tick(mut cx core.ViewCx[VaultDialog]) {
	if d.ticking || d.retry_after() <= 0 {
		return
	}
	d.ticking = true
	cx.set_timeout(500, fn (mut d VaultDialog, mut cx core.ViewCx[VaultDialog]) {
		d.ticking = false
		d.schedule_tick(mut cx)
		cx.notify()
		cx.touch_global[kit.Overlays]()
	})
}

fn (d &VaultDialog) value(i int, cx core.Cx) string {
	return cx.peek(d.inputs[i].model()).value()
}

fn (mut d VaultDialog) submit(mut cx core.ViewCx[VaultDialog]) {
	if d.busy || d.retry_after() > 0 {
		return
	}
	password := d.value(0, cx.Cx)
	if d.kind == .set_up {
		validate_new_password(password, d.value(1, cx.Cx)) or {
			d.error = err.msg()
			cx.notify()
			return
		}
	}
	mut job := vault.VaultJob{}
	if d.kind == .set_up {
		job = d.vault.begin_set_up(password) or {
			d.show_begin_error(err, mut cx)
			return
		}
	} else {
		job = d.vault.begin_unlock(password) or {
			d.show_begin_error(err, mut cx)
			return
		}
	}
	d.busy = true
	d.error = ''
	cx.touch_global[kit.Overlays]()
	cx.notify()
	core.spawn_task_with[VaultDialog, vault.VaultJob, vault.JobResult](mut cx, job, run_vault_job, fn (mut d VaultDialog, r &core.TaskResult[vault.JobResult], mut cx core.ViewCx[VaultDialog]) {
		d.finish(r, mut cx)
	})
}

// run_vault_job is the work of a vault task. Jobs carry the password: they are passed as
// task data, never captured (a closure is never freed, nor what it holds).
fn run_vault_job(job vault.VaultJob, _ &core.TaskCx) !vault.JobResult {
	return job.run()
}

fn (mut d VaultDialog) show_begin_error(err IError, mut cx core.ViewCx[VaultDialog]) {
	if err is vault.VaultActionError && err.kind == .retry_in {
		d.error = ''
		d.schedule_tick(mut cx)
	} else {
		d.error = err.msg()
	}
	cx.notify()
}

fn (mut d VaultDialog) finish(r &core.TaskResult[vault.JobResult], mut cx core.ViewCx[VaultDialog]) {
	d.busy = false
	cx.touch_global[kit.Overlays]()
	if !r.ok {
		d.error = r.error
		cx.notify()
		return
	}
	d.vault.apply(r.value) or {
		if err is vault.VaultActionError {
			if err.kind == .locked_meanwhile {
				for v in d.inputs {
					input.set_value(mut cx.Cx, v, '')
				}
			}
			if err.kind == .vault && err.vault.kind == .wrong_password {
				input.set_value(mut cx.Cx, d.inputs[0], '')
			}
			d.show_begin_error(err, mut cx)
		} else {
			d.error = err.msg()
			cx.notify()
		}
		d.schedule_tick(mut cx)
		return
	}
	d.run_followup(mut cx)
	kit.close_modal(mut cx.Cx, d.modal)
	mut p := d.panel
	p.refresh(mut cx.Cx)
	if d.ready.any.call != unsafe { nil } {
		run_ready(mut cx.Cx, d.ready)
	}
}

// run_followup applies the rehash an unlock scheduled, on the panel so it outlives the dialog.
fn (mut d VaultDialog) run_followup(mut cx core.ViewCx[VaultDialog]) {
	job := d.vault.take_followup() or { return }
	mut p := d.panel
	cx.update_with(p.view.model(), job, fn (mut p SessionPanel, job vault.VaultJob, mut cx core.ViewCx[SessionPanel]) {
		core.spawn_task_with[SessionPanel, vault.VaultJob, vault.JobResult](mut cx, job,
			run_vault_job, fn (mut p SessionPanel, r &core.TaskResult[vault.JobResult], mut cx core.ViewCx[SessionPanel]) {
			if r.ok {
				p.vault.apply(r.value) or {}
			}
		})
	})
}

@[markused]
pub fn (mut d VaultDialog) render(mut cx core.ViewCx[VaultDialog]) core.Element {
	t := kit.theme(cx)
	wait := d.retry_after()
	mut rows := []core.Element{}
	rows << ui.text(d.kind.intro()).size(t.font_size(.sm)).color(t.p.text_2)
	labels := if d.kind == .set_up {
		['Master password', 'Confirm master password']
	} else {
		['Master password']
	}
	for i, label in labels {
		rows << labelled(label, true, cx.child_view(d.inputs[i]))
		if d.kind == .set_up && i == 0 {
			hint := strength_hint(d.value(0, cx.Cx).runes().len)
			if hint != '' {
				rows << ui.text(hint).size(t.font_size(.xs)).color(t.p.text_2)
			}
		}
	}
	if d.kind == .set_up {
		rows << ui.text(irreversible_warning).size(t.font_size(.sm)).color(t.p.warn)
	}
	if d.error != '' {
		rows << ui.text(d.error).size(t.font_size(.sm)).color(t.p.error)
	}
	if wait > 0 {
		rows << ui.text(vault.retry_message(wait)).size(t.font_size(.sm)).color(t.p.error)
	}
	return ui.div().w_full().on_key_down(cx.listener(fn (mut d VaultDialog, e &core.KeyDownEvent, mut cx core.ViewCx[VaultDialog]) {
		if e.key == .enter {
			d.submit(mut cx)
			cx.stop_propagation()
		}
	})).child(dialog_body(t, cx.win.size().height, rows))
}

fn (d &VaultDialog) footer() core.Element {
	label := if d.kind == .set_up { 'Create' } else { 'Unlock' }
	return ui.div().flex_row().gap(8).child(kit.button('vault-cancel').label('Cancel').outline().on_click(core.handler_with(d.modal,
		close_modal_click))).child(kit.button('vault-confirm').label(label).primary().loading(d.busy).disabled(
		d.busy || d.retry_after() > 0).on_click(core.listener_of[VaultDialog, core.ClickEvent](d.ent, fn (mut d VaultDialog, _ &core.ClickEvent, mut cx core.ViewCx[VaultDialog]) {
		d.submit(mut cx)
	})))
}
