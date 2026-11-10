// Fuzz: packet framing and decryption (read_raw over every sealer). Hostile packets must
// fail with an error, never a panic (specs/11, fuzz tests).
module ssh

import quadra.testing.fuzz

struct FuzzPair {
mut:
	enc Sealer
	dec Sealer
}

const seal_kinds = ['plain', 'chacha', 'gcm', 'hmac-sha2-512-etm@openssh.com',
	'hmac-sha2-256-etm@openssh.com', 'hmac-sha2-512', 'hmac-sha2-256', 'hmac-sha1-etm@openssh.com',
	'hmac-sha1']

fn new_sealer(kind string) Sealer {
	key32 := []u8{len: 32, init: u8(index)}
	match kind {
		'plain' {
			return PlainSealer{}
		}
		'chacha' {
			return new_chacha_sealer([]u8{len: 64, init: u8(index * 3)}) or { panic(err) }
		}
		'gcm' {
			return new_gcm_sealer(key32, []u8{len: 12, init: u8(index + 7)}) or { panic(err) }
		}
		else {
			alg := mac_info(kind) or { panic(err) }
			return new_ctr_sealer(key32, []u8{len: 16, init: u8(index + 9)}, alg,
				[]u8{len: alg.key, init: 0x55}) or { panic(err) }
		}
	}
}

fn new_pair(kind string) FuzzPair {
	return FuzzPair{new_sealer(kind), new_sealer(kind)}
}

// crafted seals a packet whose length field says declared and whose padding byte is pad, with
// actual body bytes (rounded up to whole cipher blocks so every sealer accepts it).
fn crafted(mut enc Sealer, seq u32, declared u32, pad u8, actual int, mut rng fuzz.Rng) []u8 {
	mut n := 4 + actual
	n += (16 - n % 16) % 16
	mut pkt := rng.bytes(n)
	pkt[0], pkt[1], pkt[2], pkt[3] = u8(declared >> 24), u8(declared >> 16), u8(declared >> 8), u8(declared)
	pkt[4] = pad
	return enc.seal(seq, mut pkt) or { []u8{} }
}

fn valid(mut enc Sealer, seq u32, payload []u8) []u8 {
	mut pkt := build_packet(payload, enc)
	return enc.seal(seq, mut pkt) or { panic(err) }
}

// read_all reads packets until the stream fails, returning the payloads read.
fn read_all(kind string, mut dec Sealer, data []u8) [][]u8 {
	mut t := fuzz_transport(data)
	t.dec = dec
	mut out := [][]u8{}
	for _ in 0 .. 16 {
		pkt := t.read_raw() or { break }
		out << pkt.clone()
	}
	return out
}

fn test_reader_length_overflow_is_an_error() {
	// a string length near the int maximum: pos + n wrapped negative and passed the check
	for n in [u32(0x7ffffffd), 0x7fffffff, 0xfffffffb, 0x80000000] {
		mut r := new_reader([u8(n >> 24), u8(n >> 16), u8(n >> 8), u8(n), 1, 2, 3])
		if _ := r.get_bytes() {
			assert false, 'length ${n} accepted'
		}
	}
	mut r := new_reader([u8(0), 0, 0, 1, 9])
	r.get_u8() or { panic(err) }
	if _ := r.get_raw(0x7fffffff) {
		assert false
	}
}

fn test_ctr_short_packet_is_an_error() {
	// encrypt-and-MAC CTR decrypts the first block and sliced wire[16..] before the MAC check:
	// a packet length below 12 panicked
	for kind in ['hmac-sha2-256', 'hmac-sha1', 'hmac-sha2-512'] {
		for declared in [u32(1), 4, 8, 11, 12, 13] {
			mut p := new_pair(kind)
			mut rng := fuzz.new_rng(u64(declared))
			wire := crafted(mut p.enc, 0, declared, 4, 60, mut rng)
			assert read_all(kind, mut p.dec, wire).len == 0
		}
	}
}

fn test_valid_packets_round_trip() {
	for kind in seal_kinds {
		mut p := new_pair(kind)
		mut data := []u8{}
		for seq in 0 .. 3 {
			data << valid(mut p.enc, u32(seq), 'payload ${seq} of ${kind}'.bytes())
		}
		got := read_all(kind, mut p.dec, data)
		assert got.len == 3, kind
		assert got[2] == 'payload 2 of ${kind}'.bytes()
	}
}

fn test_fuzz_framing() {
	n := fuzz.iters(300)
	mut rng := fuzz.new_rng(0x5ea1)
	mut w := fuzz.watch('ssh framing', n * seal_kinds.len)
	lengths := [u32(0), 1, 2, 3, 4, 5, 7, 8, 11, 12, 15, 16, 17, 31, 32, 33, 512 * 1024,
		512 * 1024 + 1, 0x7fffffff, 0x80000000, 0xffffffff]
	for i in 0 .. n {
		for kind in seal_kinds {
			mut p := new_pair(kind)
			mut data := []u8{}
			mut seq := u32(0)
			for _ in 0 .. 1 + rng.intn(3) {
				if rng.chance(50) {
					data << valid(mut p.enc, seq, rng.bytes(rng.intn(300)))
				} else {
					declared := if rng.chance(70) {
						lengths[rng.intn(lengths.len)]
					} else {
						u32(rng.intn(2000))
					}
					pad := u8(if rng.chance(50) { rng.intn(8) } else { rng.intn(256) })
					data << crafted(mut p.enc, seq, declared, pad, rng.intn(600), mut rng)
				}
				seq++
			}
			if rng.chance(40) {
				data = rng.mutate(data, [])
			}
			w.start()
			read_all(kind, mut p.dec, data)
			w.done(i, data.len, 2000)
			fuzz_ceiling(i)
			p.enc.wipe_keys()
			p.dec.wipe_keys()
		}
	}
	w.finish(64)
}
