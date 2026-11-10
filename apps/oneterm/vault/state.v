// VaultState: the process-wide phase of the vault (specs/25). The key is held only while
// Unlocked; Locked never re-reads vault.json, so replacing the file cannot open this process.
// Argon2id runs off the UI thread: begin_* checks and marks busy, VaultJob.run does the work,
// apply puts the result into the state. A lock asked for while a job runs wins.
module vault

import math
import time

pub const min_password_chars = 8
const free_attempts = u32(4)
const max_backoff_ms = i64(30_000)

pub enum VaultPhase {
	uninitialised
	locked
	unlocked
	unavailable
}

pub enum VaultEvent {
	initialised   // a master password was set up; the vault is unlocked
	unlocked      // the key was loaded into memory
	locked        // the key was dropped (not only by the user: a refused seal too)
	screen_locked // every window is now covered by the lock screen
	reset         // a reset has started, before vault.json is removed: owners wipe sealed values
}

pub enum ActionErrorKind {
	vault
	retry_in
	busy
	wrong_phase
	locked_meanwhile
	reset_elsewhere
}

// VaultActionError says why a VaultState operation did not happen; msg() is dialog text.
pub struct VaultActionError {
	Error
pub:
	kind     ActionErrorKind
	vault    VaultError
	retry_ms i64
}

pub fn (e VaultActionError) msg() string {
	return match e.kind {
		.vault { vault_message(e.vault) }
		.retry_in { retry_message(e.retry_ms) }
		.busy { 'Another master password operation is still running.' }
		.locked_meanwhile { 'OneTerm was locked while the password was checked. Unlock again.' }
		.reset_elsewhere { 'The master password was reset in another OneTerm window. Nothing was saved.' }
		.wrong_phase { 'The master password changed in the meantime. Close this dialog and try again.' }
	}
}

fn vault_message(e VaultError) string {
	return match e.kind {
		.wrong_password { 'Wrong master password.' }
		.empty_password { 'Enter the master password.' }
		.already_initialised { 'A master password was already set up in another OneTerm window.' }
		.vault_replaced { 'The master password was reset or replaced in another OneTerm window. Restart OneTerm to use the current one.' }
		.changed_elsewhere { 'The master password was changed in another OneTerm window. Restart OneTerm to use it.' }
		.read_only { 'An administrator window cannot change the master password.' }
		else { 'The master password operation failed: ${e.msg()}.' }
	}
}

// retry_message is rounded up, so it never reads 0 while still waiting.
pub fn retry_message(wait_ms i64) string {
	seconds := math.max((wait_ms + 999) / 1000, 1)
	return 'Too many wrong attempts. Try again in ${seconds} s.'
}

// backoff_after: none for the first four wrong passwords, then 2, 4, 8, 16 s, then 30 s for
// every further one. In memory only; never a lockout and never a wipe.
fn backoff_after(failures u32) i64 {
	if failures <= free_attempts {
		return 0
	}
	over := failures - free_attempts
	seconds := if over >= 16 { max_backoff_ms / 1000 } else { i64(u64(1) << over) }
	return math.min(seconds * 1000, max_backoff_ms)
}

fn vault_err(e VaultError) VaultActionError {
	return VaultActionError{
		kind:  .vault
		vault: e
	}
}

fn action_err(kind ActionErrorKind) VaultActionError {
	return VaultActionError{
		kind: kind
	}
}

fn is_expected_rehash_refusal(kind VaultErrorKind) bool {
	return kind in [.read_only, .vault_replaced, .changed_elsewhere]
}

// VaultState is held by the app, one per process.
@[heap]
pub struct VaultState {
mut:
	dir              string
	read_only        bool
	phase            VaultPhase
	header           VaultHeader
	unlocked_vault   &UnlockedVault = unsafe { nil }
	reason           string
	resettable       bool
	failures         u32
	retry_at         i64 // time.ticks() milliseconds; 0 when none
	busy             bool
	locked_meanwhile bool
	screen_locked    bool
	events           []VaultEvent
	followup         ?VaultJob // a rehash scheduled by an unlock
	params           KdfParams = kdf_default
}

// new_state reads `<dir>/vault.json` once. `read_only` is the elevated window's switch.
pub fn new_state(dir string, read_only bool) &VaultState {
	mut s := &VaultState{
		dir:       dir
		read_only: read_only
	}
	s.read_phase()
	return s
}

// load_at_start is new_state as the process starts: a usable vault covers every window until
// its master password is typed. No event is emitted.
pub fn load_at_start(dir string, read_only bool) &VaultState {
	mut s := new_state(dir, read_only)
	s.screen_locked = s.can_lock_screen()
	return s
}

fn (mut s VaultState) read_phase() {
	status := load(s.dir) or {
		s.set_phase(.unavailable)
		s.reason = '${err.msg()}.'
		s.resettable = false
		return
	}
	match status.kind {
		.uninitialised {
			s.set_phase(.uninitialised)
		}
		.ready {
			s.set_phase(.locked)
			s.header = status.header
		}
		.damaged {
			s.set_phase(.unavailable)
			s.reason = 'vault.json is damaged (${status.reason}). Reset is the only way back.'
			s.resettable = true
		}
		.unsupported {
			s.set_phase(.unavailable)
			s.reason = 'vault.json was created by a newer OneTerm (${status.reason}).'
			s.resettable = false
		}
	}
}

// set_phase is the only way the phase changes, and leaving Unlocked zeroes the key.
fn (mut s VaultState) set_phase(phase VaultPhase) {
	if phase != .unlocked && s.unlocked_vault != unsafe { nil } {
		s.unlocked_vault.lock()
		s.unlocked_vault = unsafe { nil }
	}
	s.phase = phase
}

// phase is what the UI sees.
pub fn (s &VaultState) phase() VaultPhase {
	return s.phase
}

// take_events drains what happened since the last call, oldest first.
pub fn (mut s VaultState) take_events() []VaultEvent {
	events := s.events
	s.events = []
	return events
}

// is_screen_locked: the lock screen covers every window, and the key is not in memory.
pub fn (s &VaultState) is_screen_locked() bool {
	return s.screen_locked && s.phase == .locked
}

// can_lock_screen: no vault, or an unusable one, means no lock.
pub fn (s &VaultState) can_lock_screen() bool {
	return s.phase in [.locked, .unlocked]
}

// unavailable_reason says why the phase is unavailable.
pub fn (s &VaultState) unavailable_reason() string {
	return if s.phase == .unavailable { s.reason } else { '' }
}

pub fn (s &VaultState) is_read_only() bool {
	return s.read_only
}

pub fn (s &VaultState) is_busy() bool {
	return s.busy
}

// can_change: a vault exists and this window may write it.
pub fn (s &VaultState) can_change() bool {
	return !s.read_only && !s.screen_locked && s.phase in [.locked, .unlocked]
}

// can_reset: a forgotten password cannot be proved, so Locked qualifies.
pub fn (s &VaultState) can_reset() bool {
	if s.read_only || s.screen_locked {
		return false
	}
	return s.phase in [.locked, .unlocked] || (s.phase == .unavailable && s.resettable)
}

// retry_after_ms is how long until the next attempt is accepted, 0 when none.
pub fn (s &VaultState) retry_after_ms() i64 {
	return if s.retry_at == 0 { 0 } else { math.max(s.retry_at - time.ticks(), 0) }
}

// unlocked_vault is the vault for sealing and opening; none unless unlocked.
pub fn (s &VaultState) unlocked() ?&UnlockedVault {
	return if s.phase == .unlocked { s.unlocked_vault } else { none }
}

fn (mut s VaultState) current_header() ?VaultHeader {
	return match s.phase {
		.locked { s.header }
		.unlocked { s.unlocked_vault.header() }
		else { none }
	}
}

fn (s &VaultState) check_write(applies bool) ! {
	if s.read_only {
		return vault_err(VaultError{ kind: .read_only })
	}
	if s.busy {
		return action_err(.busy)
	}
	if !applies || s.screen_locked {
		return action_err(.wrong_phase)
	}
}

fn (s &VaultState) check_attempt() ! {
	if s.busy {
		return action_err(.busy)
	}
	wait := s.retry_after_ms()
	if wait > 0 {
		return VaultActionError{
			kind:     .retry_in
			retry_ms: wait
		}
	}
}

fn (mut s VaultState) begin() {
	s.busy = true
	s.locked_meanwhile = false
}

// end is how a job finishes; returns whether a lock was asked for while it ran.
fn (mut s VaultState) end() bool {
	s.busy = false
	meanwhile := s.locked_meanwhile
	s.locked_meanwhile = false
	return meanwhile
}
