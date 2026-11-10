// Save in the session dialog: validation order, the password plan and the store (specs/26).
module sessionui

import quadra.core
import store
import vault

// close_modal_click is the Cancel button of the dialogs.
fn close_modal_click(id int, _ &core.ClickEvent, mut cx core.Cx) {
	kit_close(mut cx, id)
}

// save validates in the Rust order: forwards, jump chain, form, password plan.
fn (mut d SessionDialog) save(mut cx core.ViewCx[SessionDialog]) {
	mut p := d.panel
	forwards := d.take_forwards(cx.Cx) or {
		d.reveal(.port_forward, mut cx)
		warn(mut cx.Cx, err.msg())
		return
	}
	jump := d.jump.selected(cx.Cx)
	p.sessions.jump_chain(jump, d.edit_id) or {
		d.reveal(.jump_host, mut cx)
		warn(mut cx.Cx, err.msg())
		return
	}
	mut auth := cx.peek(d.auth.model())
	form := SessionForm{
		label:            cx.peek(d.label.model()).value()
		host:             cx.peek(d.host.model()).value()
		port:             cx.peek(d.port.model()).value()
		username:         cx.peek(d.username.model()).value()
		group:            cx.peek(d.group.model()).value
		color:            d.color_value
		auth_method:      auth.method
		key_path:         auth.key_value(cx.Cx)
		logging:          d.logging
		jump_host:        jump
		forwards:         forwards
		agent_forwarding: d.agent
	}
	mut session := form.into_session() or {
		warn(mut cx.Cx, err.msg())
		return
	}
	plan := password_plan(auth.saved, session, auth.keeps_saved(), auth.wants_save(),
		auth.typed(cx.Cx) == '') or {
		warn(mut cx.Cx, err.msg())
		return
	}
	if !plan.needs_vault() {
		match plan.kind {
			.drop { session.sealed_password = none }
			else { session.sealed_password = plan.value }
		}

		d.commit(session, plan.note, mut cx)
		return
	}
	typed := if plan.kind == .seal { auth.typed(cx.Cx) } else { '' }
	old := auth.saved or { SavedPassword{} }
	reseal := plan.kind == .reseal
	match p.vault.phase() {
		.unlocked {
			d.finish(plan, session, typed, old, mut cx)
		}
		.uninitialised {
			if reseal {
				d.finish(plan, session, typed, old, mut cx) // no vault: finish drops it with a note
				return
			}
			d.after_unlock(plan, session, typed, old, mut cx)
		}
		.locked {
			if reseal {
				info(mut cx.Cx,
					'Unlock to keep the saved password for the new address, or click Remove.')
			}
			d.after_unlock(plan, session, typed, old, mut cx)
		}
		.unavailable {
			mut reason := p.vault.unavailable_reason()
			if reason == '' {
				reason = 'no master password store.'
			}
			warn(mut cx.Cx,
				'Saved passwords are unavailable: ${reason} Untick Save password or remove the saved one.')
		}
	}
}

fn (mut d SessionDialog) reveal(field InvalidField, mut cx core.ViewCx[SessionDialog]) {
	if reveals_advanced(field) {
		d.advanced = true
		cx.notify()
	}
}

fn (mut d SessionDialog) after_unlock(plan PasswordPlan, session store.Session, typed string, old SavedPassword, mut cx core.ViewCx[SessionDialog]) {
	mut p := d.panel
	// The typed password rides in the listener's data, freed with the vault dialog (a
	// capturing closure would keep it in memory for good).
	p.ensure_unlocked(mut cx.Cx, core.listener_with(d.me.id, AfterUnlock{plan, session, typed, old}, fn (mut d SessionDialog, a AfterUnlock, _ &bool, mut cx core.ViewCx[SessionDialog]) {
		d.finish(a.plan, a.session, a.typed, a.old, mut cx)
	}))
}

struct AfterUnlock {
	plan    PasswordPlan
	session store.Session
	typed   string
	old     SavedPassword
}

// finish seals the password per the plan, stores the session and closes the dialog.
fn (mut d SessionDialog) finish(plan PasswordPlan, session_ store.Session, typed string, old SavedPassword, mut cx core.ViewCx[SessionDialog]) {
	mut session := session_
	mut p := d.panel
	mut note := plan.note
	ctx := seal_context(session) or {
		session.sealed_password = none
		d.commit(session, note, mut cx)
		return
	}
	if plan.kind == .seal {
		sealed := p.vault.seal(.ssh_password, ctx, typed) or {
			warn(mut cx.Cx, err.msg())
			return
		}
		session.sealed_password = sealed.raw()
	} else {
		unlocked := p.vault.unlocked() or {
			session.sealed_password = none
			d.commit(session,
				'The saved password could not be used for the new address. It was removed.', mut cx)
			return
		}
		password := unlocked.open(.ssh_password, old.context(), vault.sealed(old.sealed)) or {
			note = '${open_failure(err)} It was removed.'
			session.sealed_password = none
			d.commit(session, note, mut cx)
			return
		}
		sealed := p.vault.seal(.ssh_password, ctx, password) or {
			warn(mut cx.Cx, err.msg())
			return
		}
		session.sealed_password = sealed.raw()
	}
	d.commit(session, note, mut cx)
}

// commit adds or updates the session and says so.
fn (mut d SessionDialog) commit(session store.Session, note string, mut cx core.ViewCx[SessionDialog]) {
	mut p := d.panel
	if id := d.edit_id {
		p.sessions.update(id, session)
		success(mut cx.Cx, 'SSH session updated.')
	} else {
		p.sessions.add(session)
		success(mut cx.Cx, 'SSH session saved.')
	}
	if note != '' {
		info(mut cx.Cx, note)
	}
	p.refresh(mut cx.Cx)
	kit_close(mut cx.Cx, d.modal)
}
