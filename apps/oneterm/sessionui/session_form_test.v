module sessionui

import store

fn filled_form() SessionForm {
	return SessionForm{
		label:    '  prod  '
		host:     ' 10.0.0.1 '
		username: '  '
		group:    ' ops '
		color:    '#56B6C2'
	}
}

fn test_form_trims_fields_and_defaults() {
	s := filled_form().into_session()!
	assert s.label == 'prod' && s.host == '10.0.0.1' && s.port == store.default_port
	assert s.username == none && s.group or { '' } == 'ops' && s.key_path == none
}

fn test_form_keeps_agent_and_logging() {
	mut f := filled_form()
	f.agent_forwarding = true
	f.logging = .off
	s := f.into_session()!
	assert s.agent_forwarding && s.logging == .off
}

fn test_form_requires_label_host_and_valid_port() {
	mut a := filled_form()
	a.label = '   '
	a.into_session() or {
		assert err.msg() == 'Label and Host are required.'
		mut b := filled_form()
		b.port = '70000'
		b.into_session() or {
			assert err.msg().contains('70000')
			mut c := filled_form()
			c.port = ' 2222 '
			assert c.into_session()!.port == 2222
			return
		}
		assert false
		return
	}
	assert false
}

fn test_key_path_is_required_only_for_private_key() {
	mut key := filled_form()
	key.auth_method = .private_key
	key.into_session() or {
		assert err.msg() == 'Private key path is required.'
		key.key_path = '/keys/id_ed25519'
		assert key.into_session()!.key_path or { '' } == '/keys/id_ed25519'
		mut pw := filled_form()
		pw.key_path = '/keys/id_ed25519'
		assert pw.into_session()!.key_path == none
		return
	}
	assert false
}

fn test_advanced_opens_only_when_configured() {
	assert !advanced_is_configured(none, [], false)
	assert advanced_is_configured(7, [], false)
	assert advanced_is_configured(none, [
		store.PortForward{ kind: .dynamic, bind_port: 1080 },
	], false)
	assert advanced_is_configured(none, [], true)
	assert reveals_advanced(.port_forward) && reveals_advanced(.jump_host)
		&& !reveals_advanced(.basic)
}

const envelope = 'otv1:AAECAw'

fn saved_for(host string) SavedPassword {
	return SavedPassword{
		sealed:   envelope
		host:     host
		port:     22
		username: 'deploy'
	}
}

fn pw_session(host string, username ?string) store.Session {
	mut f := filled_form()
	f.host = host
	f.username = username or { '' }
	return f.into_session() or { panic(err) }
}

// The reseal decision table of session_dialog.rs.
fn test_password_plan_follows_the_decision_table() {
	saved := saved_for('10.0.0.1')
	same := pw_session('10.0.0.1', 'deploy')
	unchanged := password_plan(saved, same, true, false, true)!
	assert unchanged.kind == .store && unchanged.value or { '' } == envelope
	for edited in [pw_session('10.0.0.9', 'deploy'), pw_session('10.0.0.1', 'root')] {
		assert password_plan(saved, edited, true, false, true)!.kind == .reseal
	}
	assert password_plan(saved, pw_session('10.0.0.1', none), true, false, true)!.kind == .drop
	mut agent := same
	agent.auth_method = .agent
	cleared := password_plan(saved, agent, true, false, true)!
	assert cleared.kind == .store && cleared.value == none
	assert password_plan(saved, same, false, false, true)!.value == none // Remove clicked
	assert password_plan(none, same, false, true, false)!.kind == .seal
	password_plan(none, pw_session('10.0.0.1', none), false, true, false) or {
		assert err.msg() == 'Enter a username to save a password.'
		password_plan(none, same, false, true, true) or {
			assert err.msg().contains('untick Save password')
			return
		}
		assert false
		return
	}
	assert false
}

// A password can be sealed only for an address it was typed against (specs/26 password plan).
fn test_existing_groups_are_trimmed_sorted_and_unique() {
	entries := [
		store.Entry{
			id:      1
			session: store.Session{
				group: ' ops '
			}
		},
		store.Entry{
			id: 2
		},
		store.Entry{
			id:      3
			session: store.Session{
				group: 'dev'
			}
		},
		store.Entry{
			id:      4
			session: store.Session{
				group: 'ops'
			}
		},
		store.Entry{
			id:      5
			session: store.Session{
				group: '   '
			}
		},
	]
	assert existing_group_names(entries) == ['dev', 'ops']
	assert existing_group_names([]) == []
}

fn test_credential_str_redacts_secrets() {
	c := Credential{
		method:     .password
		password:   'hunter2'
		passphrase: 'swordfish'
	}
	assert !c.str().contains('hunter2') && !c.str().contains('swordfish')
}

fn test_strength_hint_and_new_password_rules() {
	assert strength_hint(0) == ''
	assert strength_hint(5).contains('at least 8')
	assert strength_hint(8).contains('fair')
	assert strength_hint(12).contains('good')
	assert strength_hint(21).contains('strong')
	validate_new_password('1234567', '1234567') or {
		assert err.msg().contains('8')
		validate_new_password('12345678', '12345678')!
		validate_new_password('12345678 ', '12345678') or {
			assert err.msg() == 'The two passwords differ.'
			return
		}
		assert false
		return
	}
	assert false
}
