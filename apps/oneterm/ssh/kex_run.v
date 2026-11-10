// Runs one key exchange to the end: method exchange, host key proof, new keys (specs/23).
module ssh

import math.big
import time
import x.crypto.curve25519
import quadra.native.cng

const msg_kex_init = u8(30)
const msg_kex_reply = u8(31)
const msg_gex_init = u8(32)
const msg_gex_reply = u8(33)
const msg_gex_request = u8(34)

// Exchanged is what every key exchange method produces.
struct Exchanged {
	k_s []u8
	sig []u8
	h   []u8
	k   []u8 // the shared secret as wire mpint bytes (length included)
}

fn hash_prefix(v_c string, v_s string, i_c []u8, i_s []u8) Writer {
	mut w := new_writer(2048)
	w.put_str(v_c)
	w.put_str(v_s)
	w.put_bytes(i_c)
	w.put_bytes(i_s)
	return w
}

fn mpint_bytes(secret []u8) []u8 {
	mut w := new_writer(secret.len + 5)
	w.put_mpint(secret)
	return w.buf
}

fn (mut t Transport) run_kex(server_init []u8) ! {
	s := parse_kexinit(server_init)!
	t.ensure_kexinit()!
	c := parse_kexinit(t.kexinit_out)!
	neg := negotiate(c, s)!
	if !t.first_done && kex_strict_server in s.kex {
		t.strict = true
	}
	if s.first_follows && !(s.kex.len > 0 && s.kex[0] == neg.kex) {
		t.read_kex_packet()! // a wrong first guess: discard the packet that follows
	}
	ex := t.exchange(neg, server_init)!
	key := parse_host_key(ex.k_s)!
	key.verify_sig(neg.host_key, ex.h, ex.sig)!
	if t.session_id.len == 0 {
		t.session_id = ex.h.clone()
	}
	if !t.first_done {
		t.on_hostkey.check(key)!
		t.host_blob = ex.k_s.clone()
	} else if ex.k_s != t.host_blob {
		return error('the server changed its host key during a rekey')
	}
	mut w := new_writer(1)
	w.put_u8(msg_newkeys)
	t.send_direct(w.buf)!
	mut enc := t.make_keys(neg, ex, true)!
	t.send_mu.@lock()
	t.enc.wipe_keys()
	t.enc = enc
	if t.strict {
		t.send_seq = 0
	}
	t.send_mu.unlock()
	pkt := t.read_kex_packet()!
	if pkt[0] != msg_newkeys {
		return error('expected NEWKEYS, got message ${pkt[0]}')
	}
	mut dec := t.make_keys(neg, ex, false)!
	t.dec.wipe_keys()
	t.dec = dec
	if t.strict {
		t.recv_seq = 0
	}
	t.send_mu.@lock()
	t.kexinit_out = []
	t.gated = false
	t.sent_bytes = 0
	t.recv_bytes = 0
	t.last_kex = time.now()
	t.send_cond.broadcast()
	t.send_mu.unlock()
}

fn (mut t Transport) make_keys(neg Negotiated, ex Exchanged, client_to_server bool) !Sealer {
	name := if client_to_server { neg.enc_cs } else { neg.enc_sc }
	mac := if client_to_server { neg.mac_cs } else { neg.mac_sc }
	info := cipher_info(name)!
	iv_letter := if client_to_server { `A` } else { `B` }
	key_letter := if client_to_server { `C` } else { `D` }
	mac_letter := if client_to_server { `E` } else { `F` }
	iv := derive_key(neg.kex, ex.k, ex.h, iv_letter, t.session_id, info.iv)
	mut key := derive_key(neg.kex, ex.k, ex.h, key_letter, t.session_id, info.key)
	defer {
		wipe(mut key)
	}
	mut mac_key := []u8{}
	if !info.aead {
		mac_key = derive_key(neg.kex, ex.k, ex.h, mac_letter, t.session_id, mac_info(mac)!.key)
	}
	defer {
		wipe(mut mac_key)
	}
	return make_sealer(name, mac, key, iv, mac_key)
}

fn (mut t Transport) exchange(neg Negotiated, i_s []u8) !Exchanged {
	if neg.kex.starts_with('curve25519') {
		return t.kex_curve25519(neg, i_s)
	}
	if neg.kex.starts_with('ecdh-sha2-') {
		return t.kex_ecdh(neg, i_s)
	}
	if neg.kex == 'diffie-hellman-group-exchange-sha256' {
		return t.kex_gex(neg, i_s)
	}
	p_hex := match neg.kex {
		'diffie-hellman-group14-sha256' { group14_hex }
		'diffie-hellman-group16-sha512' { group16_hex }
		'diffie-hellman-group18-sha512' { group18_hex }
		else { return error('unsupported key exchange ${neg.kex}') }
	}

	p := big.integer_from_radix(p_hex, 16)!
	return t.kex_dh(neg, i_s, p, big.two_int, []u8{})
}

fn (mut t Transport) kex_curve25519(neg Negotiated, i_s []u8) !Exchanged {
	mut priv := curve25519.PrivateKey.new()!
	defer {
		unsafe { priv.free() }
	}
	q_c := priv.public_key()!.bytes()!
	mut w := new_writer(40)
	w.put_u8(msg_kex_init)
	w.put_bytes(q_c)
	t.send_direct(w.buf)!
	pkt := t.read_kex_packet()!
	if pkt[0] != msg_kex_reply {
		return error('expected KEX reply, got message ${pkt[0]}')
	}
	mut r := new_reader(pkt[1..])
	k_s := r.get_bytes()!
	q_s := r.get_bytes()!
	sig := r.get_bytes()!
	mut sec := priv.x25519(q_s)!
	defer {
		wipe(mut sec)
	}
	k := mpint_bytes(sec)
	mut h := hash_prefix(client_version, t.v_s, t.kexinit_out, i_s)
	h.put_bytes(k_s)
	h.put_bytes(q_c)
	h.put_bytes(q_s)
	h.put_raw(k)
	return Exchanged{k_s, sig, kex_hash(neg.kex, h.buf), k}
}

fn (mut t Transport) kex_ecdh(neg Negotiated, i_s []u8) !Exchanged {
	curve := cng.curve_by_name(neg.kex.all_after_last('-'))!
	mut c := cng.instance()!
	local, q_c := c.ecdh_new(curve)!
	mut w := new_writer(150)
	w.put_u8(msg_kex_init)
	w.put_bytes(q_c)
	t.send_direct(w.buf)!
	pkt := t.read_kex_packet()!
	if pkt[0] != msg_kex_reply {
		return error('expected KEX reply, got message ${pkt[0]}')
	}
	mut r := new_reader(pkt[1..])
	k_s := r.get_bytes()!
	q_s := r.get_bytes()!
	sig := r.get_bytes()!
	mut sec := c.ecdh_finish(curve, local, q_s)!
	defer {
		wipe(mut sec)
	}
	k := mpint_bytes(sec)
	mut h := hash_prefix(client_version, t.v_s, t.kexinit_out, i_s)
	h.put_bytes(k_s)
	h.put_bytes(q_c)
	h.put_bytes(q_s)
	h.put_raw(k)
	return Exchanged{k_s, sig, kex_hash(neg.kex, h.buf), k}
}

// kex_dh runs finite-field diffie-hellman; gex_head is the group-exchange part of the hash.
fn (mut t Transport) kex_dh(neg Negotiated, i_s []u8, p big.Integer, g big.Integer, gex_head []u8) !Exchanged {
	exp_bits := if neg.kex.ends_with('sha256') { 512 } else { 1024 }
	mut x_bytes := random_bytes(exp_bits / 8)
	x_bytes[0] |= 0x80
	x := big.integer_from_bytes(x_bytes)
	e := g.big_mod_pow(x, p)!
	e_bytes, _ := e.bytes()
	mut w := new_writer(e_bytes.len + 8)
	if gex_head.len > 0 {
		w.put_u8(msg_gex_init)
	} else {
		w.put_u8(msg_kex_init)
	}
	w.put_mpint(e_bytes)
	t.send_direct(w.buf)!
	pkt := t.read_kex_packet()!
	want := if gex_head.len > 0 { msg_gex_reply } else { msg_kex_reply }
	if pkt[0] != want {
		return error('expected DH reply, got message ${pkt[0]}')
	}
	mut r := new_reader(pkt[1..])
	k_s := r.get_bytes()!
	f_bytes := r.get_mpint()!
	sig := r.get_bytes()!
	f := big.integer_from_bytes(f_bytes)
	if f <= big.one_int || f >= p - big.one_int {
		return error('invalid DH public value')
	}
	sec := f.big_mod_pow(x, p)!
	sec_bytes, _ := sec.bytes()
	k := mpint_bytes(sec_bytes)
	wipe(mut x_bytes)
	mut h := hash_prefix(client_version, t.v_s, t.kexinit_out, i_s)
	h.put_bytes(k_s)
	h.put_raw(gex_head)
	h.put_mpint(e_bytes)
	h.put_mpint(f_bytes)
	h.put_raw(k)
	return Exchanged{k_s, sig, kex_hash(neg.kex, h.buf), k}
}

fn (mut t Transport) kex_gex(neg Negotiated, i_s []u8) !Exchanged {
	min_bits, want_bits, max_bits := u32(2048), u32(3072), u32(8192)
	mut w := new_writer(16)
	w.put_u8(msg_gex_request)
	w.put_u32(min_bits)
	w.put_u32(want_bits)
	w.put_u32(max_bits)
	t.send_direct(w.buf)!
	pkt := t.read_kex_packet()!
	if pkt[0] != msg_kex_reply {
		return error('expected group from the server, got message ${pkt[0]}')
	}
	mut r := new_reader(pkt[1..])
	p_bytes := r.get_mpint()!
	g_bytes := r.get_mpint()!
	if p_bytes.len * 8 < 2048 || p_bytes.len * 8 > 8192 + 8 {
		return error('the server chose a bad DH group size')
	}
	mut head := new_writer(p_bytes.len + 32)
	head.put_u32(min_bits)
	head.put_u32(want_bits)
	head.put_u32(max_bits)
	head.put_mpint(p_bytes)
	head.put_mpint(g_bytes)
	p := big.integer_from_bytes(p_bytes)
	g := big.integer_from_bytes(g_bytes)
	return t.kex_dh(neg, i_s, p, g, head.buf)
}
