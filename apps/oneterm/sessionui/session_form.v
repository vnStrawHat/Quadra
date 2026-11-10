// The session form's pure logic: validation and the password plan (specs/26).
module sessionui

import store
import vault

// SessionForm is the raw values of the session dialog, as typed.
pub struct SessionForm {
pub mut:
	label            string
	host             string
	port             string // empty means the default port
	username         string
	group            string
	color            ?string
	auth_method      store.AuthMethod
	key_path         string
	logging          store.LoggingOverride
	jump_host        ?u64
	forwards         []store.PortForward
	agent_forwarding bool
}

fn non_empty(text string) ?string {
	t := text.trim_space()
	return if t == '' { none } else { t }
}

// into_session validates and builds the session to store; the error is shown to the user.
pub fn (f SessionForm) into_session() !store.Session {
	label := f.label.trim_space()
	host := f.host.trim_space()
	if label == '' || host == '' {
		return error('Label and Host are required.')
	}
	port_text := f.port.trim_space()
	port := if port_text == '' { store.default_port } else { parse_port(port_text)! }
	mut key_path := ?string(none)
	if f.auth_method == .private_key {
		k := f.key_path.trim_space()
		if k == '' {
			return error('Private key path is required.')
		}
		key_path = k
	}
	return store.Session{
		label:            label
		host:             host
		port:             port
		username:         non_empty(f.username)
		auth_method:      f.auth_method
		key_path:         key_path
		color:            f.color
		group:            non_empty(f.group)
		logging:          f.logging
		jump_host:        f.jump_host
		forwards:         f.forwards
		agent_forwarding: f.agent_forwarding
	}
}

// PlanKind says what Save does with the session's saved password.
pub enum PlanKind {
	store  // keep this value as it is (none, or the unchanged envelope)
	drop   // drop the saved password and say why
	seal   // seal the typed password for the session's address
	reseal // open the saved password under its old address and seal it for the new one
}

pub struct PasswordPlan {
pub:
	kind  PlanKind
	value ?string // .store: the sealed value
	note  string  // .drop: why
}

fn (p PasswordPlan) needs_vault() bool {
	return p.kind == .seal || p.kind == .reseal
}

// password_plan decides what Save does with the password. keep_saved: the saved one stays
// (not removed, not replaced by a typed one); save_typed: Save/Update is ticked.
pub fn password_plan(saved ?SavedPassword, session store.Session, keep_saved bool, save_typed bool, typed_empty bool) !PasswordPlan {
	if session.auth_method != .password {
		return PasswordPlan{
			kind: .store
		}
	}
	has_context := (session.username or { '' }) != ''
	if save_typed {
		if !has_context {
			return error('Enter a username to save a password.')
		}
		if typed_empty {
			return error('Enter the password to save, or untick Save password.')
		}
		return PasswordPlan{
			kind: .seal
		}
	}
	if s := saved {
		if keep_saved {
			if !has_context {
				return PasswordPlan{
					kind: .drop
					note: 'The saved password was removed: a saved password needs a username.'
				}
			}
			if session.host == s.host && session.port == s.port
				&& (session.username or { '' }) == s.username {
				return PasswordPlan{
					kind:  .store
					value: s.sealed
				}
			}
			return PasswordPlan{
				kind: .reseal
			}
		}
	}
	return PasswordPlan{
		kind: .store
	}
}

// reveals_advanced: a refused Save opens Advanced only for the fields it hides.
pub enum InvalidField {
	port_forward
	jump_host
	basic
}

fn reveals_advanced(f InvalidField) bool {
	return f == .port_forward || f == .jump_host
}

// strength_hint is the length-based hint under a new master password; '' while empty.
pub fn strength_hint(chars int) string {
	return if chars == 0 {
		''
	} else if chars < vault.min_password_chars {
		'Too short: use at least ${vault.min_password_chars} characters.'
	} else if chars < 12 {
		'Strength: fair. A longer password is stronger.'
	} else if chars < 16 {
		'Strength: good.'
	} else {
		'Strength: strong.'
	}
}

// validate_new_password compares exactly as typed: no trimming, no normalisation.
pub fn validate_new_password(password string, confirmation string) ! {
	if password.runes().len < vault.min_password_chars {
		return error('Use at least ${vault.min_password_chars} characters.')
	}
	if password != confirmation {
		return error('The two passwords differ.')
	}
}
