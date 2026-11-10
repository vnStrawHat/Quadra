// AuthForm: the method radio, secrets and saved-password state of one credential (specs/26).
module sessionui

import os
import quadra.core
import quadra.kit.input
import quadra.platform
import store
import vault

const save_password_label = 'Save password (encrypted with your master password)'
const update_password_label = 'Update saved password'

// SavedPassword is a sealed password and the address it is bound to.
pub struct SavedPassword {
pub:
	sealed   string
	host     string
	port     int
	username string
}

// saved_of is the session's saved password with its address, when it has both.
fn saved_of(s store.Session) ?SavedPassword {
	sealed := s.saved_password() or { return none }
	username := s.username or { return none }
	if username == '' {
		return none
	}
	return SavedPassword{sealed, s.host, s.port, username}
}

fn (s SavedPassword) context() vault.SealContext {
	return vault.SealContext{s.host, u16(s.port), s.username}
}

fn seal_context(s store.Session) ?vault.SealContext {
	username := s.username or { return none }
	return vault.SealContext{s.host, u16(s.port), username}
}

fn open_failure(err IError) string {
	mut reason := 'it cannot be used for this address'
	if err is vault.OpenError {
		reason = match err.kind {
			.other_vault { 'it was saved under a master password that was reset' }
			.newer_format { 'it was saved by a newer OneTerm' }
			.malformed { 'the stored value is not a saved password' }
			.rejected { 'it cannot be used for this address' }
		}
	}
	return 'The saved password cannot be used: ${reason}.'
}

@[heap]
pub struct AuthForm {
pub mut:
	id          string
	method      store.AuthMethod
	password    core.View[input.TextInput]
	key_path    core.View[input.TextInput]
	passphrase  core.View[input.TextInput]
	saved       ?SavedPassword
	using_saved bool // the saved password stands in for the input
	save_ticked bool // Save password / Update saved password
	removed     bool // the dialog's Remove: the saved password goes on Save
	offers_save bool // a typed password may be saved from this form
	note        string
	saved_focus core.FocusHandle // the Using saved password row
	secrets     bool             // connect-time credential inputs; off in the session dialog
	editor      bool             // the session dialog's saved password row (Replace, Remove)
	vault       &vault.VaultState = unsafe { nil }
	changed     ?core.AnyListener // bool: edited (clears the dialog's inline error)
}

fn new_auth_form(mut cx core.Cx, id string, method store.AuthMethod, key_path string, secrets bool, v &vault.VaultState) core.View[AuthForm] {
	f := cx.new_view(AuthForm{
		id:      id
		method:  method
		secrets: secrets
		editor:  !secrets
		vault:   unsafe { v }
	})
	cx.update_view(f, fn [key_path] (mut a AuthForm, mut cx core.ViewCx[AuthForm]) {
		a.saved_focus = cx.focus_handle()
		a.password = input.new_input(mut cx.Cx,
			id:          '${a.id}-password'
			placeholder: 'Enter password'
			password:    true
		)
		a.key_path = input.new_input(mut cx.Cx,
			id:          '${a.id}-key'
			placeholder: 'Select or enter private key path'
			value:       key_path
		)
		a.passphrase = input.new_input(mut cx.Cx,
			id:          '${a.id}-passphrase'
			placeholder: 'Enter passphrase if the key is encrypted'
			password:    true
		)
		for v in [a.password, a.passphrase, a.key_path] {
			input.on_change(mut cx.Cx, v, cx.listener(fn (mut a AuthForm, _ &string, mut cx core.ViewCx[AuthForm]) {
				a.edited(mut cx)
			}))
		}
	})
	return f
}

fn (mut a AuthForm) edited(mut cx core.ViewCx[AuthForm]) {
	cx.notify()
	mut hit := true
	mut l := a.changed
	if listener := l {
		cx.app.invoke(listener, &hit, cx.win, true)
	}
}

fn (a &AuthForm) has_saved() bool {
	return a.method == .password && a.saved != none
}

// use_saved shows "Using saved password" instead of the input (no-op without one).
fn (mut a AuthForm) use_saved(on bool) {
	a.using_saved = on && a.has_saved()
	if on {
		a.note = ''
	}
}

fn (a &AuthForm) uses_saved() bool {
	return a.using_saved && a.has_saved()
}

// wants_save: the user asked to save the password typed in this form.
fn (a &AuthForm) wants_save() bool {
	return a.save_ticked && a.method == .password && !a.uses_saved()
}

// reveal_input swaps the saved row back to an empty input, saying why.
fn (mut a AuthForm) reveal_input(note string, tick bool) {
	a.using_saved = false
	a.save_ticked = tick
	a.note = note
}

// keeps_saved: Save keeps the saved password (not removed, not replaced by a typed one).
fn (a &AuthForm) keeps_saved() bool {
	return a.has_saved() && !a.removed && !a.wants_save()
}

fn (mut a AuthForm) begin_replace() {
	a.using_saved = false
	a.save_ticked = true
	a.removed = false
}

fn (mut a AuthForm) remove_saved() {
	a.reveal_input('The saved password is removed when you save.', false)
	a.removed = true
}

// secret_focus is the credential input to type into first; none for the agent or a saved row.
fn (a &AuthForm) secret_focus(cx core.Cx) ?core.FocusHandle {
	if a.method == .password && !a.uses_saved() {
		return cx.peek(a.password.model()).focus
	}
	if a.method == .private_key {
		return cx.peek(a.passphrase.model()).focus
	}
	return none
}

// input_focus is what a saved session focuses: the saved row stands for its input.
fn (a &AuthForm) input_focus(cx core.Cx) ?core.FocusHandle {
	return match a.method {
		.password {
			if a.uses_saved() {
				a.saved_focus
			} else {
				cx.peek(a.password.model()).focus
			}
		}
		.private_key {
			cx.peek(a.key_path.model()).focus
		}
		.agent {
			none
		}
	}
}

fn (a &AuthForm) saved_row_focus() ?core.FocusHandle {
	return if a.uses_saved() { a.saved_focus } else { none }
}

fn (a &AuthForm) typed(cx core.Cx) string {
	return cx.peek(a.password.model()).value()
}

fn (a &AuthForm) key_value(cx core.Cx) string {
	return cx.peek(a.key_path.model()).value().trim_space()
}

// open_saved opens the saved password, for this connection only; on failure the row turns
// into an empty input with the reason.
fn (mut a AuthForm) open_saved() !string {
	saved := a.saved or { return error('No saved password.') }
	unlocked := a.vault.unlocked() or {
		msg := 'Saved passwords are locked. Type the password.'
		a.reveal_input(msg, false)
		return error(msg)
	}
	return unlocked.open(vault.SecretPurpose.ssh_password, saved.context(),
		vault.sealed(saved.sealed)) or {
		msg := '${open_failure(err)} Type the password.'
		a.reveal_input(msg, false)
		return error(msg)
	}
}

// take_credential builds the request's credential, validating the private key.
fn (mut a AuthForm) take_credential(mut cx core.Cx) !Credential {
	return match a.method {
		.password {
			if a.uses_saved() {
				Credential{
					method:   .password
					password: a.open_saved()!
				}
			} else {
				Credential{
					method:   .password
					password: a.typed(cx)
				}
			}
		}
		.private_key {
			path := a.key_value(cx)
			validate_private_key_path(path)!
			Credential{
				method:     .private_key
				key_path:   path
				passphrase: cx.peek(a.passphrase.model()).value()
			}
		}
		.agent {
			Credential{
				method: .agent
			}
		}
	}
}

fn validate_private_key_path(raw string) ! {
	if raw.trim_space() == '' {
		return error('Private key path is required.')
	}
	path := raw.trim_space()
	if !os.exists(path) {
		return error('Private key file is unavailable: no such file.')
	}
	if os.is_dir(path) {
		return error('Private key path must point to a file.')
	}
	mut f := os.open(path) or { return error('Private key file is not readable: ${err.msg()}') }
	f.close()
}

// browse picks a private key with the system dialog; ignored while the screen is locked.
fn (mut a AuthForm) browse(mut cx core.ViewCx[AuthForm]) {
	picked := cx.pick_files(platform.FileDialog{
		title: 'Select an SSH private key'
	})
	if picked.len == 0 || a.vault.is_screen_locked() {
		return
	}
	input.set_value(mut cx.Cx, a.key_path, picked[0])
	a.edited(mut cx)
	if a.secrets {
		cx.peek(a.passphrase.model()).focus.focus(mut cx.Cx)
	}
}
