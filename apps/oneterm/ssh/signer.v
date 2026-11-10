// A private key that can sign (ed25519 with vlib, RSA and ECDSA with CNG).
module ssh

import crypto.ed25519
import crypto.sha256
import crypto.sha512
import quadra.native.cng

// PrivateKey holds decrypted key material; wipe it after authentication.
pub struct PrivateKey {
pub:
	kind string // ssh-ed25519, ssh-rsa or ecdsa-sha2-nistp*
mut:
	public_blob []u8
	seed        []u8 // ed25519
	rsa         cng.RsaParts
	curve       cng.Curve
	point       []u8
	d           []u8 // ecdsa
}

pub fn (mut k PrivateKey) wipe() {
	wipe(mut k.seed)
	wipe(mut k.rsa.p)
	wipe(mut k.rsa.q)
	wipe(mut k.d)
}

// sign_alg returns the signature algorithm and wire signature blob over data. RSA keys sign
// with the given hash algorithm name (rsa-sha2-256 or rsa-sha2-512), never SHA-1.
fn (k PrivateKey) sign(alg string, data []u8) ![]u8 {
	mut sig := new_writer(128)
	sig.put_str(alg)
	match k.kind {
		'ssh-ed25519' {
			priv := ed25519.new_key_from_seed(k.seed)
			sig.put_bytes(ed25519.sign(priv, data)!)
		}
		'ssh-rsa' {
			mut c := cng.instance()!
			if alg == 'rsa-sha2-256' {
				sig.put_bytes(c.rsa_sign(k.rsa, 'sha256', sha256.sum(data))!)
			} else if alg == 'rsa-sha2-512' {
				sig.put_bytes(c.rsa_sign(k.rsa, 'sha512', sha512.sum512(data))!)
			} else {
				return error('RSA keys never sign with ${alg}')
			}
		}
		else {
			mut c := cng.instance()!
			r, s := c.ecdsa_sign(k.curve, k.point, k.d, ecdsa_digest(k.curve, data))!
			mut pair := new_writer(160)
			pair.put_mpint(r)
			pair.put_mpint(s)
			sig.put_bytes(pair.buf)
		}
	}

	return sig.buf
}

fn ec_public_blob(c cng.Curve, point []u8) []u8 {
	mut w := new_writer(point.len + 32)
	w.put_str('ecdsa-sha2-${c.ssh_id}')
	w.put_str(c.ssh_id)
	w.put_bytes(point)
	return w.buf
}

fn ed_public_blob(public []u8) []u8 {
	mut w := new_writer(64)
	w.put_str('ssh-ed25519')
	w.put_bytes(public)
	return w.buf
}

fn rsa_public_blob(r cng.RsaParts) []u8 {
	mut w := new_writer(r.n.len + 32)
	w.put_str('ssh-rsa')
	w.put_mpint(r.e)
	w.put_mpint(r.n)
	return w.buf
}
