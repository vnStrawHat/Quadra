// Fuzz: the transport and authentication messages a server sends (KEXINIT, the key exchange
// replies, host keys and signatures, EXT_INFO, DISCONNECT, userauth replies, agent replies).
module ssh

import crypto.ed25519
import encoding.base64
import quadra.testing.fuzz

const fuzz_ecdsa_blob = 'AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBFNcLXjrSuD2jj2SMAfcd9sDTgFcNqmOcdzLEZ8SSy5kr+lJ5km51aZ4nHhyk/LVGf+HRJfAjl/J8hZAKBtXlAY='

fn ed_key() PrivateKey {
	seed := []u8{len: 32, init: u8(index * 5 + 1)}
	priv := ed25519.new_key_from_seed(seed)
	return PrivateKey{
		kind:        'ssh-ed25519'
		public_blob: ed_public_blob(priv.public_key())
		seed:        seed
	}
}

fn rsa_blob(mut rng fuzz.Rng) []u8 {
	mut w := new_writer(300)
	w.put_str('ssh-rsa')
	w.put_mpint([u8(1), 0, 1])
	w.put_mpint(rng.bytes(1 + rng.intn(300)))
	return w.buf
}

fn sig_blob(alg string, sig []u8) []u8 {
	mut w := new_writer(sig.len + 32)
	w.put_str(alg)
	w.put_bytes(sig)
	return w.buf
}

fn kex_reply(kind u8, parts [][]u8) []u8 {
	mut w := new_writer(512)
	w.put_u8(kind)
	for p in parts {
		w.put_bytes(p)
	}
	return w.buf
}

fn test_short_kexinit_is_an_error() {
	// parse_kexinit sliced pkt[17..] (type and cookie) without a length check
	for n in [1, 2, 16] {
		if _ := parse_kexinit([]u8{len: n, init: msg_kexinit}) {
			assert false
		}
	}
}

fn test_fuzz_kexinit_and_host_keys() {
	n := fuzz.iters(400)
	mut rng := fuzz.new_rng(0xc0de)
	key := ed_key()
	ed_sig := key.sign('ssh-ed25519', 'data'.bytes()) or { panic(err) }
	ecdsa := base64.decode(fuzz_ecdsa_blob)
	mut w := fuzz.watch('ssh kexinit and host keys', n)
	for i in 0 .. n {
		corpus := [build_kexinit(Offer{}), key.public_blob, ecdsa, rsa_blob(mut rng), ed_sig,
			sig_blob('ecdsa-sha2-nistp256', rng.bytes(72)), sig_blob('rsa-sha2-256', rng.bytes(256))]
		input := rng.mutate(corpus[rng.intn(corpus.len)], corpus)
		w.start()
		if ki := parse_kexinit(input) {
			c := parse_kexinit(build_kexinit(Offer{})) or { panic(err) }
			negotiate(c, ki) or {}
		}
		mut hk := parse_host_key(input) or { HostKey{} }
		if hk.kind == '' {
			hk = parse_host_key(corpus[rng.intn(4) + 1]) or { HostKey{} }
		}
		for alg in ['ssh-ed25519', 'ecdsa-sha2-nistp256', 'rsa-sha2-256', 'rsa-sha2-512', 'ssh-rsa'] {
			hk.verify_sig(alg, 'data'.bytes(), input) or {}
		}
		w.done(i, input.len, 2000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}

// The key exchange replies, read by the real exchange methods over a scripted stream.
fn test_fuzz_kex_replies() {
	n := fuzz.iters(60)
	mut rng := fuzz.new_rng(0xe5c)
	key := ed_key()
	mut w := fuzz.watch('ssh kex replies', n)
	for i in 0 .. n {
		kex := ['curve25519-sha256', 'ecdh-sha2-nistp256', 'ecdh-sha2-nistp384',
			'diffie-hellman-group14-sha256', 'diffie-hellman-group-exchange-sha256'][i % 5]
		mut gex := new_writer(600)
		gex.put_u8(msg_kex_reply)
		gex.put_mpint(rng.bytes(256 + rng.intn(4)))
		gex.put_mpint([u8(2)])
		mut dh := new_writer(600)
		dh.put_u8(msg_kex_reply)
		dh.put_bytes(key.public_blob)
		dh.put_mpint(rng.bytes(256))
		dh.put_bytes(rng.bytes(83))
		corpus := [
			kex_reply(msg_kex_reply, [key.public_blob, rng.bytes(32),
				rng.bytes(83)]),
			kex_reply(msg_kex_reply, [key.public_blob, rng.bytes(65),
				rng.bytes(83)]),
			gex.buf,
			dh.buf,
		]
		mut pkts := [rng.mutate(corpus[rng.intn(corpus.len)], corpus)]
		if kex.contains('exchange') {
			pkts << rng.mutate(dh.buf, corpus)
		}
		for mut p in pkts {
			if p.len == 0 {
				p << msg_kex_reply
			}
		}
		mut t := fuzz_transport(plain_packets(pkts))
		t.v_s = 'SSH-2.0-fuzz'
		t.kexinit_out = build_kexinit(Offer{})
		neg := Negotiated{
			kex:      kex
			host_key: 'ssh-ed25519'
		}
		w.start()
		if ex := t.exchange(neg, build_kexinit(Offer{})) {
			if hk := parse_host_key(ex.k_s) {
				hk.verify_sig(neg.host_key, ex.h, ex.sig) or {}
			}
		}
		w.done(i, pkts[0].len, 5000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}

fn test_fuzz_transport_and_auth_messages() {
	n := fuzz.iters(400)
	mut rng := fuzz.new_rng(0xa07)
	mut fail := new_writer(64)
	fail.put_u8(msg_userauth_failure)
	fail.put_str('publickey,keyboard-interactive')
	fail.put_bool(false)
	mut info := new_writer(64)
	info.put_u8(msg_userauth_info_request)
	info.put_str('')
	info.put_str('')
	info.put_str('')
	info.put_u32(2)
	for echo in [false, false] {
		info.put_str('Password: ')
		info.put_bool(echo)
	}
	mut ext := new_writer(64)
	ext.put_u8(msg_ext_info)
	ext.put_u32(2)
	ext.put_str('server-sig-algs')
	ext.put_str('rsa-sha2-256,ssh-ed25519')
	ext.put_str('x')
	ext.put_str('y')
	mut disc := new_writer(64)
	disc.put_u8(msg_disconnect)
	disc.put_u32(11)
	disc.put_str('bye')
	disc.put_str('')
	mut banner := new_writer(64)
	banner.put_u8(msg_userauth_banner)
	banner.put_str('hello')
	corpus := [fail.buf, info.buf, ext.buf, disc.buf, banner.buf, [
		msg_userauth_success,
	],
		[
			msg_ignore,
		], [
			msg_service_accept,
		]]
	mut w := fuzz.watch('ssh transport and auth messages', n)
	for i in 0 .. n {
		mut pkts := [][]u8{}
		for _ in 0 .. 1 + rng.intn(4) {
			mut p := rng.mutate(corpus[rng.intn(corpus.len)], corpus)
			if p.len == 0 {
				p = [msg_ignore]
			}
			pkts << p
		}
		w.start()
		parse_outcome(pkts[0]) or {}
		disconnect_reason(pkts[0])
		mut t := fuzz_transport(plain_packets(pkts))
		t.parse_ext_info(pkts[0])
		auth_password(mut t, 'user', 'secret'.bytes()) or {}
		w.done(i, pkts[0].len, 2000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}

// FuzzAgent answers every query with a fixed reply.
struct FuzzAgent {
	reply []u8
}

fn (mut a FuzzAgent) query(request []u8) ![]u8 {
	return a.reply
}

fn (mut a FuzzAgent) close() {}

fn test_fuzz_agent_replies() {
	n := fuzz.iters(400)
	mut rng := fuzz.new_rng(0xa6e)
	key := ed_key()
	mut ids := new_writer(128)
	ids.put_u8(agent_identities_answer)
	ids.put_u32(1)
	ids.put_bytes(key.public_blob)
	ids.put_str('me@host')
	mut sig := new_writer(128)
	sig.put_u8(agent_sign_response)
	sig.put_bytes(key.sign('ssh-ed25519', 'x'.bytes()) or { panic(err) })
	corpus := [ids.buf, sig.buf, [agent_failure]]
	mut w := fuzz.watch('ssh agent replies', n)
	for i in 0 .. n {
		input := rng.mutate(corpus[rng.intn(corpus.len)], corpus)
		mut link := AgentLink(FuzzAgent{input})
		w.start()
		agent_identities(mut link) or {}
		agent_sign(mut link, key.public_blob, 'x'.bytes(), 0) or {}
		w.done(i, input.len, 2000)
		fuzz_ceiling(i)
	}
	w.finish(64)
}

fn test_endless_packets_during_kex_are_an_error() {
	// packets held while a key exchange runs had no bound
	mut pkts := [][]u8{len: 100, init: [msg_channel_data, 0, 0, 0, 0, 0, 0, 0, 1, `x`]}
	pkts << [msg_newkeys]
	mut t := fuzz_transport(plain_packets(pkts))
	if _ := t.read_kex_packet() {
		assert false
	}
	mut few := [][]u8{len: 3, init: [msg_channel_data, 0, 0, 0, 0, 0, 0, 0, 1, `x`]}
	few << [msg_newkeys]
	mut t2 := fuzz_transport(plain_packets(few))
	assert (t2.read_kex_packet() or { panic(err) }) == [msg_newkeys]
}
