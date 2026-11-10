// Applying finished vault jobs to the state (specs/25).
module vault

import time

fn (mut s VaultState) record_attempt(ok bool, wrong_password bool) {
	if ok {
		s.failures = 0
		s.retry_at = 0
	} else if wrong_password {
		s.failures++
		wait := backoff_after(s.failures)
		s.retry_at = if wait > 0 { time.ticks() + wait } else { 0 }
	}
}

fn (mut s VaultState) finish_set_up(r JobResult) ! {
	locked_meanwhile := s.end()
	if failure := r.failure {
		if failure.kind == .already_initialised {
			// Another process was first: leaving Uninitialised only makes this one stricter.
			s.read_phase()
		}
		return vault_err(failure)
	}
	s.failures = 0
	s.retry_at = 0
	s.events << VaultEvent.initialised
	if locked_meanwhile {
		s.header = r.opened.header()
		s.set_phase(.locked)
		s.events << VaultEvent.locked
	} else {
		s.unlocked_vault = r.opened
		s.phase = .unlocked
	}
}

fn (mut s VaultState) finish_unlock(r JobResult, is_screen bool) ! {
	locked_meanwhile := s.end()
	if (is_screen && !s.is_screen_locked()) || (!is_screen && s.phase != .locked) {
		return action_err(.wrong_phase)
	}
	if failure := r.failure {
		s.record_attempt(false, failure.kind == .wrong_password)
		return vault_err(failure)
	}
	s.record_attempt(true, false)
	if locked_meanwhile {
		return action_err(.locked_meanwhile)
	}
	if is_screen {
		s.screen_locked = false
	}
	s.keep_unlocked(r.opened, r.password)
}

fn (mut s VaultState) keep_unlocked(v &UnlockedVault, password string) {
	needs_rehash := v.needs_rehash()
	s.unlocked_vault = unsafe { v }
	s.phase = .unlocked
	s.events << VaultEvent.unlocked
	if needs_rehash && !s.read_only {
		s.begin()
		s.followup = s.job(.rehash, v.header(), password, '')
	}
}

fn (mut s VaultState) finish_change(r JobResult) ! {
	locked_meanwhile := s.end()
	if failure := r.failure {
		s.record_attempt(false, failure.kind == .wrong_password)
		return vault_err(failure)
	}
	s.record_attempt(true, false)
	if locked_meanwhile {
		// vault.json now holds the new header; keep only that.
		s.header = r.opened.header()
		s.set_phase(.locked)
		return
	}
	was_locked := s.phase == .locked
	s.set_phase(.locked) // drops the old instance's key before taking the new one
	s.unlocked_vault = r.opened
	s.phase = .unlocked
	if was_locked {
		s.events << VaultEvent.unlocked
	}
}

fn (mut s VaultState) finish_reset(r JobResult) ! {
	s.end()
	if failure := r.failure {
		return vault_err(failure)
	}
	s.set_phase(.uninitialised)
	s.screen_locked = false
	s.failures = 0
	s.retry_at = 0
}

fn (mut s VaultState) finish_rehash(r JobResult) {
	locked_meanwhile := s.end()
	if failure := r.failure {
		if !is_expected_rehash_refusal(failure.kind) {
			eprintln('vault rehash failed: ${failure.msg()}')
		}
		return
	}
	match s.phase {
		.unlocked {
			if locked_meanwhile {
				s.header = r.opened.header()
				s.set_phase(.locked)
			} else {
				s.set_phase(.locked)
				s.unlocked_vault = r.opened
				s.phase = .unlocked
			}
		}
		.locked {
			s.header = r.opened.header()
		}
		else {}
	}
}
