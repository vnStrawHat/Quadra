// Closing a pseudo console leaves no shell behind (specs/17): a shell closed before its
// start-up finished never exits by itself and is terminated after the grace period.
module win

import time
import quadra.platform

struct ExitBox {
	exited chan int
}

fn test_a_shell_closed_at_once_still_exits() {
	for _ in 0 .. 3 {
		exited := chan int{cap: 1}
		mut pty := spawn_conpty(platform.PtyOptions{
			command: 'cmd.exe /K chcp 65001 >nul'
			cols:    80
			rows:    24
			ctx:     &ExitBox{exited}
			on_exit: fn (ctx voidptr, code int) {
				b := unsafe { &ExitBox(ctx) }
				b.exited <- code
			}
		}) or {
			eprintln('skipped: ${err}')
			return
		}
		pty.close() // at once, while cmd.exe is still starting
		mut code := -1
		select {
			c := <-exited {
				code = c
			}
			(child_exit_grace_ms + 3000) * time.millisecond {
				assert false, 'the shell is still running after the grace period'
			}
		}
		assert code != -1
	}
}
