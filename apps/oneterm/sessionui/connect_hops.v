// The connect dialog's hops, initial focus and saved-password activation (specs/26).
module sessionui

import quadra.core

// rebuild_hops rebuilds the credential blocks when the picker's selection moved.
fn (mut d ConnectDialog) rebuild_hops(mut cx core.ViewCx[ConnectDialog]) {
	mut p := d.panel
	selected := d.picker.selected(cx.Cx)
	d.picked = selected
	d.chain_error = ''
	chain := p.sessions.jump_chain(selected, none) or {
		d.chain_error = err.msg()
		d.hops = []
		cx.notify()
		return
	}
	mut specs := []HopSpec{}
	for e in chain {
		specs << hop_spec_of(e) or {
			d.chain_error = err.msg()
			d.hops = []
			cx.notify()
			return
		}
	}
	d.hops = build_hop_forms(mut cx.Cx, specs, p)
	unlocked := p.vault.phase() == .unlocked
	for h in d.hops {
		d.watch_auth(h.form, mut cx)
		mut fm := cx.peek(h.form.model())
		fm.use_saved(unlocked)
	}
	cx.notify()
}

// focus_first puts the caret where the typing starts; a saved row stands for its input, so
// Enter connects at once when every password is saved.
fn (mut d ConnectDialog) focus_first(mut cx core.Cx) {
	if d.has_username && d.mode == .saved {
		cx.peek(d.username.model()).focus.focus(mut cx)
		return
	}
	if d.mode == .quick {
		cx.peek(d.host.model()).focus.focus(mut cx)
		return
	}
	for h in d.hops {
		if f := cx.peek(h.form.model()).secret_focus(cx) {
			f.focus(mut cx)
			return
		}
	}
	a := cx.peek(d.auth.model())
	mut target := a.secret_focus(cx)
	if d.mode == .saved {
		target = a.input_focus(cx)
	}
	if t := target {
		t.focus(mut cx)
		return
	}
	for h in d.hops {
		if f := cx.peek(h.form.model()).saved_row_focus() {
			f.focus(mut cx)
			return
		}
	}
}

// activate_saved turns saved rows on when the vault is unlocked or asks to unlock once.
fn (mut d ConnectDialog) activate_saved(m core.Model[ConnectDialog], mut cx core.Cx) {
	mut p := d.panel
	mut forms := d.hops.map(it.form)
	if sid := d.session_id {
		if rejected_value := p.rejected[sid] {
			if rejected_value == (d.session.saved_password() or { '' }) {
				mut a := cx.peek(d.auth.model())
				a.reveal_input(rejected_note, true)
			} else {
				forms << d.auth
			}
		} else {
			forms << d.auth
		}
	}
	saved_forms := forms.filter(cx.peek(it.model()).has_saved())
	if saved_forms.len == 0 {
		return
	}
	mut note := ''
	match p.vault.phase() {
		.unlocked {
			for f in saved_forms {
				mut fm := cx.peek(f.model())
				fm.use_saved(true)
			}
			return
		}
		.locked {
			p.ensure_unlocked(mut cx, fn [saved_forms, m] (mut cx core.Cx) {
				for f in saved_forms {
					mut fm := cx.peek(f.model())
					fm.use_saved(true)
				}
				cx.update(m, fn (mut d ConnectDialog, mut cx core.ViewCx[ConnectDialog]) {
					d.focus_first(mut cx.Cx)
				})
			})
			return
		}
		.uninitialised {
			note = 'Saved password unavailable: the master password was reset. Type the password.'
		}
		.unavailable {
			mut reason := p.vault.unavailable_reason()
			if reason == '' {
				reason = 'no master password store.'
			}
			note = 'Saved password unavailable: ${reason}'
		}
	}

	for f in saved_forms {
		mut fm := cx.peek(f.model())
		fm.note = note
	}
}
