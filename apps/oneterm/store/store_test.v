module store

import os

const otv1 = 'otv1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'
const otv2 = 'otv2:BBBB'

fn with_password(label string, envelope string) Session {
	mut s := session_named(label)
	s.username = 'deploy'
	s.sealed_password = envelope
	return s
}

fn test_logging_override_precedence() {
	assert !LoggingOverride.inherit.resolve(false)
	assert LoggingOverride.inherit.resolve(true)
	assert LoggingOverride.on.resolve(false)
	assert !LoggingOverride.off.resolve(true)
}

fn test_defaults_and_full_session() {
	loaded := parse_session_document('[{ "label": "dev", "host": "localhost" }]')!
	s := loaded.document.entries[0].session
	assert s.port == default_port
	assert s.username == none
	assert s.auth_method == .password
	assert s.logging == .inherit
	out := encode_session_document(loaded.document)
	assert !out.contains('logging') && !out.contains('username')
	assert out.contains('"group": null')
	full :=
		parse_session_document('[{ "label": "prod", "host": "10.0.0.1", "port": 2222, "username": "ubuntu" }]')!
	assert full.document.entries[0].session.port == 2222
	assert full.document.entries[0].session.username or { '' } == 'ubuntu'
}

fn test_private_key_and_agent_and_jump_and_forwarding_round_trip() {
	mut s := session_named('k')
	s.auth_method = .private_key
	s.key_path = '/keys/test'
	s.jump_host = 7
	s.agent_forwarding = true
	s.forwards = [
		PortForward{
			kind:        .local
			bind_port:   8080
			target_host: 'localhost'
			target_port: 80
		},
		PortForward{
			kind:        .remote
			bind:        '0.0.0.0'
			bind_port:   0
			target_host: 'h'
			target_port: 1
		},
		PortForward{
			kind:      .dynamic
			bind_port: 1080
		},
	]
	json := encode_session_document(doc_of([entry_of(3, s)], 4))
	assert json.contains('"auth_method": "private_key"')
	assert json.contains('"kind": "local"')
	assert json.contains('"bind_host": "0.0.0.0"')
	again := parse_session_document(json)!.document.entries[0].session
	assert again.key_path or { '' } == '/keys/test'
	assert again.jump_host or { 0 } == 7
	assert again.agent_forwarding
	assert again.forwards.len == 3
	assert again.forwards[1].bind == '0.0.0.0'
	assert again.forwards[2].kind == .dynamic
	plain := encode_session_document(doc_of([entry_of(1, session_named('p'))], 2))
	assert !plain.contains('port_forwards') && !plain.contains('agent_forwarding')
	assert !plain.contains('jump_host')
}

fn test_forward_rules() {
	f := PortForward{
		kind:        .local
		bind_port:   8080
		target_host: 'db'
		target_port: 5432
	}
	assert f.validate() == none
	assert f.summary() == 'L 127.0.0.1:8080 -> db:5432'
	assert f.binds_loopback()
	assert PortForward{
		...f
		bind_port: 0
	}.validate() or { ForwardError.empty_host } == .bind_port
	assert PortForward{
		...f
		target_host: 'a b'
	}.validate() or { ForwardError.empty_host } == .whitespace
	assert PortForward{
		kind:        .remote
		bind:        'localhost'
		bind_port:   0
		target_host: 'h'
		target_port: 1
	}.validate() == none
}

fn store_with(links []u64, jumps []?u64) &SessionStore {
	mut s := &SessionStore{
		persist_blocked: true
	}
	for i, id in links {
		mut session := session_named('n${id}')
		session.jump_host = jumps[i]
		s.entries << entry_of(id, session)
	}
	return s
}

fn test_jump_chain_lists_hops_outermost_first() {
	s := store_with([u64(1), 2, 3], [?u64(none), 1, 2])
	chain := s.jump_chain(3, none)!
	assert chain.map(it.id) == [u64(1), 2, 3]
	assert s.jump_chain(none, none)!.len == 0
}

fn test_jump_chain_rejects_missing_cycles_and_long_chains() {
	missing := store_with([u64(1)], [?u64(9)])
	missing.jump_chain(1, none) or {
		assert (err as JumpError).kind == .missing
		assert (err as JumpError).id == 9
		cycle := store_with([u64(1), 2], [?u64(2), 1])
		cycle.jump_chain(1, none) or {
			assert (err as JumpError).kind == .cycle
			target := store_with([u64(1)], [?u64(2)])
			target.jump_chain(1, 1) or {
				assert (err as JumpError).kind == .cycle
				long := store_with([u64(1), 2, 3, 4, 5], [?u64(none), 1, 2, 3, 4])
				long.jump_chain(5, none) or {
					assert (err as JumpError).kind == .too_long
					assert (err as JumpError).count == 5
					return
				}
			}
		}
	}
	assert false
}

fn test_unreadable_document_is_a_load_error_and_is_left_untouched() {
	dir := test_dir('unreadable')
	path := os.join_path(dir, document_name)
	os.mkdir_all(path)!
	if _ := load_sessions(path, false) {
		assert false, 'a directory must not read as an empty list'
	}
	assert os.is_dir(path)
	s := open(dir, false)
	assert s.persist_blocked
	assert !s.may_persist()
}

fn test_roundtrip_and_corruption_quarantine_are_isolated() {
	dir := test_dir('roundtrip')
	path := os.join_path(dir, document_name)
	mut key_session := session_named('test')
	key_session.port = 2222
	key_session.username = 'user'
	key_session.auth_method = .private_key
	key_session.key_path = '/keys/test'
	key_session.group = 'group'
	save_snapshot(doc_of([entry_of(3, key_session)], 4), path)!
	loaded := load_sessions(path, false)!
	assert !loaded.needs_resave
	assert loaded.document.entries[0].id == 3
	assert loaded.document.next_id == 4
	assert loaded.document.entries[0].session.auth_method == .private_key

	write_text(path, 'not-json')
	assert load_sessions(path, false)!.document.entries.len == 0
	assert !os.exists(path)
	assert os.ls(dir)!.any(it.contains('.invalid-'))
}

fn test_legacy_array_migrates_to_versioned_document_with_ids() {
	dir := test_dir('v0')
	path := os.join_path(dir, document_name)
	write_text(path, '[{"label":"legacy","host":"legacy.example.test","port":22,"group":null}]')
	loaded := load_sessions(path, false)!
	assert loaded.needs_resave
	assert loaded.document.entries[0].id == 1
	assert loaded.document.next_id == 2
	save_snapshot(loaded.document, path)!
	raw := os.read_file(path)!
	assert raw.contains('"schema_version": 2') && raw.contains('"next_session_id": 2')
	again := load_sessions(path, false)!
	assert !again.needs_resave
	assert again.document.entries[0].session.host == 'legacy.example.test'
}

fn test_v1_document_gains_ids() {
	raw := '{"schema_version":1,"sessions":[{"label":"alpha","host":"a","port":22,"group":null},{"label":"beta","host":"b","port":2200,"username":"deploy","auth_method":"private_key","key_path":"/keys/beta","color":"#112233","group":"infra"},{"label":"gamma","host":"c","group":null}]}'
	loaded := parse_session_document(raw)!
	assert loaded.needs_resave
	assert loaded.document.entries.map(it.id) == [u64(1), 2, 3]
	assert loaded.document.next_id == 4
	beta := loaded.document.entries[1].session
	assert beta.group or { '' } == 'infra' && beta.color or { '' } == '#112233'
}

fn test_v2_rows_without_unique_ids_are_repaired() {
	raw := '{"schema_version":2,"next_session_id":3,"sessions":[{"id":5,"label":"keep","host":"a","group":null},{"label":"no-id","host":"b","group":null},{"id":5,"label":"dup","host":"c","group":null}]}'
	loaded := parse_session_document(raw)!
	assert loaded.needs_resave
	assert loaded.document.entries.map(it.id) == [u64(5), 6, 7]
	assert loaded.document.next_id == 8
}

fn test_newer_schema_versions_are_rejected_not_truncated() {
	parse_session_document('{"schema_version":99,"sessions":[]}') or {
		assert err is InvalidData
		return
	}
	assert false
}

fn test_a_non_string_sealed_password_is_ignored_without_quarantine() {
	raw := '{"schema_version":2,"next_session_id":2,"sessions":[{"id":1,"label":"a","host":"h","group":null,"sealed_password":42}]}'
	assert parse_session_document(raw)!.document.entries[0].session.sealed_password == none
}

fn test_sealed_passwords_round_trip_untouched() {
	dir := test_dir('sealed-doc')
	path := os.join_path(dir, document_name)
	save_snapshot(doc_of([entry_of(1, with_password('a', otv2))], 2), path)!
	assert load_sessions(path, false)!.document.entries[0].session.sealed_password or { '' } == otv2
}

fn test_only_a_write_that_retires_a_sealed_password_scrubs_the_backup() {
	dir := test_dir('scrub')
	path := os.join_path(dir, document_name)
	backup := backup_path(path)
	save_snapshot(doc_of([entry_of(1, with_password('a', otv1))], 2), path)!
	mut renamed := with_password('renamed', otv1)
	renamed.group = 'g'
	save_snapshot(doc_of([entry_of(1, renamed)], 2), path)!
	assert os.read_file(backup)!.contains('"label": "a"')
	save_snapshot(doc_of([entry_of(1, with_password('a', otv2))], 2), path)!
	assert os.read_file(backup)! == os.read_file(path)!
	assert !os.read_file(backup)!.contains(otv1)
	save_snapshot(doc_of([entry_of(1, session_named('a'))], 2), path)!
	assert !os.read_file(backup)!.contains('sealed_password')
}

fn test_the_reset_wipe_is_idempotent() {
	dir := test_dir('wipe')
	path := os.join_path(dir, document_name)
	mut doc := doc_of([entry_of(1, with_password('a', otv1)),
		entry_of(2, with_password('b', otv2)), entry_of(3, session_named('c'))], 4)
	save_snapshot(doc, path)!
	assert clear_sealed_in(mut doc.entries)
	save_snapshot(doc, path)!
	assert !clear_sealed_in(mut doc.entries)
	assert doc.entries.len == 3
	assert !os.read_file(path)!.contains('otv')
	assert !os.read_file(backup_path(path))!.contains('otv')
}

fn test_a_sealed_password_is_stored_only_for_its_own_address() {
	mut entries := [entry_of(1, with_password('a', otv1))]
	set_sealed_in(mut entries, 1, 'a.example.test', 22, 'deploy', otv2)!
	assert entries[0].session.sealed_password or { '' } == otv2
	if _ := set_sealed_in(mut entries, 1, 'other.example.test', 22, 'deploy', otv1) {
		assert false
	}
	if _ := set_sealed_in(mut entries, 9, 'a.example.test', 22, 'deploy', otv1) {
		assert false
	}
	entries[0].session.auth_method = .agent
	if _ := set_sealed_in(mut entries, 1, 'a.example.test', 22, 'deploy', otv1) {
		assert false
	}
	assert entries[0].session.saved_password() == none
}
