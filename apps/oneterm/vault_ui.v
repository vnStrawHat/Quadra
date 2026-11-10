// The master-password dialogs and the vault job protocol (specs/22, specs/25). Argon2id runs on
// a worker owned by the OneTerm view; a job's result is applied there whatever became of the
// dialog that started it.
module main

import vault
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

const irreversible_warning = 'OneTerm cannot recover this password. If you forget it, the only way back is Reset, which deletes every saved password. Your sessions are kept.'

const reset_confirmation = 'RESET'

const wrong_password = 'Wrong master password.' // vault's message for a failed attempt

// strength_hint follows the length only, as in the original; '' while the field is empty.
fn strength_hint(chars int) string {
	return match true {
		chars == 0 { '' }
		chars < vault.min_password_chars { 'Too short: use at least ${vault.min_password_chars} characters.' }
		chars < 12 { 'Strength: fair. A longer password is stronger.' }
		chars < 16 { 'Strength: good.' }
		else { 'Strength: strong.' }
	}
}

// validate_new_password compares exactly as typed: no trimming, no normalisation.
fn validate_new_password(password string, confirmation string) ?string {
	if password.runes().len < vault.min_password_chars {
		return 'Use at least ${vault.min_password_chars} characters.'
	}
	if password != confirmation {
		return 'The passwords do not match.'
	}
	return none
}

// VaultDialog is the form in one master-password modal.
pub struct VaultDialog {
pub mut:
	one    core.Model[OneTerm]
	kind   DialogKind
	fields []core.View[input.TextInput]
	error  string
	busy   bool
	modal  int
}

// open_vault_dialog shows one of the four dialogs on the current window.
fn open_vault_dialog(kind DialogKind, one core.Model[OneTerm], mut cx core.Cx) {
	mut fields := []core.View[input.TextInput]{}
	for i, f in kind.fields() {
		fields << input.new_input(mut cx, id: 'vault-field-${i}', password: f.masked)
	}
	view := cx.new_view(VaultDialog{
		one:    one
		kind:   kind
		fields: fields
	})
	id := kit.open_modal(mut cx, kit.ModalOptions{
		title:     kind.title()
		width:     480
		body_with: kit.view_body(view.any())
	})
	mut d := cx.peek(view.model())
	d.modal = id
	for f in fields {
		input.on_change(mut cx, f, core.listener_of[VaultDialog, string](view.id, fn (mut d VaultDialog, _ &string, mut cx core.ViewCx[VaultDialog]) {
			cx.notify()
		}))
		input.on_submit(mut cx, f, core.listener_of[VaultDialog, string](view.id, fn (mut d VaultDialog, _ &string, mut cx core.ViewCx[VaultDialog]) {
			d.submit(mut cx)
		}))
	}
	cx.peek(fields[0].model()).focus.focus(mut cx)
}

fn (d &VaultDialog) value(i int, cx &core.Cx) string {
	return cx.peek(d.fields[i].model()).value()
}

// clear_field drops rejected text from the input.
fn (d &VaultDialog) clear_field(i int, mut cx core.Cx) {
	input.set_value(mut cx, d.fields[i], '')
}

fn (mut d VaultDialog) submit(mut cx core.ViewCx[VaultDialog]) {
	mut o := cx.peek(d.one)
	if d.busy || o.vault.is_busy() {
		return
	}
	if d.kind in [.unlock, .change] && o.vault.retry_after_ms() > 0 {
		return
	}
	job := d.begin(mut o, mut cx) or {
		if err.msg() != '' {
			d.error = err.msg()
		}
		cx.notify()
		return
	}
	d.busy, d.error = true, ''
	o.vault_dialog = cx.self_model()
	cx.notify()
	o.start_vault_job(job, d.one, mut cx.Cx)
}

// begin validates the form and asks the state for a job.
fn (mut d VaultDialog) begin(mut o OneTerm, mut cx core.ViewCx[VaultDialog]) !vault.VaultJob {
	match d.kind {
		.set_up {
			if msg := validate_new_password(d.value(0, &cx.Cx), d.value(1, &cx.Cx)) {
				return error(msg)
			}
			return o.vault.begin_set_up(d.value(0, &cx.Cx))
		}
		.unlock {
			return o.vault.begin_unlock(d.value(0, &cx.Cx))
		}
		.change {
			if msg := validate_new_password(d.value(1, &cx.Cx), d.value(2, &cx.Cx)) {
				return error(msg)
			}
			return o.vault.begin_change(d.value(0, &cx.Cx), d.value(1, &cx.Cx))
		}
		.reset {
			if d.value(0, &cx.Cx).trim_space() != reset_confirmation {
				return error('Type ${reset_confirmation} to confirm.')
			}
			job := o.vault.begin_reset()!
			o.drain_vault_events(mut cx.Cx) // sealed values go before the file does
			return job
		}
	}
}

@[markused]
pub fn (mut d VaultDialog) render(mut cx core.ViewCx[VaultDialog]) core.Element {
	t := kit.theme(cx)
	o := cx.peek(d.one)
	wait := if d.kind in [.unlock, .change] { o.vault.retry_after_ms() } else { i64(0) }
	if wait > 0 {
		cx.request_animation_frame() // the countdown
	}
	mut col :=
		ui.div().test_id('vault-form').flex_col().gap(12).w_full().child(ui.text(d.kind.intro()).size(t.font_size(.sm)).color(t.p.text_2))
	for i, f in d.kind.fields() {
		mut field :=
			ui.div().flex_col().gap(4).w_full().child(ui.text(f.label).size(t.font_size(.sm))).child(ui.div().flex_col().w_full().child(core.view_element(d.fields[i].any())))
		if i == d.kind.new_password_field() {
			hint := strength_hint(d.value(i, &cx.Cx).runes().len)
			if hint != '' {
				field.child(ui.text(hint).size(11).color(t.p.text_2))
			}
		}
		col.child(field)
	}
	if d.kind.new_password_field() >= 0 {
		col.child(ui.text(irreversible_warning).size(t.font_size(.sm)).color(t.p.warn))
	}
	if d.error != '' {
		col.child(ui.text(d.error).size(t.font_size(.sm)).color(t.p.error))
	}
	if wait > 0 {
		col.child(ui.text(vault.retry_message(wait)).size(t.font_size(.sm)).color(t.p.error))
	}
	mut confirm := kit.button('vault-confirm').label(d.kind.confirm_label()).disabled(d.busy
		|| wait > 0).on_click(cx.listener(fn (mut d VaultDialog, _ &core.ClickEvent, mut cx core.ViewCx[VaultDialog]) {
		d.submit(mut cx)
	}))
	if d.kind == .reset {
		confirm.danger()
	} else {
		confirm.primary()
	}
	col.child(ui.div().flex_row().justify_end().gap(8).child(kit.button('vault-cancel').label('Cancel').ghost().on_click(cx.listener(fn (mut d VaultDialog, _ &core.ClickEvent, mut cx core.ViewCx[VaultDialog]) {
		kit.close_modal(mut cx.Cx, d.modal)
	}))).child(confirm))
	return col
}

// start_vault_job runs a job on a worker owned by the OneTerm view: the settings window that
// started it may close before it ends.
fn (mut o OneTerm) start_vault_job(job vault.VaultJob, m core.Model[OneTerm], mut cx core.Cx) {
	cx.update_with(m, job, fn (mut o OneTerm, job vault.VaultJob, mut cx core.ViewCx[OneTerm]) {
		core.spawn_task_with[OneTerm, vault.VaultJob, vault.JobResult](mut cx, job, run_vault_job, fn (mut o OneTerm, r &core.TaskResult[vault.JobResult], mut cx core.ViewCx[OneTerm]) {
			o.finish_vault(r, mut cx)
		})
	})
}

// run_vault_job is the work of a vault task. Jobs carry the password: they are passed as
// task data, never captured (a closure is never freed, nor what it holds).
fn run_vault_job(job vault.VaultJob, _ &core.TaskCx) !vault.JobResult {
	return job.run()
}

// finish_vault applies a job and reports to the dialog or the lock screen that waits for it.
fn (mut o OneTerm) finish_vault(r &core.TaskResult[vault.JobResult], mut cx core.ViewCx[OneTerm]) {
	mut message := ''
	if r.ok {
		o.vault.apply(r.value) or { message = err.msg() }
	} else {
		message = r.error
	}
	o.drain_vault_events(mut cx.Cx)
	if job := o.vault.take_followup() {
		o.start_vault_job(job, cx.self_model(), mut cx.Cx)
	}
	if o.vault_dialog.id != 0 {
		o.finish_dialog(message, mut cx)
		o.vault_dialog = core.Model[VaultDialog]{}
	} else {
		o.finish_lock(message, mut cx)
	}
	cx.notify()
}

fn (mut o OneTerm) finish_dialog(message string, mut cx core.ViewCx[OneTerm]) {
	mut d := cx.peek(o.vault_dialog)
	d.busy = false
	if message == '' {
		kit.close_modal(mut cx.Cx, d.modal)
	} else {
		if message == wrong_password {
			d.clear_field(0, mut cx.Cx) // the rejected text leaves the input
		}
		d.error = message
		cx.touch(o.vault_dialog)
	}
}

// drain_vault_events reacts to the state's events; Reset comes before the file is removed.
fn (mut o OneTerm) drain_vault_events(mut cx core.Cx) {
	for e in o.vault.take_events() {
		if e == .reset && o.sessions != unsafe { nil } {
			o.sessions.clear_sealed_passwords()
			if o.sess_view.id != 0 {
				cx.peek(o.sess_view.model()).refresh(mut cx)
			}
		}
	}
}

// vault_status_text is the Status row, per phase.
fn vault_status_text(o &OneTerm) string {
	base := match o.vault.phase() {
		.uninitialised { 'Not set up.' }
		.locked { 'Set, locked. Saved passwords need it before they can be used.' }
		.unlocked { 'Set, unlocked.' }
		.unavailable { o.vault.unavailable_reason() }
	}

	return if o.vault.is_read_only() {
		'${base} Managed from the normal OneTerm window.'
	} else {
		base
	}
}
