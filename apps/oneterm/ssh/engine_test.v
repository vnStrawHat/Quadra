// Table tests with published vectors, and authentication against a scripted server.
module ssh

import crypto.hmac
import crypto.sha256
import encoding.base64
import encoding.hex
import io
import os
import quadra.native.cng

fn test_mpint_rfc4251_examples() {
	for value, expected in {
		'':                 '00000000'
		'09a378f9b2e332a7': '0000000809a378f9b2e332a7'
		'80':               '000000020080'
		'0080':             '000000020080'
		'0000':             '00000000'
	} {
		mut w := new_writer(16)
		w.put_mpint(hex.decode(value) or { panic(err) })
		assert w.buf.hex() == expected, value
	}
}

fn test_reader_rejects_truncated_input() {
	mut r := new_reader([u8(0), 0, 0, 5, 1])
	if _ := r.get_bytes() {
		assert false
	}
}

fn test_hmac_sha256_rfc4231_case1() {
	key := []u8{len: 20, init: 0x0b}
	tag := hmac.new(key, 'Hi There'.bytes(), sha256.sum, sha256.block_size)
	assert tag.hex() == 'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7'
}

fn test_aes_ctr_nist_sp800_38a() {
	mut c := cng.instance() or { panic(err) }
	key := hex.decode('2b7e151628aed2a6abf7158809cf4f3c') or { panic(err) }
	mut iv := hex.decode('f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff') or { panic(err) }
	mut block := hex.decode('6bc1bee22e409f96e93d7e117393172a') or { panic(err) }
	h := c.aes_key('ChainingModeECB', key) or { panic(err) }
	c.aes_ctr_xor(h, mut iv, mut block) or { panic(err) }
	assert block.hex() == '874d6191b620e3261bef6864990db6ce'
	c.destroy(h)
}

fn round_trip(mut enc Sealer, mut dec Sealer) ! {
	payload := 'a payload that spans at least one block of the cipher, surely.'.bytes()
	for seq in [u32(0), 1, 0xffffffff] {
		mut pkt := build_packet(payload, enc)
		wire := enc.seal(seq, mut pkt)!
		first := wire[..dec.first_len()]
		plen := dec.read_len(seq, first)!
		mut full := wire.clone()
		assert 4 + plen + dec.mac_len() == full.len
		body := dec.open(seq, mut full)!
		pad := int(body[0])
		assert body[1..body.len - pad] == payload
	}
	// tampering with the ciphertext must fail authentication
	mut pkt := build_packet(payload, enc)
	mut wire := enc.seal(5, mut pkt)!
	wire[wire.len - 1] ^= 1
	first := wire[..dec.first_len()].clone()
	dec.read_len(5, first)!
	if _ := dec.open(5, mut wire) {
		return error('a corrupted packet was accepted')
	}
}

fn test_every_cipher_round_trips_and_rejects_tampering() {
	key32 := []u8{len: 32, init: u8(index)}
	key64 := []u8{len: 64, init: u8(index)}
	iv12 := []u8{len: 12, init: u8(index + 7)}
	iv16 := []u8{len: 16, init: u8(index + 9)}
	mac32 := []u8{len: 32, init: 0x55}
	mac64 := []u8{len: 64, init: 0xaa}
	mut pairs := []SealerPair{}
	pairs << SealerPair{new_chacha_sealer(key64) or { panic(err) }, new_chacha_sealer(key64) or {
		panic(err)
	}}
	pairs << SealerPair{new_gcm_sealer(key32, iv12) or { panic(err) }, new_gcm_sealer(key32, iv12) or {
		panic(err)
	}}
	for name in mac_algorithms {
		alg := mac_info(name) or { panic(err) }
		mac_key := if alg.key == 64 { mac64 } else { mac32[..alg.key] }
		pairs << SealerPair{new_ctr_sealer(key32, iv16, alg, mac_key) or { panic(err) }, new_ctr_sealer(key32,
			iv16, alg, mac_key) or { panic(err) }}
	}
	for mut p in pairs {
		round_trip(mut p.enc, mut p.dec) or { panic(err) }
		p.enc.wipe_keys()
		p.dec.wipe_keys()
	}
}

struct SealerPair {
mut:
	enc Sealer
	dec Sealer
}

// ScriptStream hands out plain-framed packets and records what was written.
@[heap]
struct ScriptStream {
mut:
	incoming []u8
	pos      int
	sent     [][]u8
}

fn (mut s ScriptStream) read(mut buf []u8) !int {
	if s.pos >= s.incoming.len {
		return io.Eof{}
	}
	n := copy(mut buf, s.incoming[s.pos..])
	s.pos += n
	return n
}

fn (mut s ScriptStream) write(data []u8) ! {
	s.sent << data.clone()
}

fn (mut s ScriptStream) close() {}

fn script_transport(replies [][]u8) (&Transport, &ScriptStream) {
	mut s := &ScriptStream{}
	for r in replies {
		s.incoming << build_packet(r, PlainSealer{})
	}
	t := new_transport(Stream(s), Offer{}, HostCheck{})
	return t, s
}

fn failure_packet(methods string) []u8 {
	mut w := new_writer(32)
	w.put_u8(msg_userauth_failure)
	w.put_str(methods)
	w.put_bool(false)
	return w.buf
}

fn info_request(echoes []bool) []u8 {
	mut w := new_writer(64)
	w.put_u8(msg_userauth_info_request)
	w.put_str('')
	w.put_str('')
	w.put_str('')
	w.put_u32(u32(echoes.len))
	for e in echoes {
		w.put_str('Password: ')
		w.put_bool(e)
	}
	return w.buf
}

fn test_password_falls_back_to_keyboard_interactive() {
	mut t, _ := script_transport([failure_packet('keyboard-interactive'),
		info_request([false]), [msg_userauth_success]])
	o := auth_password(mut t, 'user', 'secret'.bytes()) or { panic(err) }
	assert o.success
}

fn test_keyboard_refuses_echo_prompt_and_second_round() {
	mut t, _ := script_transport([failure_packet('keyboard-interactive'),
		info_request([true])])
	if _ := auth_password(mut t, 'user', 'secret'.bytes()) {
		assert false
	} else {
		assert err.msg().contains('other than a password')
	}
	mut t2, _ := script_transport([failure_packet('keyboard-interactive'),
		info_request([false]), info_request([false])])
	if _ := auth_password(mut t2, 'user', 'secret'.bytes()) {
		assert false
	} else {
		assert err.msg().contains('second round')
	}
}

fn test_failure_message_names_remaining_methods() {
	assert failure_message(AuthOutcome{}) == 'rejected by the server'
	assert failure_message(AuthOutcome{
		remaining: ['publickey', 'keyboard-interactive']
	}) == 'rejected by the server; the server accepts: publickey, keyboard-interactive'
}

fn test_rsa_hash_prefers_server_choice_and_never_sha1() {
	mut t, _ := script_transport([])
	assert pubkey_alg(mut t, 'ssh-rsa') == 'rsa-sha2-512'
	t.has_sig = true
	t.sig_algs = ['rsa-sha2-256']
	assert pubkey_alg(mut t, 'ssh-rsa') == 'rsa-sha2-256'
	t.sig_algs = ['ssh-rsa']
	assert pubkey_alg(mut t, 'ssh-rsa') == 'rsa-sha2-512'
	assert pubkey_alg(mut t, 'ssh-ed25519') == 'ssh-ed25519'
}

const kh_dir = os.join_path(@DIR, '..', '..', '..', '.tmp', 'ssh-unit')

const fixture_blob_b64 = 'AAAAC3NzaC1lZDI1NTE5AAAAIPJc8OF/864CmV0I5RpMGvhRN63HNlmahjFptECCh1W4'
const hashed_line = '|1|95gNYggxa9oatHgUeraaa0nz+kU=|WM2yVWqTLohfVRMkRga7/fwAMFs= ssh-ed25519 ${fixture_blob_b64}'

fn kh_key() HostKey {
	return parse_host_key(base64.decode(fixture_blob_b64)) or { panic(err) }
}

fn test_known_hosts_verdict_table() {
	os.mkdir_all(kh_dir) or { panic(err) }
	path := os.join_path(kh_dir, 'known_hosts')
	key := kh_key()
	// no file, then no entry for the host: unknown, fail closed
	os.rm(path) or {}
	if _ := check_host_key(path, '10.0.0.1', 22, HostKeyPolicy{}, key) {
		assert false
	} else {
		assert err is HostKeyUnknown
	}
	// exact match through a hashed [host]:port entry
	os.write_file(path, '${hashed_line}\n') or { panic(err) }
	check_host_key(path, '127.0.0.1', 42000, HostKeyPolicy{}, key) or { panic(err) }
	// an entry with the same algorithm and another key is refused
	other := 'AAAAC3NzaC1lZDI1NTE5AAAAIKoVi2rlYwaW7mP7yYgA+LgDGNJk1IuUqGwdxn1leIyN'
	os.write_file(path, 'somehost ssh-ed25519 ${other}\n') or { panic(err) }
	if _ := check_host_key(path, 'somehost', 22, HostKeyPolicy{}, key) {
		assert false
	} else {
		assert err is HostKeyChanged
	}
	// a wrong fingerprint is unknown; the right one is learned and then matches
	os.rm(path) or {}
	if _ := check_host_key(path, 'h', 22, accept_new('SHA256:wrong'), key) {
		assert false
	} else {
		assert err is HostKeyUnknown
	}
	check_host_key(path, 'h', 22, accept_new(key.fingerprint()), key) or { panic(err) }
	check_host_key(path, 'h', 22, HostKeyPolicy{}, key) or { panic(err) }
}

fn test_host_name_patterns() {
	assert patterns_match('*.example.com,!bad.example.com', 'good.example.com')
	assert !patterns_match('*.example.com,!bad.example.com', 'bad.example.com')
	assert patterns_match('[host]:2222', '[host]:2222')
	assert glob_match('h?st*', 'host-a')
	assert host_port('h', 22) == 'h'
	assert host_port('h', 2222) == '[h]:2222'
}

fn test_preferred_algorithms_put_known_first() {
	rsa := KnownKey{'ssh-rsa', [], false}
	first := preferred_algorithms([rsa])
	assert first[0] == 'rsa-sha2-512' && first[2] == 'ssh-rsa'
	assert first[3] == 'ssh-ed25519'
}

fn test_remote_paths_and_names() {
	assert remote_path('a//b/') == 'a/b'
	assert remote_path(r'x\y') == 'x/y'
	assert remote_join('/a', 'b') == '/a/b'
	assert remote_join('/a', '/b') == '/b'
	assert remote_parent('/a/b') == '/a'
	assert remote_parent('/') == ''
	assert remote_file_name('/a/b.txt') == 'b.txt'
	for bad in ['', '.', '..', 'a/b', r'a\b', 'c:d', 'tail.', 'tail ', 'NUL', 'com1.txt'] {
		if _ := validate_remote_entry_name(bad) {
			assert false, bad
		}
	}
	validate_remote_entry_name('ok.txt') or { panic(err) }
}

fn test_passwd_and_group_parsers() {
	mut uids := map[u32]string{}
	mut gids := map[u32]string{}
	parse_passwd('root:x:0:0:root:/root:/bin/sh\nbroken\nalice:x:1000:1000:::\n', mut uids, mut
		gids)
	parse_group('wheel:x:0:\n', mut gids)
	assert uids[0] == 'root' && uids[1000] == 'alice'
	assert gids[0] == 'wheel' && gids[1000] == 'alice'
}
