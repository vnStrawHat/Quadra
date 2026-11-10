// A vault unlock gives Argon2id's 64 MiB back: on real backends the conservative collector
// kept that buffer alive through stale words until the toolchain patch freed it explicitly
// (tools/patches/v-argon2-release.diff). The null-device harness never showed it.
module main

import os
import time
import vault
import quadra.kit.term
import quadra.native.sys
import quadra.testing

fn private_mb() u64 {
	return sys.process_counters().private_ws >> 20
}

fn test_a_vault_unlock_keeps_no_argon2_memory() {
	dir := '.tmp/oneterm-vault-memory'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	vault.init(dir, 'correct-horse', false)!
	mut h := testing.new_pixel_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	one.vault = vault.load_at_start(dir, false)
	h.open(one, width: 1000, height: 700)!
	h.frame()
	r := h.find('lock-input') or { panic('no lock input') }
	assert r.height == 32 // the original's medium input
	h.app.trim_now()
	locked := private_mb()
	h.type_text('correct-horse')
	h.press('enter')!
	// No long main-thread step while the job runs or when its result lands: the unlock
	// once ran two full collections on that frame.
	mut longest := i64(0)
	for _ in 0 .. 1000 {
		time.sleep(2 * time.millisecond)
		sw := time.new_stopwatch()
		h.plat.run_posted()
		h.frame()
		step := sw.elapsed().microseconds()
		if step > longest {
			longest = step
		}
		if h.app.tasks_running() == 0 {
			break
		}
	}
	assert h.app.tasks_running() == 0, 'the unlock job did not finish'
	unlocked := private_mb()
	eprintln('oneterm vault memory: private ws locked ${locked} MB, unlocked ${unlocked} MB; longest step ${longest} us')
	assert longest < 50_000
	assert !one.vault.is_screen_locked()
	assert locked < 48 // vault.init's Argon2 run is gone too
	assert unlocked < locked + 16 // not + 64
}
