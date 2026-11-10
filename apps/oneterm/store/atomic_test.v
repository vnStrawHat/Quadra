module store

import os
import x.json2

fn test_atomic_write_replaces_complete_content_and_keeps_backup() {
	dir := test_dir('atomic')
	path := os.join_path(dir, 'state.json')
	atomic_write(path, '{"version":1}'.bytes())!
	atomic_write(path, '{"version":2}'.bytes())!
	assert os.read_file(path)! == '{"version":2}'
	assert os.read_file(os.join_path(dir, 'state.bak'))! == '{"version":1}'
	assert os.ls(dir)!.len == 2 // the lock file is removed at release (specs/25)
}

fn test_failed_writes_preserve_the_previous_document() {
	dir := test_dir('failed')
	path := os.join_path(dir, 'state.json')
	atomic_write(path, '{"version":1}'.bytes())!
	os.mkdir(backup_path(path))! // a directory on the .bak name makes the backup copy fail
	if _ := atomic_write(path, '{"version":2}'.bytes()) {
		assert false, 'expected the write to fail'
	}
	assert os.read_file(path)! == '{"version":1}'
	assert os.ls(dir)!.filter(it.contains('.tmp-')).len == 0
}

fn test_update_json_preserves_unknown_fields_and_integers() {
	path := os.join_path(test_dir('update'), 'shared.json')
	write_text(path, '{"memory":1048577,"nested":{"unknown":65536,"f":1.5}}')
	update_json_file(path, fn (mut doc json2.Any) ! {
		mut fields := object_of(doc) or { return error('not an object') }
		fields['added'] = json2.Any(i64(7))
		doc = json2.Any(fields)
	})!
	text := os.read_file(path)!
	assert text.contains('"memory": 1048577')
	assert text.contains('"unknown": 65536')
	assert text.contains('"f": 1.5')
	assert text.contains('"added": 7')
	assert !text.contains('e+')
}

fn count_up(path string) {
	update_json_file(path, fn (mut doc json2.Any) ! {
		mut fields := object_of(doc) or { return error('not an object') }
		count := u64_of(fields['count'] or { return error('missing') }) or { 0 }
		fields['count'] = json2.Any(i64(count + 1))
		doc = json2.Any(fields)
	}) or { panic(err) }
}

fn test_json_updates_serialize_the_read_modify_write_transaction() {
	path := os.join_path(test_dir('serialize'), 'shared.json')
	atomic_write(path, '{"count":0}'.bytes())!
	mut writers := []thread{}
	for _ in 0 .. 8 {
		writers << spawn count_up(path)
	}
	writers.wait()
	doc := parse_json(os.read_file(path)!)!
	assert u64_of(object_of(doc) or { panic('object') }['count'] or { panic('count') }) or { 0 } == 8
}

fn test_invalid_document_update_is_invalid_data() {
	path := os.join_path(test_dir('invalid'), 'x.json')
	write_text(path, 'not-json')
	update_json_file(path, fn (mut doc json2.Any) ! {}) or {
		assert err is InvalidData
		return
	}
	assert false
}

fn test_secret_updates_leave_a_backup_equal_to_the_new_document() {
	path := os.join_path(test_dir('secret'), 'secret.json')
	for generation in [1, 2] {
		update_secret_json_file(path, fn [generation] (mut doc json2.Any) ! {
			mut fields := object_of(doc) or { return error('not an object') }
			fields['generation'] = json2.Any(i64(generation))
			doc = json2.Any(fields)
		})!
	}
	assert os.read_file(path)!.contains('"generation": 2')
	assert os.read_file(backup_path(path))! == os.read_file(path)!
}

fn test_sealed_writes_scrub_the_backup_only_when_they_retire_a_secret() {
	path := os.join_path(test_dir('sealed'), 'sealed.json')
	atomic_write_sealed(path, 'one'.bytes(), fn (_ []u8) bool {
		panic('no previous document')
	})!
	assert !os.exists(backup_path(path))
	atomic_write_sealed(path, 'two'.bytes(), fn (prev []u8) bool {
		assert prev == 'one'.bytes()
		return false
	})!
	assert os.read_file(backup_path(path))! == 'one'
	atomic_write_sealed(path, 'three'.bytes(), fn (_ []u8) bool {
		return true
	})!
	assert os.read_file(path)! == 'three'
	assert os.read_file(backup_path(path))! == 'three'
}

fn test_remove_document_removes_the_file_and_its_backup_and_tolerates_absence() {
	path := os.join_path(test_dir('remove'), 'state.json')
	atomic_write(path, '1'.bytes())!
	atomic_write(path, '2'.bytes())!
	remove_document(path, false)!
	assert !os.exists(path)
	assert !os.exists(backup_path(path))
	remove_document(path, false)!
}

fn test_a_read_only_window_removes_nothing() {
	path := os.join_path(test_dir('ro-remove'), 'state.json')
	atomic_write(path, '1'.bytes())!
	if _ := remove_document(path, true) {
		assert false
	}
	assert os.exists(path)
}

fn test_quarantine_moves_invalid_content_without_overwriting_it() {
	dir := test_dir('quarantine')
	path := os.join_path(dir, 'state.json')
	write_text(path, 'invalid')
	first := quarantine_file(path, false)!
	assert first.contains('.invalid-')
	assert !os.exists(path)
	assert os.read_file(first)! == 'invalid'
	write_text(path, 'second')
	second := quarantine_file(path, false)!
	assert second != first
	assert os.read_file(first)! == 'invalid'
	assert os.read_file(second)! == 'second'
	assert quarantine_file(path, false)! == ''
}

fn test_a_read_only_window_quarantines_nothing() {
	path := os.join_path(test_dir('ro-quarantine'), 'state.json')
	write_text(path, 'invalid')
	assert quarantine_file(path, true)! == ''
	assert os.exists(path)
}

fn test_absent_version_migrates_sequentially_and_is_idempotent() {
	doc := parse_json('{"value":7}')!
	migrated := migrate_json_value(doc, 2, 'test', fn (v u32, value json2.Any) !json2.Any {
		mut fields := object_of(value) or { return error('x') }
		fields['step_${v}'] = json2.Any(true)
		return json2.Any(fields)
	})!
	assert schema_version(migrated)! == 2
	fields := object_of(migrated) or { panic('object') }
	assert 'step_0' in fields && 'step_1' in fields
	again := migrate_json_value(migrated, 2, 'test', fn (_ u32, _ json2.Any) !json2.Any {
		panic('current documents must not migrate')
	})!
	assert object_of(again) or { panic('object') }.len == fields.len
}

fn test_future_versions_are_rejected() {
	migrate_json_value(parse_json('{"schema_version":9}')!, 1, 'test', fn (_ u32, v json2.Any) !json2.Any {
		return v
	}) or {
		assert err is InvalidData
		assert err.msg().contains('newer than supported')
		return
	}
	assert false
}
