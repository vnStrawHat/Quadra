module vault

import crypto.argon2
import os
import store
import x.json2

fn test_frozen_kdf_vector() {
	kek := derive_kek('correct horse battery staple', []u8{len: 16, init: 2}, KdfParams{64, 2, 1})!
	assert kek.hex() == '7c177df272a1b705e396ed8da3b16f8892b204ec523d65abfd310e2b2b927177'
}

fn test_rfc_9106_vector_shape_is_argon2id() {
	k := argon2.id_key([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, 3, 32, 4, 32)!
	assert k.len == 32
}

// The existing-user case: a file written by the Rust app, password and sealed value included.
fn test_the_v1_fixture_loads_unlocks_and_opens_its_sealed_password() {
	status := parse(v1_vault_json)
	assert status.kind == .ready
	assert status.header.kdf == kdf_floor
	v := unlock(status.header, v1_password)!
	assert v.needs_rehash()
	assert v.open(.ssh_password, ctx, sealed(v1_sealed))! == 's3cret'
}

fn test_bounds_cover_floor_default_and_ceiling_only() {
	assert kdf_floor.is_within_bounds()
	assert kdf_default.is_within_bounds()
	assert kdf_ceiling.is_within_bounds()
	for p in [KdfParams{kdf_floor.memory_kib - 1, 3, 1}, KdfParams{65536, 1, 1},
		KdfParams{kdf_ceiling.memory_kib + 1, 3, 1}, KdfParams{65536, 3, 5}] {
		assert !p.is_within_bounds()
	}
	assert kdf_floor.is_below_default()
	assert !kdf_default.is_below_default()
}

fn test_missing_file_is_uninitialised_and_unreadable_file_is_a_load_error() {
	dir := test_dir('load')
	assert load(dir)!.kind == .uninitialised
	os.mkdir(vault_path(dir))!
	if _ := load(dir) {
		assert false
	}
}

fn v1_with(change fn (mut doc map[string]json2.Any)) string {
	mut fields := store.object_of(store.parse_json(v1_vault_json) or { panic(err) }) or {
		panic('object')
	}
	change(mut fields)
	return store.encode_pretty(json2.Any(fields))
}

fn set_kdf(key string, value json2.Any) string {
	return v1_with(fn [key, value] (mut d map[string]json2.Any) {
		mut kdf := store.object_of(d['kdf'] or { panic('kdf') }) or { panic('kdf') }
		kdf[key] = value
		d['kdf'] = json2.Any(kdf)
	})
}

fn set_top(key string, value json2.Any) string {
	return v1_with(fn [key, value] (mut d map[string]json2.Any) {
		d[key] = value
	})
}

fn remove_top(key string) string {
	return v1_with(fn [key] (mut d map[string]json2.Any) {
		d.delete(key)
	})
}

fn test_every_field_rule_gives_its_outcome() {
	floor := i64(kdf_floor.memory_kib)
	damaged_cases := [remove_top('schema_version'), set_top('schema_version', json2.Any('1')),
		'[1, 2]', remove_top('wrapped_key'), set_kdf('memory_kib', json2.Any('19456')),
		set_top('vault_id', json2.Any('3F9C0A4E8B1D4C27A6E2F0B9C1D2E3F4')),
		set_top('vault_id', json2.Any('3f9c')), set_kdf('salt', json2.Any('not base64!')),
		set_top('wrapped_key', json2.Any('plain')),
		set_top('vault_id',
			json2.Any('00000000000000000000000000000000')), 'not json']
	for raw in damaged_cases {
		assert parse(raw).kind == .damaged, raw
	}
	unsupported_cases := [set_top('schema_version', json2.Any(i64(2))),
		set_kdf('algorithm', json2.Any('argon2i')), set_kdf('version', json2.Any(i64(16))),
		set_kdf('memory_kib', json2.Any(floor - 1)), set_kdf('iterations', json2.Any(i64(1))),
		set_kdf('iterations', json2.Any(i64(17))), set_kdf('parallelism', json2.Any(i64(5))),
		set_kdf('salt', json2.Any('AAAAAAAAAAA=')), set_top('wrapped_key', json2.Any('otv2:AAAA'))]
	for raw in unsupported_cases {
		assert parse(raw).kind == .unsupported, raw
	}
}

fn test_unknown_fields_are_ignored_on_read() {
	assert parse(set_top('future_field', json2.Any(i64(1)))).kind == .ready
}

fn test_set_up_writes_a_header_that_loads_and_never_holds_the_password() {
	dir := test_dir('setup')
	v := floor_vault(dir, 'hunter2hunter2')
	raw := os.read_file(vault_path(dir))!
	assert !raw.contains('hunter2')
	assert raw.contains('"memory_kib": 19456')
	status := load(dir)!
	assert status.kind == .ready
	assert status.header.vault_id == v.vault_id()
	assert os.read_file(store.backup_path(vault_path(dir)))! == raw
}

fn test_set_up_is_create_only_and_leaves_an_existing_or_damaged_file_alone() {
	dir := test_dir('create-only')
	for content in ['{}', 'not json', v1_vault_json] {
		os.write_file(vault_path(dir), content)!
		init_with(dir, 'password', fast_params, false) or {
			assert (err as VaultError).kind == .already_initialised
			assert os.read_file(vault_path(dir))! == content
			continue
		}
		assert false
	}
}

fn try_set_up(dir string) bool {
	init_with(dir, 'password', fast_params, false) or { return false }
	return true
}

fn test_two_concurrent_set_ups_produce_exactly_one_vault() {
	dir := test_dir('race')
	mut threads := []thread bool{}
	threads << spawn try_set_up(dir)
	threads << spawn try_set_up(dir)
	assert threads.wait().filter(it).len == 1
}

fn change_kind(mut v UnlockedVault, dir string) VaultErrorKind {
	v.change_password_with(dir, 'one', 'two', floor_params, false) or {
		return (err as VaultError).kind
	}
	panic('expected a refusal')
}

fn test_change_password_rewraps_the_same_data_key_and_scrubs_the_backup() {
	dir := test_dir('change')
	mut v := floor_vault(dir, 'one')
	value := v.seal(.ssh_password, ctx, 'pw')!
	v.change_password_with(dir, 'one', 'two', floor_params, false)!
	status := load(dir)!
	assert status.header.vault_id == v.vault_id()
	v2 := unlock(status.header, 'two')!
	assert v2.open(.ssh_password, ctx, value)! == 'pw'
	if _ := unlock(status.header, 'one') {
		assert false
	}
	backup := os.read_file(store.backup_path(vault_path(dir)))!
	assert backup == os.read_file(vault_path(dir))!
}

fn test_wrong_old_password_is_refused() {
	dir := test_dir('wrong-old')
	mut v := floor_vault(dir, 'one')
	v.change_password_with(dir, 'wrong', 'two', floor_params, false) or {
		assert (err as VaultError).kind == .wrong_password
		return
	}
	assert false
}

fn test_change_password_preserves_fields_this_binary_does_not_know() {
	dir := test_dir('preserve')
	mut v := floor_vault(dir, 'one')
	mut fields := store.object_of(store.parse_json(os.read_file(vault_path(dir))!)!) or {
		panic('object')
	}
	fields['future'] = json2.Any(i64(1048577))
	mut kdf := store.object_of(fields['kdf'] or { panic('kdf') }) or { panic('kdf') }
	kdf['future_kdf'] = json2.Any('x')
	fields['kdf'] = json2.Any(kdf)
	os.write_file(vault_path(dir), store.encode_pretty(json2.Any(fields)))!
	v.change_password_with(dir, 'one', 'two', floor_params, false)!
	raw := os.read_file(vault_path(dir))!
	assert raw.contains('"future": 1048577')
	assert raw.contains('"future_kdf": "x"')
}

fn test_change_refuses_when_the_vault_was_reset_or_replaced_elsewhere() {
	dir := test_dir('replaced')
	mut v := floor_vault(dir, 'one')
	os.rm(vault_path(dir))!
	assert change_kind(mut v, dir) == .vault_replaced
	os.write_file(vault_path(dir), 'not json')!
	assert change_kind(mut v, dir) == .vault_replaced
	os.write_file(vault_path(dir), v1_vault_json)!
	assert change_kind(mut v, dir) == .vault_replaced
}

fn test_change_refuses_a_header_rewrapped_by_another_process() {
	dir := test_dir('rewrapped')
	mut v := floor_vault(dir, 'one')
	mut other := unlock(v.header, 'one')!
	other.change_password_with(dir, 'one', 'three', floor_params, false)!
	assert change_kind(mut v, dir) == .changed_elsewhere
}

fn test_an_io_failure_is_not_reported_as_a_refusal() {
	dir := test_dir('io')
	mut v := floor_vault(dir, 'one')
	os.rm(vault_path(dir))!
	os.mkdir(vault_path(dir))!
	assert change_kind(mut v, dir) == .io
}

fn test_reset_removes_the_header_and_its_backup_and_refuses_a_newer_file() {
	dir := test_dir('reset')
	floor_vault(dir, 'one')
	reset(dir, false)!
	assert !os.exists(vault_path(dir))
	assert !os.exists(store.backup_path(vault_path(dir)))
	reset(dir, false)!
	os.write_file(vault_path(dir), 'not json')!
	reset(dir, false)!
	assert !os.exists(vault_path(dir))
	newer := set_top('schema_version', json2.Any(i64(9)))
	os.write_file(vault_path(dir), newer)!
	reset(dir, false) or {
		assert (err as VaultError).kind == .unsupported
		assert os.read_file(vault_path(dir))! == newer
		return
	}
	assert false
}

fn test_a_read_only_window_writes_nothing() {
	dir := test_dir('read-only')
	init_with(dir, 'one', fast_params, true) or {
		assert (err as VaultError).kind == .read_only
		assert !os.exists(vault_path(dir))
		mut v := floor_vault(dir, 'one')
		raw := os.read_file(vault_path(dir))!
		v.change_password_with(dir, 'one', 'two', floor_params, true) or {
			assert (err as VaultError).kind == .read_only
			reset(dir, true) or {
				assert (err as VaultError).kind == .read_only
				assert os.read_file(vault_path(dir))! == raw
				return
			}
		}
	}
	assert false
}

fn test_unlock_uses_the_header_held_and_never_rereads_the_file() {
	dir := test_dir('no-reread')
	v := floor_vault(dir, 'one')
	header := v.header()
	os.rm(vault_path(dir))!
	unlock(header, 'one')!
	unlock(header, '') or {
		assert (err as VaultError).kind == .empty_password
		unlock(header, 'wrong') or {
			assert (err as VaultError).kind == .wrong_password
			return
		}
	}
	assert false
}

fn test_lock_zeroes_the_key() {
	mut v := fast_vault(test_dir('lock'), 'one')
	v.lock()
	assert v.data_key.len == 0
}
