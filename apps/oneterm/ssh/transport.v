// The SSH transport layer: version exchange and packets (RFC 4253). One reader (the connection
// thread) calls read_*; any thread may call send; rekey gates ordinary senders (specs/23 D3).
module ssh

import sync
import time

const client_version = 'SSH-2.0-OneTerm_V_0.6.11'
const max_packet_len = 512 * 1024
const rekey_bytes = u64(1) << 30
const rekey_after = 3600 * time.second

const msg_disconnect = u8(1)
const msg_ignore = u8(2)
const msg_unimplemented = u8(3)
const msg_debug = u8(4)
const msg_service_request = u8(5)
const msg_service_accept = u8(6)
const msg_ext_info = u8(7)
const msg_kexinit = u8(20)
const msg_newkeys = u8(21)

@[heap]
struct Transport {
mut:
	stream      Stream
	rx          []u8 = []u8{len: 64 * 1024}
	rx_pos      int
	rx_end      int
	enc         Sealer = PlainSealer{}
	dec         Sealer = PlainSealer{}
	send_seq    u32
	recv_seq    u32
	send_mu     &sync.Mutex = sync.new_mutex()
	send_cond   &sync.Cond  = unsafe { nil }
	gated       bool // a key exchange is running: ordinary sends wait
	kexinit_out []u8 // our KEXINIT of the exchange in progress, empty when none
	v_s         string
	host_blob   []u8
	session_id  []u8
	strict      bool
	first_done  bool
	sig_algs    []string // server-sig-algs from SSH_MSG_EXT_INFO
	has_sig     bool
	deferred    [][]u8 // packets that arrived during a rekey
	sent_bytes  u64
	recv_bytes  u64
	last_kex    time.Time
	offer       Offer
	on_hostkey  HostCheck
	closing     bool
}

fn new_transport(stream Stream, offer Offer, on_hostkey HostCheck) &Transport {
	mut t := &Transport{
		stream:     stream
		offer:      offer
		on_hostkey: on_hostkey
		last_kex:   time.now()
	}
	t.send_cond = sync.new_cond(t.send_mu)
	return t
}

fn (mut t Transport) read_exact(n int) ![]u8 {
	mut out := []u8{len: n}
	mut got := 0
	for got < n {
		if t.rx_pos == t.rx_end {
			t.rx_pos = 0
			t.rx_end = t.stream.read(mut t.rx)!
		}
		take := if n - got < t.rx_end - t.rx_pos { n - got } else { t.rx_end - t.rx_pos }
		copy(mut out[got..], t.rx[t.rx_pos..t.rx_pos + take])
		t.rx_pos += take
		got += take
	}
	return out
}

// read_line returns one line of the version exchange, without CR LF.
fn (mut t Transport) read_line() !string {
	mut line := []u8{cap: 128}
	for line.len < 8192 {
		b := t.read_exact(1)!
		if b[0] == `\n` {
			if line.len > 0 && line.last() == `\r` {
				line.pop()
			}
			return line.bytestr()
		}
		line << b[0]
	}
	return error('server identification line too long')
}

fn (mut t Transport) exchange_versions() ! {
	t.stream.write('${client_version}\r\n'.bytes())!
	for _ in 0 .. 1024 {
		line := t.read_line()!
		if line.starts_with('SSH-') {
			if !line.starts_with('SSH-2.0-') && !line.starts_with('SSH-1.99-') {
				return error('unsupported SSH protocol: ${line}')
			}
			t.v_s = line
			return
		}
	}
	return error('no SSH identification from the server')
}

// handshake runs the version exchange and the first key exchange.
fn (mut t Transport) handshake() ! {
	t.exchange_versions()!
	t.send_kexinit()!
	pkt := t.read_kex_packet()!
	if pkt[0] != msg_kexinit {
		return error('expected KEXINIT, got message ${pkt[0]}')
	}
	t.run_kex(pkt)!
	t.first_done = true
}

// read_raw returns the next packet's payload, handling only the sequence number.
fn (mut t Transport) read_raw() ![]u8 {
	mut first := t.read_exact(t.dec.first_len())!
	plen := t.dec.read_len(t.recv_seq, first)!
	if plen < 1 || plen > max_packet_len {
		return error('bad packet length ${plen}')
	}
	total := 4 + plen + t.dec.mac_len()
	mut wire := []u8{cap: total}
	wire << first
	wire << t.read_exact(total - first.len)!
	body := t.dec.open(t.recv_seq, mut wire)!
	t.recv_seq++
	t.recv_bytes += u64(total)
	pad := int(body[0])
	if body.len < 1 + pad + 1 {
		return error('bad packet padding')
	}
	return body[1..body.len - pad]
}

fn disconnect_reason(pkt []u8) string {
	mut r := new_reader(pkt[1..])
	code := r.get_u32() or { 0 }
	desc := r.get_str() or { '' }
	return 'server disconnected (${code}): ${desc}'
}

// handle_common processes a transport message; true when the packet was consumed.
fn (mut t Transport) handle_common(pkt []u8) !bool {
	match pkt[0] {
		msg_disconnect {
			return error(disconnect_reason(pkt))
		}
		msg_ignore, msg_debug, msg_unimplemented {
			return true
		}
		msg_ext_info {
			t.parse_ext_info(pkt)
			return true
		}
		else {
			return false
		}
	}
}

fn (mut t Transport) parse_ext_info(pkt []u8) {
	mut r := new_reader(pkt[1..])
	n := r.get_u32() or { return }
	for _ in 0 .. n {
		name := r.get_str() or { return }
		value := r.get_str() or { return }
		if name == 'server-sig-algs' {
			t.send_mu.@lock()
			t.has_sig = true
			t.sig_algs = value.split(',')
			t.send_mu.unlock()
		}
	}
}

// read_kex_packet reads the next packet of a running key exchange; other packets are held
// until it ends (a rekey may overlap channel traffic).
fn (mut t Transport) read_kex_packet() ![]u8 {
	for {
		pkt := t.read_raw()!
		if t.strict && !t.first_done && pkt[0] !in [msg_kexinit, msg_newkeys]
			&& (pkt[0] < 30 || pkt[0] > 49) && pkt[0] != msg_disconnect {
			return error('strict key exchange: unexpected message ${pkt[0]}')
		}
		if pkt[0] == msg_kexinit || pkt[0] == msg_newkeys || (pkt[0] >= 30 && pkt[0] <= 49) {
			return pkt
		}
		if !t.handle_common(pkt)! {
			t.deferred << pkt.clone()
		}
	}
	return error('unreachable')
}

// next_packet returns the next packet for the layer above, running a rekey when one arrives.
fn (mut t Transport) next_packet() ![]u8 {
	for {
		if t.deferred.len > 0 {
			pkt := t.deferred.first()
			t.deferred.delete(0)
			return pkt
		}
		pkt := t.read_raw()!
		if t.handle_common(pkt)! {
			continue
		}
		if pkt[0] == msg_kexinit {
			t.run_kex(pkt)!
			continue
		}
		return pkt
	}
	return error('unreachable')
}

// seal_and_send encrypts and writes one payload. The caller holds send_mu.
fn (mut t Transport) seal_and_send(payload []u8) ! {
	mut pkt := build_packet(payload, t.enc)
	wire := t.enc.seal(t.send_seq, mut pkt)!
	t.send_seq++
	t.sent_bytes += u64(wire.len)
	t.stream.write(wire)!
}

// send queues nothing: it writes the payload, waiting out a running key exchange.
fn (mut t Transport) send(payload []u8) ! {
	t.send_mu.@lock()
	defer {
		t.send_mu.unlock()
	}
	for t.gated && !t.closing {
		t.send_cond.wait()
	}
	if t.closing {
		return error('connection closed')
	}
	t.seal_and_send(payload)!
	if t.kexinit_out.len == 0 && (t.sent_bytes >= rekey_bytes
		|| t.recv_bytes >= rekey_bytes || time.since(t.last_kex) >= rekey_after) {
		t.send_kexinit_locked()!
	}
}

// ensure_kexinit sends our KEXINIT unless a client-started rekey already did.
fn (mut t Transport) ensure_kexinit() ! {
	t.send_mu.@lock()
	defer {
		t.send_mu.unlock()
	}
	if t.kexinit_out.len == 0 {
		t.send_kexinit_locked()!
	}
}

// send_direct bypasses the gate: only the key exchange itself uses it.
fn (mut t Transport) send_direct(payload []u8) ! {
	t.send_mu.@lock()
	defer {
		t.send_mu.unlock()
	}
	t.seal_and_send(payload)!
}

fn (mut t Transport) send_kexinit() ! {
	t.send_mu.@lock()
	defer {
		t.send_mu.unlock()
	}
	t.send_kexinit_locked()!
}

fn (mut t Transport) send_kexinit_locked() ! {
	t.gated = true
	t.kexinit_out = build_kexinit(t.offer)
	t.seal_and_send(t.kexinit_out)!
}

// close stops the stream and wakes every sender; safe to call more than once.
fn (mut t Transport) close() {
	t.send_mu.@lock()
	t.closing = true
	t.send_cond.broadcast()
	t.send_mu.unlock()
	t.stream.close()
	t.send_mu.@lock()
	t.enc.wipe_keys()
	t.send_mu.unlock()
}
