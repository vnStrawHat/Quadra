// Packet protection (RFC 4253 section 6, PROTOCOL.chacha20poly1305). Each Sealer owns the keys
// of one direction; the transport serialises its calls.
module ssh

import crypto.rand
import encoding.binary
import x.crypto.chacha20
import x.crypto.poly1305

interface Sealer {
	first_len() int // bytes needed before the packet length is known
	mac_len() int
	pad_unit() int
	length_excluded() bool // padding ignores the 4-byte length field
mut:
	seal(seq u32, mut pkt []u8) ![]u8
	read_len(seq u32, first []u8) !int
	open(seq u32, mut wire []u8) ![]u8 // returns padding_length || payload || padding
	wipe_keys()
}

fn random_bytes(n int) []u8 {
	return rand.bytes(n) or { []u8{len: n} }
}

// build_packet makes packet_length || padding_length || payload || padding.
fn build_packet(payload []u8, s Sealer) []u8 {
	unit := if s.pad_unit() < 8 { 8 } else { s.pad_unit() }
	header := if s.length_excluded() { 1 } else { 5 }
	mut pad := unit - (header + payload.len) % unit
	if pad < 4 {
		pad += unit
	}
	mut pkt := []u8{len: 5 + payload.len + pad}
	binary.big_endian_put_u32(mut pkt, u32(1 + payload.len + pad))
	pkt[4] = u8(pad)
	copy(mut pkt[5..], payload)
	padding := random_bytes(pad)
	copy(mut pkt[5 + payload.len..], padding)
	return pkt
}

struct PlainSealer {}

fn (s PlainSealer) first_len() int {
	return 4
}

fn (s PlainSealer) mac_len() int {
	return 0
}

fn (s PlainSealer) pad_unit() int {
	return 8
}

fn (s PlainSealer) length_excluded() bool {
	return false
}

fn (mut s PlainSealer) seal(seq u32, mut pkt []u8) ![]u8 {
	return pkt
}

fn (mut s PlainSealer) read_len(seq u32, first []u8) !int {
	return int(binary.big_endian_u32(first))
}

fn (mut s PlainSealer) open(seq u32, mut wire []u8) ![]u8 {
	return wire[4..]
}

fn (mut s PlainSealer) wipe_keys() {}

// ChachaSealer is chacha20-poly1305@openssh.com: a 64-byte key, the first half for the payload
// and the poly key, the second half for the length; the nonce is the sequence number.
struct ChachaSealer {
mut:
	key_main []u8
	key_head []u8
}

fn new_chacha_sealer(key []u8) !&ChachaSealer {
	if key.len != 64 {
		return error('chacha20-poly1305 needs a 64-byte key')
	}
	return &ChachaSealer{
		key_main: key[..32].clone()
		key_head: key[32..].clone()
	}
}

fn (s ChachaSealer) first_len() int {
	return 4
}

fn (s ChachaSealer) mac_len() int {
	return 16
}

fn (s ChachaSealer) pad_unit() int {
	return 8
}

fn (s ChachaSealer) length_excluded() bool {
	return true
}

fn seq_nonce(seq u32) []u8 {
	mut nonce := []u8{len: 8}
	binary.big_endian_put_u32(mut nonce[4..], seq)
	return nonce
}

fn xor_at(key []u8, nonce []u8, counter u64, mut data []u8) ! {
	mut c := chacha20.new_cipher(key, nonce)!
	c.set_counter(counter)
	out := data.clone()
	c.xor_key_stream(mut data, out)
}

fn poly_key(key []u8, nonce []u8) ![]u8 {
	mut block := []u8{len: 32}
	xor_at(key, nonce, 0, mut block)!
	return block
}

fn (mut s ChachaSealer) seal(seq u32, mut pkt []u8) ![]u8 {
	nonce := seq_nonce(seq)
	mut body := unsafe { pkt[4..] }
	xor_at(s.key_main, nonce, 1, mut body)!
	mut head := unsafe { pkt[..4] }
	xor_at(s.key_head, nonce, 0, mut head)!
	mut pk := poly_key(s.key_main, nonce)!
	defer {
		wipe(mut pk)
	}
	mut tag := []u8{len: 16}
	poly1305.create_tag(mut tag, pkt, pk)!
	mut out := []u8{cap: pkt.len + 16}
	out << pkt
	out << tag
	return out
}

fn (mut s ChachaSealer) read_len(seq u32, first []u8) !int {
	mut head := first.clone()
	xor_at(s.key_head, seq_nonce(seq), 0, mut head)!
	return int(binary.big_endian_u32(head))
}

fn (mut s ChachaSealer) open(seq u32, mut wire []u8) ![]u8 {
	nonce := seq_nonce(seq)
	mut pk := poly_key(s.key_main, nonce)!
	defer {
		wipe(mut pk)
	}
	body_end := wire.len - 16
	tag := wire[body_end..]
	if !poly1305.verify_tag(tag, wire[..body_end], pk) {
		return error('bad packet authentication tag')
	}
	mut body := unsafe { wire[4..body_end] }
	xor_at(s.key_main, nonce, 1, mut body)!
	return body
}

fn (mut s ChachaSealer) wipe_keys() {
	wipe(mut s.key_main)
	wipe(mut s.key_head)
}
