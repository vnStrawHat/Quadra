// A vault unlock gives Argon2id's 64 MiB back: on real backends the conservative collector
// kept that buffer alive through stale words until the toolchain patch freed it explicitly
// (tools/patches/v-argon2-release.diff). The null-device harness never showed it.
module main

import os
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
	h.app.trim_now()
	locked := private_mb()
	h.type_text('correct-horse')
	h.press('enter')!
	h.wait_tasks()!
	h.frame()
	unlocked := private_mb()
	eprintln('oneterm vault memory: private ws locked ${locked} MB, unlocked ${unlocked} MB')
	assert !one.vault.is_screen_locked()
	assert locked < 48 // vault.init's Argon2 run is gone too
	assert unlocked < locked + 16 // not + 64
}
