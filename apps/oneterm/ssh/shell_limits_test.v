// The SSH shell's write queue (specs/17, Limits).
module ssh

import quadra.platform

fn test_the_write_queue_holds_256_writes_and_4_mib() {
	mut p := new_shell_pty(unsafe { nil })
	for _ in 0 .. platform.pty_write_items {
		p.write([u8(1)])!
	}
	p.write([u8(1)]) or {
		assert err.msg() == 'SSH write queue is full'
		mut q := new_shell_pty(unsafe { nil })
		q.write([]u8{len: platform.pty_write_budget})!
		q.write([u8(1)]) or { return }
		assert false, 'past the budget'
		return
	}
	assert false, 'past the item cap'
}
