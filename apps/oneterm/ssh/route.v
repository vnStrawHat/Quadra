// The route of a connection: TCP to the first hop, a direct-tcpip channel of the previous hop
// for each later one, one authentication per hop. Deadlines and cancellation live in Guard.
module ssh

import sync
import time

const phase_deadline = 20 * time.second
const connect_deadline = 60 * time.second

@[heap]
struct Guard {
mut:
	mu       &sync.Mutex     = sync.new_mutex()
	wake     &sync.Semaphore = sync.new_semaphore()
	cancel   &Cancel
	stream   ?Stream // the outermost stream, closed to unblock on timeout or cancel
	phase    ConnectPhase
	phase_at time.Time
	start_at time.Time
	expired  bool
	overall  bool
	done     bool
}

fn new_guard(cancel &Cancel) &Guard {
	mut g := &Guard{
		cancel:   cancel
		phase_at: time.now()
		start_at: time.now()
	}
	mut wake := g.wake
	mut c := g.cancel
	c.set_hook(fn [mut wake] () {
		wake.post()
	})
	return g
}

fn (mut g Guard) watch() {
	for {
		g.wake.timed_wait(100 * time.millisecond)
		g.mu.@lock()
		if g.done {
			g.mu.unlock()
			return
		}
		mut c := g.cancel
		cancelled := c.is_cancelled()
		g.overall = time.since(g.start_at) >= connect_deadline
		if time.since(g.phase_at) >= phase_deadline || g.overall {
			g.expired = true
		}
		fire := cancelled || g.expired
		stream := g.stream
		g.mu.unlock()
		if fire {
			if mut s := stream {
				s.close()
			}
			return
		}
	}
}

fn (mut g Guard) finish() {
	g.mu.@lock()
	g.done = true
	g.mu.unlock()
	g.wake.post()
}

// enter starts a phase: its deadline restarts.
fn (mut g Guard) enter(phase ConnectPhase) {
	g.mu.@lock()
	g.phase = phase
	g.phase_at = time.now()
	g.mu.unlock()
}

fn (mut g Guard) set_stream(s Stream) {
	g.mu.@lock()
	g.stream = s
	g.mu.unlock()
}

// fail turns a blocked operation's failure into the right typed error.
fn (mut g Guard) fail(err IError) IError {
	g.mu.@lock()
	defer {
		g.mu.unlock()
	}
	mut c := g.cancel
	if c.is_cancelled() {
		return Cancelled{}
	}
	if g.expired {
		text := if g.overall {
			'connection attempt timed out after 60 s'
		} else {
			'timed out after 20 s'
		}
		return ConnectError{
			phase:   g.phase
			message: text
		}
	}
	return phase_err(g.phase, err)
}

struct DialResult {
mut:
	stream ?&TcpStream
	reason string
}

fn dial_worker(host string, port int, mut sem sync.Semaphore, mut res DialResult) {
	res.stream = dial_stream(host, port) or {
		res.reason = err.msg()
		sem.post()
		return
	}
	sem.post()
}

// dial connects by TCP on a helper thread, so a slow DNS or SYN stays cancellable.
fn (mut g Guard) dial(host string, port int) !&TcpStream {
	mut sem := sync.new_semaphore()
	mut res := &DialResult{}
	spawn dial_worker(host, port, mut sem, mut res)
	for !sem.timed_wait(50 * time.millisecond) {
		g.mu.@lock()
		expired := g.expired
		done := time.since(g.phase_at) >= phase_deadline
		mut c := g.cancel
		cancelled := c.is_cancelled()
		g.mu.unlock()
		if cancelled || expired || done {
			if done {
				g.mu.@lock()
				g.expired = true
				g.mu.unlock()
			}
			return error('dial interrupted')
		}
	}
	if stream := res.stream {
		return stream
	}
	return error(res.reason)
}

// connect_hop opens the transport to a hop and authenticates. carrier is the previous hop.
fn connect_hop(mut g Guard, carrier &Conn, mut hop Hop, cfg &Config) !&Conn {
	g.enter(.transport)
	mut stream := Stream(&ChanStream{
		ch: unsafe { nil }
	})
	if carrier != unsafe { nil } {
		mut c := unsafe { carrier }
		ch := c.open_direct_tcpip(hop.host, hop.port, '127.0.0.1', 0) or {
			return g.fail(error('the jump host refused a direct-tcpip channel to ${hop.host}:${hop.port}: ${err}'))
		}
		stream = Stream(&ChanStream{
			ch: ch
		})
	} else {
		tcp := g.dial(hop.host, hop.port) or { return g.fail(err) }
		stream = Stream(tcp)
		g.set_stream(stream)
	}
	name := host_port(hop.host, hop.port)
	known := known_keys(cfg.known_hosts_path, name) or { []KnownKey{} }
	path := cfg.known_hosts_path
	host, port, policy := hop.host, hop.port, hop.policy
	mut tr := new_transport(stream, Offer{
		host_key: preferred_algorithms(known)
	}, fn [path, host, port, policy] (key HostKey) ! {
		check_host_key(path, host, port, policy, key)!
	})
	tr.handshake() or {
		tr.close()
		return g.fail(err)
	}
	g.enter(.authentication)
	authenticate(mut tr, hop.username, mut hop.auth, cfg.agent_pipe) or {
		tr.close()
		return g.fail(err)
	}
	mut conn := new_conn(tr)
	conn.start(cfg.keepalive.normalized())
	return conn
}

// hop_error prefixes connect errors with the hop; host keys and cancellation stay as they are.
fn hop_error(label string, err IError) IError {
	if err is ConnectError {
		return ConnectError{
			phase:   err.phase
			message: 'jump host ${label}: ${err.message}'
		}
	}
	return err
}
