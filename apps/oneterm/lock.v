// The lock screen (specs/22).
module main

import os
import time
import vault
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

// lock_refusal says why there is nothing to lock with.
fn lock_refusal(reason string, read_only bool) string {
	if reason != '' {
		return 'OneTerm cannot lock: ${reason}'
	}
	if read_only {
		return 'No master password is set up. Managed from the normal OneTerm window.'
	}
	return 'Set up a master password in Settings > Security first: the lock screen asks for it.'
}

fn (mut o OneTerm) lock_now(mut cx core.ViewCx[OneTerm]) {
	if !o.vault.can_lock_screen() {
		kit.notify(mut cx.Cx, .info, lock_refusal(o.vault.unavailable_reason(),
			o.vault.is_read_only()), '')
		return
	}
	o.lock_screen(mut cx)
}

fn (mut o OneTerm) lock_screen(mut cx core.ViewCx[OneTerm]) {
	o.vault.lock_screen()
	o.drain_vault_events(mut cx.Cx)
	o.lock_error = ''
	if o.lock_input.id != 0 {
		input.set_value(mut cx.Cx, o.lock_input, '')
	}
	cx.notify()
}

// tick_lock locks after the idle timeout, whichever clock noticed first.
fn (mut o OneTerm) tick_lock(mut cx core.ViewCx[OneTerm]) {
	if o.vault.is_screen_locked() || !o.vault.can_lock_screen() {
		return
	}
	if time.since(o.last_input).seconds() >= f64(o.ui.lock_timeout_minutes() * 60) {
		o.lock_screen(mut cx)
	}
}

// try_unlock starts a job against the vault header held in memory.
fn (mut o OneTerm) try_unlock(password string, mut cx core.ViewCx[OneTerm]) {
	if password == '' || o.vault.retry_after_ms() > 0 || o.vault.is_busy() {
		return
	}
	job := o.vault.begin_unlock_screen(password) or {
		o.lock_error = err.msg()
		cx.notify()
		return
	}
	o.lock_error = ''
	o.vault_dialog = core.Model[VaultDialog]{}
	o.start_vault_job(job, cx.self_model(), mut cx.Cx)
	cx.notify()
}

// finish_lock ends an unlock from the lock screen.
fn (mut o OneTerm) finish_lock(message string, mut cx core.ViewCx[OneTerm]) {
	o.lock_error = message
	input.set_value(mut cx.Cx, o.lock_input, '')
	if message == '' {
		o.last_input = time.now()
		o.focus_active(mut cx)
	}
}

fn (mut o OneTerm) lock_view(t &kit.Theme, mut cx core.ViewCx[OneTerm]) core.Element {
	if o.lock_input.id == 0 {
		o.lock_input = input.new_input(mut cx.Cx,
			id:       'lock-input'
			password: true
		)
		input.on_submit(mut cx.Cx, o.lock_input, cx.listener(fn (mut o OneTerm, v &string, mut cx core.ViewCx[OneTerm]) {
			o.try_unlock(*v, mut cx)
		}))
		cx.peek(o.lock_input.model()).focus.focus(mut cx.Cx)
	}
	folder := os.abs_path(o.dir)
	wait := o.vault.retry_after_ms()
	msg := if wait > 0 { vault.retry_message(wait) } else { o.lock_error }
	unlock := kit.button('lock-unlock').label('Unlock').primary().loading(o.vault.is_busy()).disabled(
		wait > 0 || o.vault.is_busy()).on_click(cx.listener(fn (mut o OneTerm, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
		v := cx.peek(o.lock_input.model()).value()
		o.try_unlock(v, mut cx)
	}))
	quit := kit.button('lock-quit').label('Quit OneTerm').outline().on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
		cx.app.quit()
	}))
	// Up to 420 wide; only the icon and title are centred, the buttons sit at the right.
	head :=
		ui.div().flex_col().items_center().gap(t.sp(8)).child(kit.icon(kit.icon_lock).size(t.sp(32)).color(t.p.text_2)).child(ui.text('OneTerm is locked').size(t.scaled(t.text_xl)))
	mut form :=
		ui.div().flex_col().gap(t.sp(12)).w_full().max_w(420).child(head).child(ui.text('Master password').size(t.font_size(.sm)).color(t.p.text_2)).child(ui.div().flex_col().w_full().child(core.view_element(o.lock_input.any())))
	if msg != '' {
		form.child(ui.text(msg).size(t.font_size(.sm)).color(t.p.error))
	}
	form.child(ui.div().flex_row().justify_end().child(unlock)).child(ui.div().pt(t.sp(16)).child(ui.text('Forgot it? The lock screen cannot reset the master password. Quit OneTerm, then delete vault.json in the folder "${folder}". OneTerm then starts without a lock, and every saved password is lost. Your saved sessions are kept; type their passwords again.').size(t.font_size(.xs)).color(t.p.text_2))).child(ui.div().flex_row().justify_end().child(quit))
	// The title bar names the window (and an elevated one), at the small text size.
	name := ui.div().flex_row().pl(t.sp(12)).gap(t.sp(4)).text_size(t.font_size(.sm)).child(ui.text(
		'OneTerm' + elevation_suffix(elevation)))
	return ui.div().flex_col().size_full().bg(t.p.canvas).text_color(t.p.text).child(kit.titlebar('').controls([
		core.Element(name),
	])).child(ui.div().grow().flex_col().items_center().justify_center().px(t.sp(16)).child(form))
}

// lock_screen_from_settings is Lock Now from a window with no ViewCx[OneTerm].
fn (mut o OneTerm) lock_screen_from_settings(mut cx core.Cx) {
	o.vault.lock_screen()
	o.drain_vault_events(mut cx)
	o.lock_error = ''
}
