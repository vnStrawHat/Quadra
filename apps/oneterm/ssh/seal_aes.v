// AES-GCM (RFC 5647) and AES-CTR with HMAC (RFC 4253, and the OpenSSH etm variants).
module ssh

import crypto.hmac
import crypto.sha1
import crypto.sha256
import crypto.sha512
import encoding.binary
import quadra.native.cng

// MacAlg is a negotiated HMAC.
struct MacAlg {
	name string
	size int
	etm  bool
	key  int
}

fn mac_info(name string) !MacAlg {
	return match name {
		'hmac-sha2-512-etm@openssh.com' { MacAlg{name, 64, true, 64} }
		'hmac-sha2-256-etm@openssh.com' { MacAlg{name, 32, true, 32} }
		'hmac-sha2-512' { MacAlg{name, 64, false, 64} }
		'hmac-sha2-256' { MacAlg{name, 32, false, 32} }
		'hmac-sha1-etm@openssh.com' { MacAlg{name, 20, true, 20} }
		'hmac-sha1' { MacAlg{name, 20, false, 20} }
		else { error('unknown MAC ${name}') }
	}
}

fn hmac_with(alg MacAlg, key []u8, data []u8) []u8 {
	return match alg.size {
		64 { hmac.new(key, data, sha512.sum512, sha512.block_size) }
		32 { hmac.new(key, data, sha256.sum, sha256.block_size) }
		else { hmac.new(key, data, sha1.sum, sha1.block_size) }
	}
}

// GcmSealer: the length is authenticated but not encrypted; the nonce is a 4-byte fixed part
// and an 8-byte invocation counter incremented per packet.
struct GcmSealer {
mut:
	key   voidptr
	nonce []u8
	cng   &cng.Cng = unsafe { nil }
}

fn new_gcm_sealer(key []u8, iv []u8) !&GcmSealer {
	mut c := cng.instance()!
	return &GcmSealer{
		key:   c.aes_key('ChainingModeGCM', key)!
		nonce: iv.clone()
		cng:   c
	}
}

fn (s GcmSealer) first_len() int {
	return 4
}

fn (s GcmSealer) mac_len() int {
	return 16
}

fn (s GcmSealer) pad_unit() int {
	return 16
}

fn (s GcmSealer) length_excluded() bool {
	return true
}

fn (mut s GcmSealer) next_nonce() {
	for i := 11; i >= 4; i-- {
		s.nonce[i]++
		if s.nonce[i] != 0 {
			break
		}
	}
}

fn (mut s GcmSealer) seal(seq u32, mut pkt []u8) ![]u8 {
	mut body := unsafe { pkt[4..] }
	mut tag := []u8{len: 16}
	s.cng.aes_gcm_seal(s.key, s.nonce, pkt[..4], mut body, mut tag)!
	s.next_nonce()
	mut out := []u8{cap: pkt.len + 16}
	out << pkt
	out << tag
	return out
}

fn (mut s GcmSealer) read_len(seq u32, first []u8) !int {
	return int(binary.big_endian_u32(first))
}

fn (mut s GcmSealer) open(seq u32, mut wire []u8) ![]u8 {
	end := wire.len - 16
	mut body := unsafe { wire[4..end] }
	s.cng.aes_gcm_open(s.key, s.nonce, wire[..4], mut body, wire[end..])!
	s.next_nonce()
	return body
}

fn (mut s GcmSealer) wipe_keys() {
	s.cng.destroy(s.key)
}

// CtrSealer is AES-CTR with an HMAC, encrypt-and-MAC or encrypt-then-MAC.
struct CtrSealer {
mut:
	key     voidptr
	iv      []u8
	mac     MacAlg
	mac_key []u8
	cng     &cng.Cng = unsafe { nil }
	first   []u8 // decrypted first block of the packet being read
}

fn new_ctr_sealer(key []u8, iv []u8, mac MacAlg, mac_key []u8) !&CtrSealer {
	mut c := cng.instance()!
	return &CtrSealer{
		key:     c.aes_key('ChainingModeECB', key)!
		iv:      iv.clone()
		mac:     mac
		mac_key: mac_key.clone()
		cng:     c
	}
}

fn (s CtrSealer) first_len() int {
	return if s.mac.etm { 4 } else { 16 }
}

fn (s CtrSealer) mac_len() int {
	return s.mac.size
}

fn (s CtrSealer) pad_unit() int {
	return 16
}

fn (s CtrSealer) length_excluded() bool {
	return s.mac.etm
}

fn with_seq(seq u32, data []u8) []u8 {
	mut b := []u8{cap: data.len + 4}
	b << u8(seq >> 24)
	b << u8(seq >> 16)
	b << u8(seq >> 8)
	b << u8(seq)
	b << data
	return b
}

fn (mut s CtrSealer) seal(seq u32, mut pkt []u8) ![]u8 {
	mut tag := []u8{}
	if s.mac.etm {
		mut body := unsafe { pkt[4..] }
		s.cng.aes_ctr_xor(s.key, mut s.iv, mut body)!
		tag = hmac_with(s.mac, s.mac_key, with_seq(seq, pkt))
	} else {
		tag = hmac_with(s.mac, s.mac_key, with_seq(seq, pkt))
		s.cng.aes_ctr_xor(s.key, mut s.iv, mut pkt)!
	}
	mut out := []u8{cap: pkt.len + tag.len}
	out << pkt
	out << tag
	return out
}

fn (mut s CtrSealer) read_len(seq u32, first []u8) !int {
	if s.mac.etm {
		return int(binary.big_endian_u32(first))
	}
	s.first = first.clone()
	s.cng.aes_ctr_xor(s.key, mut s.iv, mut s.first)!
	return int(binary.big_endian_u32(s.first))
}

fn (mut s CtrSealer) open(seq u32, mut wire []u8) ![]u8 {
	end := wire.len - s.mac.size
	if s.mac.etm {
		if !hmac.equal(hmac_with(s.mac, s.mac_key, with_seq(seq, wire[..end])), wire[end..]) {
			return error('bad packet authentication tag')
		}
		mut body := unsafe { wire[4..end] }
		s.cng.aes_ctr_xor(s.key, mut s.iv, mut body)!
		return body
	}
	mut rest := unsafe { wire[16..end] }
	s.cng.aes_ctr_xor(s.key, mut s.iv, mut rest)!
	copy(mut wire[..16], s.first)
	if !hmac.equal(hmac_with(s.mac, s.mac_key, with_seq(seq, wire[..end])), wire[end..]) {
		return error('bad packet authentication tag')
	}
	return wire[4..end]
}

fn (mut s CtrSealer) wipe_keys() {
	s.cng.destroy(s.key)
	wipe(mut s.mac_key)
}
