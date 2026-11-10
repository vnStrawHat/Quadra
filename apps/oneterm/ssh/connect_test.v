// Connection-attempt behaviour that needs no real server (specs/23).
module ssh

import net
import time

fn test_cancel_wakes_a_blocked_handshake() {
	mut l := net.listen_tcp(.ip, '127.0.0.1:0') or { panic(err) }
	defer {
		l.close() or {}
	}
	addr := l.addr() or { panic(err) }
	port := int(addr.port() or { 0 })
	// a server that accepts and then stays silent: the handshake blocks on the banner
	spawn fn [mut l] () {
		l.accept() or { return }
		time.sleep(5 * time.second)
	}()
	mut cancel := new_cancel()
	cfg := Config{
		target:           Hop{
			host:     '127.0.0.1'
			port:     port
			username: 'nobody'
		}
		known_hosts_path: '/nonexistent'
		cancel:           cancel
	}
	t := spawn fn [mut cancel] () {
		time.sleep(100 * time.millisecond)
		cancel.cancel()
	}()
	start := time.now()
	mut s := connect(cfg) or {
		t.wait()
		assert err is Cancelled, err.msg()
		assert time.since(start) < 3 * time.second
		return
	}
	s.close()
	assert false, 'connect must not succeed'
}

fn test_too_many_jump_hosts_is_refused() {
	mut cfg := Config{
		target: Hop{
			host: 'x'
		}
	}
	for _ in 0 .. max_jump_hops + 1 {
		cfg.jump_hops << Hop{
			host: 'y'
		}
	}
	if _ := connect(cfg) {
		assert false
	} else {
		assert err.msg().contains('too many')
	}
}

fn test_keepalive_is_clamped() {
	k := Keepalive{
		enabled: true
		seconds: 1
		max:     100
	}.normalized()
	assert k.seconds == 5 && k.max == 20
}
