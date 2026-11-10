module vault

import os

fn state_in(dir string) &VaultState {
	mut s := new_state(dir, false)
	s.params = kdf_floor
	return s
}

fn unlocked_state(dir string, password string) &VaultState {
	mut s := state_in(dir)
	s.apply(s.begin_set_up(password) or { panic(err) }.run()) or { panic(err) }
	return s
}

fn expect_vault_err(err IError, kind VaultErrorKind) {
	assert (err as VaultActionError).kind == .vault
	assert (err as VaultActionError).vault.kind == kind
}

fn test_the_backoff_starts_at_the_fifth_failure_and_caps_at_thirty_seconds() {
	expected := [i64(0), 0, 0, 0, 2000, 4000, 8000, 16000, 30000, 30000]
	for i, wait in expected {
		assert backoff_after(u32(i + 1)) == wait, 'failure ${i + 1}'
	}
	assert backoff_after(1000) == max_backoff_ms
}

fn test_the_retry_message_never_reads_zero_seconds() {
	assert retry_message(1) == 'Too many wrong attempts. Try again in 1 s.'
	assert retry_message(1500) == 'Too many wrong attempts. Try again in 2 s.'
}

fn test_only_the_three_refusals_skip_the_rehash_silently() {
	assert is_expected_rehash_refusal(.read_only)
	assert is_expected_rehash_refusal(.vault_replaced)
	assert is_expected_rehash_refusal(.changed_elsewhere)
	assert !is_expected_rehash_refusal(.io)
}

fn test_set_up_lock_and_unlock_walk_the_phases() {
	dir := test_dir('phases')
	mut s := state_in(dir)
	assert s.phase() == .uninitialised
	assert !s.can_lock_screen() && !s.can_reset() && !s.can_change()
	job := s.begin_set_up('password')!
	assert s.is_busy()
	s.begin_set_up('password') or { assert (err as VaultActionError).kind == .busy }
	s.apply(job.run())!
	assert s.phase() == .unlocked && !s.is_busy()
	assert s.take_events() == [VaultEvent.initialised]
	s.lock()
	assert s.phase() == .locked
	assert s.take_events() == [VaultEvent.locked]
	assert s.unlocked() == none
	u := s.begin_unlock('password')!
	s.apply(u.run())!
	assert s.phase() == .unlocked
	assert s.take_events() == [VaultEvent.unlocked]
	// Floor params are below the default: a rehash is scheduled and the state is busy meanwhile.
	assert s.is_busy()
	follow := s.take_followup() or { panic('expected a rehash') }
	s.apply(follow.run())!
	assert !s.is_busy() && s.phase() == .unlocked
}

fn test_wrong_passwords_back_off_from_the_fifth_and_a_success_resets_the_count() {
	dir := test_dir('backoff')
	mut s := unlocked_state(dir, 'password')
	s.lock()
	for i in 0 .. 4 {
		job := s.begin_unlock('wrong') or { panic('attempt ${i}: ${err}') }
		mut failed := false
		s.apply(job.run()) or {
			expect_vault_err(err, .wrong_password)
			failed = true
		}
		assert failed
	}
	assert s.retry_after_ms() == 0
	job := s.begin_unlock('wrong')!
	s.apply(job.run()) or {}
	assert s.retry_after_ms() > 0
	s.begin_unlock('password') or { assert (err as VaultActionError).kind == .retry_in }
	s.retry_at = 0
	s.apply(s.begin_unlock('password')!.run())!
	assert s.failures == 0
}

fn test_locked_never_rereads_the_file_and_a_missing_file_skips_the_rehash() {
	dir := test_dir('reread')
	mut s := unlocked_state(dir, 'password')
	s.lock()
	os.rm(vault_path(dir))!
	s.apply(s.begin_unlock('password')!.run())!
	assert s.phase() == .unlocked
	follow := s.take_followup() or { panic('expected a rehash') }
	s.apply(follow.run())!
	assert s.phase() == .unlocked
}

fn test_change_password_checks_the_old_one() {
	dir := test_dir('change')
	mut s := unlocked_state(dir, 'password')
	s.apply(s.begin_change('wrong', 'newpassword')!.run()) or {
		expect_vault_err(err, .wrong_password)
		assert s.failures == 1
		s.apply(s.begin_change('password', 'newpassword')!.run())!
		assert s.phase() == .unlocked
		assert s.failures == 0
		return
	}
	assert false
}

fn test_reset_announces_itself_first_then_removes_both_files() {
	dir := test_dir('reset')
	mut s := unlocked_state(dir, 'password')
	s.take_events()
	job := s.begin_reset()!
	assert s.take_events() == [VaultEvent.reset]
	assert s.phase() == .locked && s.unlocked() == none
	assert os.exists(vault_path(dir))
	s.apply(job.run())!
	assert s.phase() == .uninitialised
	assert !os.exists(vault_path(dir))
}

fn test_the_read_only_window_unlocks_but_never_writes() {
	dir := test_dir('read-only')
	mut owner := unlocked_state(dir, 'password')
	owner.lock()
	raw := os.read_file(vault_path(dir))!
	mut s := new_state(dir, true)
	assert s.phase() == .locked && s.is_read_only()
	s.begin_change('password', 'x') or { expect_vault_err(err, .read_only) }
	s.begin_reset() or { expect_vault_err(err, .read_only) }
	s.apply(s.begin_unlock('password')!.run())!
	assert s.phase() == .unlocked && !s.is_busy()
	assert os.read_file(vault_path(dir))! == raw
}

fn test_damaged_and_newer_files_are_unavailable_and_only_damaged_resets() {
	dir := test_dir('unavailable')
	os.write_file(vault_path(dir), 'not json')!
	mut s := state_in(dir)
	assert s.phase() == .unavailable && s.can_reset()
	assert s.unavailable_reason().contains('damaged')
	os.write_file(vault_path(dir), set_version(9))!
	mut newer := state_in(dir)
	assert newer.phase() == .unavailable && !newer.can_reset()
	assert newer.unavailable_reason().contains('newer')
}

fn set_version(v int) string {
	return v1_vault_json.replace('"schema_version": 1', '"schema_version": ${v}')
}

fn fail_job(mut s VaultState, kind JobKind, failure VaultErrorKind) ! {
	s.apply(JobResult{
		kind:    kind
		failure: VaultError{
			kind: failure
		}
	})!
}

fn test_a_second_set_up_after_another_process_recovers_to_locked() {
	dir := test_dir('other-process')
	mut s := state_in(dir)
	job := s.begin_set_up('password')!
	mut other := state_in(dir)
	other.apply(other.begin_set_up('password')!.run())!
	s.apply(job.run()) or { expect_vault_err(err, .already_initialised) }
	assert s.phase() == .locked
}

fn test_a_lock_during_an_unlock_wins() {
	dir := test_dir('lock-unlock')
	mut s := unlocked_state(dir, 'password')
	s.lock()
	s.take_events()
	job := s.begin_unlock('password')!
	s.lock()
	s.apply(job.run()) or {
		assert (err as VaultActionError).kind == .locked_meanwhile
		assert s.phase() == .locked
		return
	}
	assert false
}

fn test_a_lock_during_a_set_up_wins_and_keeps_the_vault() {
	dir := test_dir('lock-setup')
	mut s := state_in(dir)
	job := s.begin_set_up('password')!
	s.lock()
	s.apply(job.run())!
	assert s.phase() == .locked
	assert s.take_events() == [VaultEvent.initialised, .locked]
	assert s.unlocked() == none
}

fn test_a_lock_during_a_change_keeps_only_the_new_header() {
	dir := test_dir('lock-change')
	mut s := unlocked_state(dir, 'password')
	job := s.begin_change('password', 'newpassword')!
	s.lock()
	s.apply(job.run())!
	assert s.phase() == .locked
	s.apply(s.begin_unlock('newpassword')!.run())!
	assert s.phase() == .unlocked
}

fn test_a_failed_reset_has_already_announced_itself() {
	dir := test_dir('failed-reset')
	mut s := unlocked_state(dir, 'password')
	s.take_events()
	s.begin_reset()!
	assert s.take_events() == [VaultEvent.reset]
	fail_job(mut s, .reset, .io) or { expect_vault_err(err, .io) }
	assert s.phase() == .locked
}

fn test_a_seal_under_the_vault_on_disk_opens_again() {
	dir := test_dir('seal')
	mut s := unlocked_state(dir, 'password')
	value := s.seal(.ssh_password, ctx, 'pw')!
	v := s.unlocked() or { panic('unlocked') }
	assert v.open(.ssh_password, ctx, value)! == 'pw'
}

fn test_a_seal_after_a_reset_elsewhere_is_refused_and_drops_the_key() {
	dir := test_dir('seal-reset')
	mut s := unlocked_state(dir, 'password')
	s.take_events()
	reset(dir, false)!
	s.seal(.ssh_password, ctx, 'pw') or {
		assert (err as VaultActionError).kind == .reset_elsewhere
		assert s.phase() == .uninitialised
		assert s.take_events() == [VaultEvent.locked]
		return
	}
	assert false
}

fn test_a_screen_lock_drops_the_key_and_only_the_lock_screen_lifts_it() {
	dir := test_dir('screen')
	mut s := unlocked_state(dir, 'password')
	s.take_events()
	s.lock_screen()
	assert s.is_screen_locked() && s.phase() == .locked
	assert s.take_events() == [VaultEvent.locked, .screen_locked]
	s.begin_unlock('password') or { assert (err as VaultActionError).kind == .wrong_phase }
	assert !s.can_change() && !s.can_reset()
	s.apply(s.begin_unlock_screen('password')!.run())!
	assert !s.is_screen_locked() && s.phase() == .unlocked
}

fn test_no_vault_means_no_screen_lock() {
	mut s := state_in(test_dir('no-lock'))
	s.lock_screen()
	assert !s.is_screen_locked()
	assert s.take_events().len == 0
}

fn test_the_lock_screen_ignores_a_vault_replaced_on_disk() {
	dir := test_dir('screen-replaced')
	mut s := unlocked_state(dir, 'password')
	s.lock_screen()
	reset(dir, false)!
	os.write_file(vault_path(dir), v1_vault_json)!
	s.apply(s.begin_unlock_screen('password')!.run())!
	assert s.phase() == .unlocked
	assert !s.is_screen_locked()
}

fn test_a_usable_vault_at_start_locks_the_screen() {
	dir := test_dir('at-start')
	mut owner := unlocked_state(dir, 'password')
	owner.lock()
	assert load_at_start(dir, false).is_screen_locked()
	assert !load_at_start(test_dir('at-start-none'), false).is_screen_locked()
	os.write_file(vault_path(dir), 'not json')!
	assert !load_at_start(dir, false).is_screen_locked()
}
