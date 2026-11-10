// Port forwards: local and dynamic listeners, remote forwards, the relays (specs/23).
module ssh

import net
import sync

// ForwardTarget is where connections of a remote forward go.
struct ForwardTarget {
	host string
	port int
}

@[heap]
struct Forwards {
mut:
	conn      &Conn
	mu        &sync.Mutex = sync.new_mutex()
	listeners []&net.TcpListener
	active    map[int]&net.TcpConn
	remote    map[string]ForwardTarget
	next      int
	stopped   bool
	cfg       &Config
}

fn new_forwards(conn &Conn, cfg &Config) &Forwards {
	mut f := &Forwards{
		conn: conn
		cfg:  cfg
	}
	mut c := unsafe { conn }
	c.forwards = f
	for spec in cfg.port_forwards {
		f.start(spec)
	}
	return f
}

fn (mut f Forwards) start(spec PortForward) {
	match spec {
		LocalForward {
			f.listen('${spec.bind}:${spec.bind_port}', spec.target_host, spec.target_port, false)
		}
		DynamicForward {
			f.listen('${spec.bind}:${spec.bind_port}', '', 0, true)
		}
		RemoteForward {
			f.request_remote(spec)
		}
	}
}

fn (mut f Forwards) warn(msg string) {
	warn(f.cfg, msg)
}

fn (mut f Forwards) listen(addr string, host string, port int, socks bool) {
	mut l := net.listen_tcp(.ip, addr, net.ListenOptions{}) or {
		f.warn('Port forward ${addr} not started: ${err}. The session stays connected.')
		return
	}
	f.mu.@lock()
	f.listeners << l
	f.mu.unlock()
	spawn f.accept_loop(mut l, host, port, socks)
}

// bound_ports lists the local listening ports of Local and Dynamic forwards.
pub fn (s &Session) bound_ports() []int {
	if s.forwards == unsafe { nil } {
		return []
	}
	mut f := s.forwards
	f.mu.@lock()
	defer {
		f.mu.unlock()
	}
	mut out := []int{}
	for l in f.listeners {
		addr := l.addr() or { continue }
		out << int(addr.port() or { 0 })
	}
	return out
}

fn (mut f Forwards) request_remote(spec RemoteForward) {
	mut w := new_writer(64)
	w.put_str(spec.bind_host)
	w.put_u32(u32(spec.bind_port))
	mut waiter := f.conn.global_request('tcpip-forward', w.buf, true, false) or {
		f.warn('Remote forward ${spec.bind_host}:${spec.bind_port} refused by the server: ${err}. The session stays connected.')
		return
	}
	if !waiter.wait() {
		f.warn('Remote forward ${spec.bind_host}:${spec.bind_port} refused by the server. The session stays connected.')
		return
	}
	mut port := spec.bind_port
	if spec.bind_port == 0 {
		mut r := new_reader(waiter.payload)
		port = int(r.get_u32() or { 0 })
	}
	f.mu.@lock()
	f.remote['${spec.bind_host}:${port}'] = ForwardTarget{spec.target_host, spec.target_port}
	f.mu.unlock()
}

// remote_target finds the local target of a forwarded-tcpip open; none refuses it.
fn (mut f Forwards) remote_target(addr string, port u32) ?ForwardTarget {
	f.mu.@lock()
	defer {
		f.mu.unlock()
	}
	return f.remote['${addr}:${port}'] or { return none }
}

fn (mut f Forwards) accept_loop(mut l net.TcpListener, host string, port int, socks bool) {
	for {
		mut conn := l.accept() or {
			f.mu.@lock()
			stopped := f.stopped
			f.mu.unlock()
			if stopped {
				return
			}
			continue
		}
		conn.set_read_timeout(net.infinite_timeout)
		conn.set_write_timeout(net.infinite_timeout)
		spawn f.relay_inbound(mut conn, host, port, socks)
	}
}

// relay_inbound opens a direct-tcpip channel for one accepted connection.
fn (mut f Forwards) relay_inbound(mut tcp net.TcpConn, host_ string, port_ int, socks bool) {
	mut host := host_
	mut port := port_
	if socks {
		host, port = socks5_target(mut tcp) or {
			tcp.close() or {}
			return
		}
	}
	peer := tcp.peer_addr() or { net.Addr{} }
	// ponytail: Addr.str needs inet_ntop, absent from tcc headers; the originator address is
	// only informational to the server, so loopback stands in with the real port.
	mut ch := f.conn.open_direct_tcpip(host, port, '127.0.0.1', int(peer.port() or { 0 })) or {
		if socks {
			tcp.write(socks5_reply(0x01)) or {}
		}
		tcp.close() or {}
		return
	}
	if socks {
		tcp.write(socks5_reply(0x00)) or {
			tcp.close() or {}
			ch.close()
			return
		}
	}
	f.bridge(mut tcp, mut ch)
}

// relay_outbound connects a forwarded channel to its local target.
fn (mut f Forwards) relay_outbound(mut ch Channel, target ForwardTarget) {
	mut tcp := net.dial_tcp('${target.host}:${target.port}') or {
		ch.close()
		return
	}
	tcp.set_read_timeout(net.infinite_timeout)
	tcp.set_write_timeout(net.infinite_timeout)
	f.bridge(mut tcp, mut ch)
}

// bridge runs both directions and half-closes as each ends.
fn (mut f Forwards) bridge(mut tcp net.TcpConn, mut ch Channel) {
	f.mu.@lock()
	if f.stopped {
		f.mu.unlock()
		tcp.close() or {}
		ch.close()
		return
	}
	id := f.next
	f.next++
	f.active[id] = tcp
	f.mu.unlock()
	t1 := spawn pump_to_channel(mut tcp, mut ch)
	t2 := spawn pump_to_tcp(mut ch, mut tcp)
	t1.wait()
	t2.wait()
	f.mu.@lock()
	f.active.delete(id)
	f.mu.unlock()
	tcp.close() or {}
	ch.close()
}

fn pump_to_channel(mut tcp net.TcpConn, mut ch Channel) {
	mut buf := []u8{len: 32 * 1024}
	for {
		n := tcp.read(mut buf) or { break }
		if n == 0 {
			break
		}
		ch.write(buf[..n]) or { break }
	}
	ch.send_eof() or {}
}

fn pump_to_tcp(mut ch Channel, mut tcp net.TcpConn) {
	mut buf := []u8{len: 32 * 1024}
	for {
		n := ch.read(mut buf) or { break }
		tcp.write(buf[..n]) or { break }
	}
	tcp_shutdown_send(tcp)
}

// stop closes the listeners and every relay; idempotent.
fn (mut f Forwards) stop() {
	f.mu.@lock()
	f.stopped = true
	listeners := f.listeners
	active := f.active.clone()
	f.mu.unlock()
	for l in listeners {
		mut listener := unsafe { l }
		listener.close() or {}
	}
	for _, c in active {
		mut conn := unsafe { c }
		conn.close() or {}
	}
}
