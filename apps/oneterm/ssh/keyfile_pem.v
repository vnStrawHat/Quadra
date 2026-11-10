// The non-OpenSSH private key formats: PKCS#1, SEC1, PKCS#8, legacy DEK-Info and PBES2.
module ssh

import crypto.ed25519
import crypto.md5
import crypto.pbkdf2
import crypto.sha1
import crypto.sha256
import encoding.hex
import quadra.native.cng

const oid_p256 = [u8(0x2a), 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]
const oid_p384 = [u8(0x2b), 0x81, 0x04, 0x00, 0x22]
const oid_p521 = [u8(0x2b), 0x81, 0x04, 0x00, 0x23]
const oid_rsa = [u8(0x2a), 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01]
const oid_ec = [u8(0x2a), 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]
const oid_ed25519 = [u8(0x2b), 0x65, 0x70]
const oid_pbes2 = [u8(0x2a), 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x05, 0x0d]
const oid_pbkdf2 = [u8(0x2a), 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x05, 0x0c]
const oid_hmac_sha256 = [u8(0x2a), 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x02, 0x09]
const oid_aes128_cbc = [u8(0x60), 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x02]
const oid_aes192_cbc = [u8(0x60), 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x16]
const oid_aes256_cbc = [u8(0x60), 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x01, 0x2a]

fn curve_from_oid(oid []u8) !cng.Curve {
	if oid == oid_p256 {
		return cng.curve_by_name('nistp256')
	}
	if oid == oid_p384 {
		return cng.curve_by_name('nistp384')
	}
	if oid == oid_p521 {
		return cng.curve_by_name('nistp521')
	}
	return error('unsupported elliptic curve')
}

fn decode_pkcs1(der []u8) !PrivateKey {
	mut top := Der{
		buf: der
	}
	mut s := top.seq()!
	s.int_val()!
	n := s.int_bytes()!
	e := s.int_bytes()!
	s.int_bytes()! // d
	p := s.int_bytes()!
	q := s.int_bytes()!
	rsa := cng.RsaParts{
		n: n.clone()
		e: e.clone()
		p: p.clone()
		q: q.clone()
	}
	return PrivateKey{
		kind:        'ssh-rsa'
		public_blob: rsa_public_blob(rsa)
		rsa:         rsa
	}
}

fn decode_sec1(der []u8, known ?cng.Curve) !PrivateKey {
	mut top := Der{
		buf: der
	}
	mut s := top.seq()!
	s.int_val()!
	d := s.octets()!
	mut curve := cng.Curve{}
	mut have_curve := false
	if c := known {
		curve = c
		have_curve = true
	}
	mut point := []u8{}
	for s.has_more() {
		tag, body := s.next()!
		if tag == 0xa0 {
			mut inner := Der{
				buf: body
			}
			curve = curve_from_oid(inner.oid()!)!
			have_curve = true
		} else if tag == 0xa1 {
			mut inner := Der{
				buf: body
			}
			bits := inner.expect(0x03)!
			point = bits[1..].clone()
		}
	}
	if !have_curve || point.len == 0 {
		return error('EC key without curve or public point')
	}
	return PrivateKey{
		kind:        'ecdsa-sha2-${curve.ssh_id}'
		public_blob: ec_public_blob(curve, point)
		curve:       curve
		point:       point
		d:           d.clone()
	}
}

fn decode_pkcs8(der []u8) !PrivateKey {
	mut top := Der{
		buf: der
	}
	mut s := top.seq()!
	s.int_val()!
	mut alg := s.seq()!
	oid := alg.oid()!
	key := s.octets()!
	if oid == oid_rsa {
		return decode_pkcs1(key)
	}
	if oid == oid_ec {
		return decode_sec1(key, curve_from_oid(alg.oid()!)!)
	}
	if oid == oid_ed25519 {
		mut inner := Der{
			buf: key
		}
		seed := inner.octets()!
		if seed.len != 32 {
			return error('bad ed25519 key')
		}
		priv := ed25519.new_key_from_seed(seed)
		return PrivateKey{
			kind:        'ssh-ed25519'
			public_blob: ed_public_blob(priv.public_key())
			seed:        seed.clone()
		}
	}
	return error('unsupported PKCS#8 key algorithm')
}

fn strip_pkcs7(data []u8) ![]u8 {
	if data.len == 0 || data.len % 16 != 0 {
		return error('wrong passphrase')
	}
	pad := int(data.last())
	if pad < 1 || pad > 16 {
		return error('wrong passphrase')
	}
	for b in data[data.len - pad..] {
		if int(b) != pad {
			return error('wrong passphrase')
		}
	}
	return data[..data.len - pad]
}

// legacy_decrypt handles the DEK-Info encryption of PKCS#1 and SEC1 files.
fn legacy_decrypt(pem Pem, pass []u8) ![]u8 {
	info := pem.headers['DEK-Info'] or { return pem.body }
	if pass.len == 0 {
		return error('the key needs a passphrase')
	}
	keylen := match info.all_before(',') {
		'AES-128-CBC' { 16 }
		'AES-256-CBC' { 32 }
		else { return error('unsupported key encryption ${info.all_before(',')}') }
	}

	iv := hex.decode(info.all_after(','))!
	if iv.len != 16 {
		return error('bad DEK-Info')
	}
	mut km := []u8{}
	mut last := []u8{}
	for km.len < keylen {
		mut block := last.clone()
		block << pass
		block << iv[..8]
		last = md5.sum(block)
		km << last
	}
	mut data := pem.body.clone()
	mut c := cng.instance()!
	c.aes_cbc_decrypt(km[..keylen], iv, mut data)!
	return strip_pkcs7(data)!.clone()
}

// pbes2_decrypt unwraps an ENCRYPTED PRIVATE KEY (PBES2, PBKDF2, AES-CBC).
fn pbes2_decrypt(der []u8, pass []u8) ![]u8 {
	if pass.len == 0 {
		return error('the key needs a passphrase')
	}
	mut top := Der{
		buf: der
	}
	mut s := top.seq()!
	mut alg := s.seq()!
	if alg.oid()! != oid_pbes2 {
		return error('unsupported PKCS#8 encryption')
	}
	body := s.octets()!
	mut params := alg.seq()!
	mut kdf := params.seq()!
	if kdf.oid()! != oid_pbkdf2 {
		return error('unsupported PKCS#8 key derivation')
	}
	mut kp := kdf.seq()!
	salt := kp.octets()!
	iterations := kp.int_val()!
	mut sha256_prf := false
	for kp.has_more() {
		tag, b := kp.next()!
		if tag == 0x30 {
			mut p2 := Der{
				buf: b
			}
			if p2.oid()! == oid_hmac_sha256 {
				sha256_prf = true
			}
		}
	}
	mut enc := params.seq()!
	enc_oid := enc.oid()!
	iv := enc.octets()!
	keylen := if enc_oid == oid_aes128_cbc {
		16
	} else if enc_oid == oid_aes192_cbc {
		24
	} else if enc_oid == oid_aes256_cbc {
		32
	} else {
		0
	}
	if keylen == 0 {
		return error('unsupported PKCS#8 cipher')
	}
	key := if sha256_prf {
		pbkdf2.key(pass, salt, iterations, keylen, sha256.new())!
	} else {
		pbkdf2.key(pass, salt, iterations, keylen, sha1.new())!
	}
	mut data := body.clone()
	mut c := cng.instance()!
	c.aes_cbc_decrypt(key, iv, mut data)!
	return strip_pkcs7(data)!.clone()
}
