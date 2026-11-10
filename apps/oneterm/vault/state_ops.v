// VaultState operations: the begin_* / VaultJob.run / apply protocol (specs/25).
module vault

pub enum JobKind {
	set_up
	unlock
	unlock_screen
	change
	reset
	rehash
}

// VaultJob holds what one Argon2id operation needs. run() touches nothing shared.
pub struct VaultJob {
pub:
	kind         JobKind
	dir          string
	header       VaultHeader
	password     string
	new_password string
	read_only    bool
	params       KdfParams
}

// JobResult is what run() produces, applied on the owner's thread.
pub struct JobResult {
pub:
	kind     JobKind
	opened   &UnlockedVault = unsafe { nil }
	password string
	failure  ?VaultError
}

fn rewrap(dir string, header VaultHeader, old string, new_ string, params KdfParams, read_only bool) !&UnlockedVault {
	mut v := unlock(header, old)!
	v.change_password_with(dir, old, new_, params, read_only)!
	return v
}

// run does the slow work; call it off the UI thread.
pub fn (j VaultJob) run() JobResult {
	mut opened := unsafe { &UnlockedVault(nil) }
	mut failure := ?VaultError(none)
	match j.kind {
		.set_up {
			opened = init_with(j.dir, j.password, j.params, j.read_only) or {
				failure = io_error(err)
				unsafe { nil }
			}
		}
		.unlock, .unlock_screen {
			opened = unlock(j.header, j.password) or {
				failure = io_error(err)
				unsafe { nil }
			}
		}
		.change {
			opened = rewrap(j.dir, j.header, j.password, j.new_password, j.params, j.read_only) or {
				failure = io_error(err)
				unsafe { nil }
			}
		}
		.rehash {
			opened = rewrap(j.dir, j.header, j.password, j.password, j.params, j.read_only) or {
				failure = io_error(err)
				unsafe { nil }
			}
		}
		.reset {
			reset(j.dir, j.read_only) or { failure = io_error(err) }
		}
	}

	return JobResult{
		kind:     j.kind
		opened:   opened
		password: j.password
		failure:  failure
	}
}

fn (s &VaultState) job(kind JobKind, header VaultHeader, password string, new_password string) VaultJob {
	return VaultJob{
		kind:         kind
		dir:          s.dir
		header:       header
		password:     password
		new_password: new_password
		read_only:    s.read_only
		params:       s.params
	}
}

// seal re-reads vault.json first: when it is no longer the vault this process unlocked, the
// key is dropped and the seal refused, so nothing is sealed under a vault nobody can open.
pub fn (mut s VaultState) seal(purpose SecretPurpose, context SealContext, plaintext string) !SealedSecret {
	if s.phase != .unlocked {
		return action_err(.wrong_phase)
	}
	status := load(s.dir) or {
		VaultStatus{
			kind: .damaged
		}
	}
	if status.kind != .ready || status.header.vault_id != s.unlocked_vault.vault_id() {
		s.read_phase()
		s.events << VaultEvent.locked
		return action_err(.reset_elsewhere)
	}
	return s.unlocked_vault.seal(purpose, context, plaintext) or {
		return vault_err(err as VaultError)
	}
}

// begin_set_up: Uninitialised -> Unlocked (Locked when a lock was asked for while it ran).
pub fn (mut s VaultState) begin_set_up(password string) !VaultJob {
	s.check_write(s.phase == .uninitialised)!
	s.begin()
	return s.job(.set_up, VaultHeader{}, password, '')
}

// begin_unlock: Locked -> Unlocked, verified against the header in memory.
pub fn (mut s VaultState) begin_unlock(password string) !VaultJob {
	if s.phase != .locked || s.screen_locked {
		return action_err(.wrong_phase)
	}
	s.check_attempt()!
	s.begin()
	return s.job(.unlock, s.header, password, '')
}

// begin_unlock_screen lifts the screen lock with `password`.
pub fn (mut s VaultState) begin_unlock_screen(password string) !VaultJob {
	if s.phase != .locked || !s.screen_locked {
		return action_err(.wrong_phase)
	}
	s.check_attempt()!
	s.begin()
	return s.job(.unlock_screen, s.header, password, '')
}

// begin_change changes the master password; `old` is checked against the header in memory.
pub fn (mut s VaultState) begin_change(old string, new_password string) !VaultJob {
	s.check_write(s.can_change())!
	s.check_attempt()!
	header := s.current_header() or { return action_err(.wrong_phase) }
	s.begin()
	return s.job(.change, header, old, new_password)
}

// lock drops the key: Unlocked -> Locked. A running job is told too, and never puts a key back.
pub fn (mut s VaultState) lock() {
	if s.busy {
		s.locked_meanwhile = true
	}
	if s.phase != .unlocked {
		return
	}
	s.header = s.unlocked_vault.header()
	s.set_phase(.locked)
	s.events << VaultEvent.locked
}

// lock_screen covers every window and drops the key. Does nothing without a vault.
pub fn (mut s VaultState) lock_screen() {
	if !s.can_lock_screen() {
		return
	}
	if s.busy {
		s.locked_meanwhile = true
	}
	if s.phase == .unlocked {
		s.header = s.unlocked_vault.header()
		s.set_phase(.locked)
		s.events << VaultEvent.locked
	}
	if !s.screen_locked {
		s.screen_locked = true
		s.events << VaultEvent.screen_locked
	}
}

// begin_reset drops the key at once and emits Reset before the file goes.
pub fn (mut s VaultState) begin_reset() !VaultJob {
	s.check_write(s.can_reset())!
	if s.phase == .unlocked {
		s.header = s.unlocked_vault.header()
		s.set_phase(.locked)
	}
	s.events << VaultEvent.reset
	s.begin()
	return s.job(.reset, VaultHeader{}, '', '')
}

// take_followup returns the rehash an unlock scheduled, if any: run and apply it like any job.
pub fn (mut s VaultState) take_followup() ?VaultJob {
	job := s.followup or { return none }
	s.followup = none
	return job
}

// apply puts a finished job's result into the state. Returns the outcome for the dialog.
pub fn (mut s VaultState) apply(r JobResult) ! {
	match r.kind {
		.set_up { s.finish_set_up(r)! }
		.unlock { s.finish_unlock(r, false)! }
		.unlock_screen { s.finish_unlock(r, true)! }
		.change { s.finish_change(r)! }
		.reset { s.finish_reset(r)! }
		.rehash { s.finish_rehash(r) }
	}
}
