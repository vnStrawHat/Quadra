// RSA, ECDSA and ECDH keys through CNG (specs/23 D2). Blobs are built by hand; all integers are
// big-endian magnitudes. Coordinates of the NIST curves are fixed width.
module cng

const rsa_public_magic = u32(0x31415352)
const rsa_private_magic = u32(0x32415352)

struct Padding {
	alg &u16
}

fn le32(v u32) []u8 {
	return [u8(v), u8(v >> 8), u8(v >> 16), u8(v >> 24)]
}

fn hash_id(alg string) string {
	return match alg {
		'sha1' { 'SHA1' }
		'sha256' { 'SHA256' }
		'sha384' { 'SHA384' }
		else { 'SHA512' }
	}
}

fn pad_left(b []u8, n int) []u8 {
	if b.len >= n {
		return b[b.len - n..]
	}
	mut out := []u8{len: n - b.len, init: 0}
	out << b
	return out
}

fn rsa_bits(n []u8) int {
	if n.len == 0 {
		return 0
	}
	mut top := n[0]
	mut bits := (n.len - 1) * 8
	for top != 0 {
		bits++
		top >>= 1
	}
	return bits
}

fn (r RsaParts) blob(private bool) []u8 {
	mut b := []u8{}
	n := strip_zeros(r.n)
	e := strip_zeros(r.e)
	p := strip_zeros(r.p)
	q := strip_zeros(r.q)
	if private {
		b << le32(rsa_private_magic)
	} else {
		b << le32(rsa_public_magic)
	}
	b << le32(u32(rsa_bits(n)))
	b << le32(u32(e.len))
	b << le32(u32(n.len))
	if private {
		b << le32(u32(p.len))
		b << le32(u32(q.len))
	} else {
		b << le32(0)
		b << le32(0)
	}
	b << e
	b << n
	if private {
		b << p
		b << q
	}
	return b
}

fn (mut c Cng) import_blob(alg string, kind string, blob []u8) !voidptr {
	provider := c.provider(alg, '')!
	mut h := unsafe { nil }
	st := c.import_key(provider, unsafe { nil }, kind.to_wide(), &h, voidptr(blob.data),
		u32(blob.len), 0)
	if st != 0 {
		return error('${alg} key import failed: ${u32(st).hex()}')
	}
	return h
}

// rsa_verify checks a PKCS#1 v1.5 signature over the digest.
pub fn (mut c Cng) rsa_verify(key RsaParts, alg string, digest []u8, sig []u8) ! {
	h := c.import_blob('RSA', 'RSAPUBLICBLOB', key.blob(false))!
	defer {
		c.destroy(h)
	}
	pad := Padding{
		alg: hash_id(alg).to_wide()
	}
	st := c.verify(h, voidptr(&pad), digest.data, u32(digest.len), sig.data, u32(sig.len),
		0x00000002)
	if st != 0 {
		return error('RSA signature verification failed')
	}
}

// rsa_sign makes a PKCS#1 v1.5 signature over the digest.
pub fn (mut c Cng) rsa_sign(key RsaParts, alg string, digest []u8) ![]u8 {
	h := c.import_blob('RSA', 'RSAPRIVATEBLOB', key.blob(true))!
	defer {
		c.destroy(h)
	}
	pad := Padding{
		alg: hash_id(alg).to_wide()
	}
	mut sig := []u8{len: strip_zeros(key.n).len}
	mut got := u32(0)
	st := c.sign(h, voidptr(&pad), digest.data, u32(digest.len), sig.data, u32(sig.len), &got,
		0x00000002)
	if st != 0 {
		return error('RSA signing failed: ${u32(st).hex()}')
	}
	return sig[..int(got)]
}

// point_blob turns an uncompressed SEC1 point (04 || X || Y) into X and Y.
fn split_point(c Curve, point []u8) !([]u8, []u8) {
	if point.len != 1 + 2 * c.size || point[0] != 4 {
		return error('bad elliptic curve point')
	}
	return point[1..1 + c.size], point[1 + c.size..]
}

fn ecc_blob(magic u32, c Curve, x []u8, y []u8, d []u8) []u8 {
	mut b := []u8{}
	b << le32(magic)
	b << le32(u32(c.size))
	b << x
	b << y
	b << d
	return b
}

// ecdsa_verify checks a signature given as raw r and s magnitudes.
pub fn (mut cng_ Cng) ecdsa_verify(c Curve, point []u8, digest []u8, r []u8, s []u8) ! {
	x, y := split_point(c, point)!
	h := cng_.import_blob(c.ecdsa_alg, 'ECCPUBLICBLOB', ecc_blob(c.ecs1, c, x, y, []))!
	defer {
		cng_.destroy(h)
	}
	mut sig := pad_left(strip_zeros(r), c.size)
	sig << pad_left(strip_zeros(s), c.size)
	st := cng_.verify(h, unsafe { nil }, digest.data, u32(digest.len), sig.data, u32(sig.len), 0)
	if st != 0 {
		return error('ECDSA signature verification failed')
	}
}

// ecdsa_sign returns the signature as (r, s).
pub fn (mut cng_ Cng) ecdsa_sign(c Curve, point []u8, d []u8, digest []u8) !([]u8, []u8) {
	x, y := split_point(c, point)!
	h := cng_.import_blob(c.ecdsa_alg, 'ECCPRIVATEBLOB', ecc_blob(c.ecs2, c, x, y, pad_left(strip_zeros(d),
		c.size)))!
	defer {
		cng_.destroy(h)
	}
	mut sig := []u8{len: 2 * c.size}
	mut got := u32(0)
	st :=
		cng_.sign(h, unsafe { nil }, digest.data, u32(digest.len), sig.data, u32(sig.len), &got, 0)
	if st != 0 {
		return error('ECDSA signing failed: ${u32(st).hex()}')
	}
	return sig[..c.size], sig[c.size..]
}

// ecdh_new makes an ephemeral key and returns its handle and public point.
pub fn (mut cng_ Cng) ecdh_new(c Curve) !(voidptr, []u8) {
	alg := cng_.provider(c.ecdh_alg, '')!
	mut local := unsafe { nil }
	if cng_.gen_pair(alg, &local, c.bits, 0) != 0 || cng_.finalize(local, 0) != 0 {
		return error('ECDH key generation failed')
	}
	mut blob := []u8{len: 8 + 2 * c.size}
	mut got := u32(0)
	if cng_.export_key(local, unsafe { nil }, 'ECCPUBLICBLOB'.to_wide(), blob.data, u32(blob.len),
		&got, 0) != 0 {
		return error('ECDH key export failed')
	}
	mut point := [u8(4)]
	point << blob[8..8 + 2 * c.size]
	return local, point
}

// ecdh_finish returns the raw shared secret, big-endian, and destroys the ephemeral key.
pub fn (mut cng_ Cng) ecdh_finish(c Curve, local voidptr, peer []u8) ![]u8 {
	defer {
		cng_.destroy(local)
	}
	x, y := split_point(c, peer)!
	remote := cng_.import_blob(c.ecdh_alg, 'ECCPUBLICBLOB', ecc_blob(c.eck1, c, x, y, []))!
	defer {
		cng_.destroy(remote)
	}
	mut agreed := unsafe { nil }
	if cng_.agree(local, remote, &agreed, 0) != 0 {
		return error('ECDH agreement failed')
	}
	defer {
		cng_.destroy_s(agreed)
	}
	mut raw := []u8{len: c.size}
	mut got := u32(0)
	// TRUNCATE is the raw shared secret, little-endian (Windows 10 and later).
	if cng_.derive(agreed, 'TRUNCATE'.to_wide(), unsafe { nil }, raw.data, u32(raw.len), &got, 0) != 0 {
		return error('ECDH derive failed')
	}
	raw.reverse_in_place()
	return raw
}
