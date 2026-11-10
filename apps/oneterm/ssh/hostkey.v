// Public keys and signatures (RFC 4253 section 6.6, RFC 5656, RFC 8709, RFC 8332).
module ssh

import crypto.ed25519
import crypto.sha1
import crypto.sha256
import crypto.sha512
import encoding.base64
import quadra.native.cng

// HostKey is a parsed public key blob: a server's host key, or an identity's public half.
pub struct HostKey {
pub:
	kind string // the key type: ssh-ed25519, ecdsa-sha2-nistp256, ssh-rsa, ...
	blob []u8
mut:
	rsa   cng.RsaParts
	point []u8 // ed25519: 32 bytes; ecdsa: SEC1 point
	curve cng.Curve
}

fn parse_host_key(blob []u8) !HostKey {
	mut r := new_reader(blob)
	kind := r.get_str()!
	match kind {
		'ssh-ed25519' {
			point := r.get_bytes()!
			if point.len != 32 {
				return error('bad ed25519 public key')
			}
			return HostKey{
				kind:  kind
				blob:  blob.clone()
				point: point.clone()
			}
		}
		'ssh-rsa' {
			e := r.get_mpint()!
			n := r.get_mpint()!
			return HostKey{
				kind: kind
				blob: blob.clone()
				rsa:  cng.RsaParts{
					n: n.clone()
					e: e.clone()
				}
			}
		}
		else {
			if kind.starts_with('ecdsa-sha2-') {
				id := r.get_str()!
				point := r.get_bytes()!
				return HostKey{
					kind:  kind
					blob:  blob.clone()
					point: point.clone()
					curve: cng.curve_by_name(id)!
				}
			}
			if kind.contains('-cert-') {
				return error('host certificates are not supported')
			}
			return error('unsupported key type ${kind}')
		}
	}
}

// fingerprint is the OpenSSH form: SHA256: and unpadded base64 of the blob.
pub fn fingerprint(blob []u8) string {
	return 'SHA256:' + base64.encode(sha256.sum(blob)).trim_right('=')
}

pub fn (k HostKey) fingerprint() string {
	return fingerprint(k.blob)
}

// verify_sig checks signature (a wire blob) over data for the negotiated algorithm name.
fn (k HostKey) verify_sig(alg string, data []u8, sig_blob []u8) ! {
	mut outer := new_reader(sig_blob)
	got_alg := outer.get_str()!
	if got_alg != alg {
		return error('signature uses ${got_alg}, expected ${alg}')
	}
	mut sig := outer.get_bytes()!
	match alg {
		'ssh-ed25519' {
			if k.kind != alg || !ed25519.verify(ed25519.PublicKey(k.point), data, sig)! {
				return error('host key signature verification failed')
			}
		}
		'ssh-rsa' {
			mut c := cng.instance()!
			c.rsa_verify(k.rsa, 'sha1', sha1.sum(data), sig)!
		}
		'rsa-sha2-256' {
			mut c := cng.instance()!
			c.rsa_verify(k.rsa, 'sha256', sha256.sum(data), sig)!
		}
		'rsa-sha2-512' {
			mut c := cng.instance()!
			c.rsa_verify(k.rsa, 'sha512', sha512.sum512(data), sig)!
		}
		else {
			if !alg.starts_with('ecdsa-sha2-') || k.kind != alg {
				return error('unsupported signature ${alg}')
			}
			mut pair := new_reader(sig)
			r := pair.get_mpint()!
			s := pair.get_mpint()!
			mut c := cng.instance()!
			c.ecdsa_verify(k.curve, k.point, ecdsa_digest(k.curve, data), r, s)!
		}
	}
}

fn ecdsa_digest(curve cng.Curve, data []u8) []u8 {
	return match curve.ssh_id {
		'nistp256' { sha256.sum(data) }
		'nistp384' { sha512.sum384(data) }
		else { sha512.sum512(data) }
	}
}

// is_rsa_alg reports a signature algorithm that uses an RSA key.
fn is_rsa_alg(alg string) bool {
	return alg == 'ssh-rsa' || alg.starts_with('rsa-sha2-')
}
