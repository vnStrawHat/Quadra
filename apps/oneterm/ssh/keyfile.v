// Private key files: format dispatch and the OpenSSH format (PROTOCOL.key).
module ssh

import encoding.base64
import os
import x.crypto.poly1305
import quadra.native.cng

const openssh_magic = 'openssh-key-v1\0'.bytes()

// load_private_key reads a key file, decrypting with the passphrase when it is encrypted.
pub fn load_private_key(path string, passphrase ?Secret) !PrivateKey {
	if !os.is_file(path) {
		return error('failed to load key ${path}: not a regular file')
	}
	text := os.read_file(path) or { return error('failed to load key ${path}: ${err}') }
	return decode_private_key(text, passphrase) or {
		return error('failed to load key ${path}: ${err.msg()}')
	}
}

fn decode_private_key(text string, passphrase ?Secret) !PrivateKey {
	mut pass := []u8{}
	if p := passphrase {
		pass = p.bytes.clone()
	}
	defer {
		wipe(mut pass)
	}
	pem := parse_pem(text)!
	return match pem.label {
		'OPENSSH PRIVATE KEY' { decode_openssh(pem.body, pass)! }
		'RSA PRIVATE KEY' { decode_pkcs1(legacy_decrypt(pem, pass)!)! }
		'EC PRIVATE KEY' { decode_sec1(legacy_decrypt(pem, pass)!, none)! }
		'PRIVATE KEY' { decode_pkcs8(pem.body)! }
		'ENCRYPTED PRIVATE KEY' { decode_pkcs8(pbes2_decrypt(pem.body, pass)!)! }
		else { return error('unsupported key format: ${pem.label}') }
	}
}

// Pem is one decoded PEM block.
struct Pem {
	label   string
	headers map[string]string
	body    []u8
}

fn parse_pem(text string) !Pem {
	lines := text.split_into_lines()
	mut label := ''
	mut headers := map[string]string{}
	mut b64 := []string{}
	for raw in lines {
		line := raw.trim_space()
		if label == '' {
			if line.starts_with('-----BEGIN ') && line.ends_with('-----') {
				label = line.after('BEGIN ').all_before('-----').trim_space()
			}
			continue
		}
		if line.starts_with('-----END ') {
			body := base64.decode(b64.join(''))
			if body.len == 0 {
				return error('empty PEM body')
			}
			return Pem{label, headers, body}
		}
		if line.contains(': ') && b64.len == 0 {
			headers[line.all_before(': ')] = line.all_after(': ')
			continue
		}
		if line != '' {
			b64 << line
		}
	}
	return error('no PEM key found')
}

fn decode_openssh(blob []u8, pass []u8) !PrivateKey {
	if blob.len < openssh_magic.len || blob[..openssh_magic.len] != openssh_magic {
		return error('bad OpenSSH key magic')
	}
	mut r := new_reader(blob[openssh_magic.len..])
	cipher := r.get_str()!
	kdf := r.get_str()!
	kdf_options := r.get_bytes()!
	if r.get_u32()! != 1 {
		return error('only single-key files are supported')
	}
	r.get_bytes()! // the public key, repeated inside the private section
	encrypted := r.get_bytes()!
	tag := r.buf[r.pos..]
	mut private := openssh_decrypt(cipher, kdf, kdf_options, pass, encrypted, tag)!
	defer {
		wipe(mut private)
	}
	mut p := new_reader(private)
	check_a := p.get_u32()! // two statements: the order of an expression's calls is not defined
	check_b := p.get_u32()!
	if check_a != check_b {
		if cipher == 'none' {
			return error('corrupt key')
		}
		return error('wrong passphrase')
	}
	return decode_openssh_key(mut p)
}

fn openssh_decrypt(cipher string, kdf string, kdf_options []u8, pass []u8, encrypted []u8, tag []u8) ![]u8 {
	if cipher == 'none' {
		return encrypted.clone()
	}
	if kdf != 'bcrypt' {
		return error('unsupported key KDF ${kdf}')
	}
	if pass.len == 0 {
		return error('the key needs a passphrase')
	}
	keylen, ivlen := match cipher {
		'aes128-ctr', 'aes128-cbc', 'aes128-gcm@openssh.com' { 16, 16 }
		'aes192-ctr', 'aes192-cbc' { 24, 16 }
		'aes256-ctr', 'aes256-cbc', 'aes256-gcm@openssh.com' { 32, 16 }
		'chacha20-poly1305@openssh.com' { 64, 0 }
		else { return error('unsupported key cipher ${cipher}') }
	}

	ivreal := if cipher.contains('gcm') { 12 } else { ivlen }
	mut opts := new_reader(kdf_options)
	salt := opts.get_bytes()!
	rounds := int(opts.get_u32()!)
	mut km := bcrypt_pbkdf(pass, salt, rounds, keylen + ivreal)!
	defer {
		wipe(mut km)
	}
	// Views into km, so the deferred wipe clears the key and iv too.
	key := unsafe { km[..keylen] }
	iv := unsafe { km[keylen..] }
	mut data := encrypted.clone()
	mut c := cng.instance()!
	if cipher.ends_with('-ctr') {
		h := c.aes_key('ChainingModeECB', key)!
		defer {
			c.destroy(h)
		}
		mut ivm := iv.clone()
		c.aes_ctr_xor(h, mut ivm, mut data)!
	} else if cipher.ends_with('-cbc') {
		c.aes_cbc_decrypt(key, iv, mut data)!
	} else if cipher.contains('-gcm@') {
		if tag.len < 16 {
			return error('truncated key file')
		}
		h := c.aes_key('ChainingModeGCM', key)!
		defer {
			c.destroy(h)
		}
		c.aes_gcm_open(h, iv, [], mut data, tag[..16]) or { return error('wrong passphrase') }
	} else {
		if tag.len < 16 {
			return error('truncated key file')
		}
		nonce := seq_nonce(0)
		pk := poly_key(key[..32], nonce)!
		if !poly1305.verify_tag(tag[..16], data, pk) {
			return error('wrong passphrase')
		}
		xor_at(key[..32], nonce, 1, mut data)!
	}
	return data
}

fn decode_openssh_key(mut p Reader) !PrivateKey {
	kind := p.get_str()!
	match kind {
		'ssh-ed25519' {
			public := p.get_bytes()!
			secret_ := p.get_bytes()!
			if public.len != 32 || secret_.len != 64 {
				return error('bad ed25519 key')
			}
			return PrivateKey{
				kind:        kind
				public_blob: ed_public_blob(public)
				seed:        secret_[..32].clone()
			}
		}
		'ssh-rsa' {
			n := p.get_mpint()!
			e := p.get_mpint()!
			p.get_mpint()! // d: CNG only needs n, e, p, q
			p.get_mpint()! // iqmp
			prime1 := p.get_mpint()!
			prime2 := p.get_mpint()!
			rsa := cng.RsaParts{
				n: n.clone()
				e: e.clone()
				p: prime1.clone()
				q: prime2.clone()
			}
			return PrivateKey{
				kind:        kind
				public_blob: rsa_public_blob(rsa)
				rsa:         rsa
			}
		}
		else {
			if !kind.starts_with('ecdsa-sha2-') {
				return error('unsupported key type ${kind}')
			}
			c := cng.curve_by_name(p.get_str()!)!
			point := p.get_bytes()!
			d := p.get_mpint()!
			return PrivateKey{
				kind:        kind
				public_blob: ec_public_blob(c, point)
				curve:       c
				point:       point.clone()
				d:           d.clone()
			}
		}
	}
}
