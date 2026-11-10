// Fuzz: private key files (OpenSSH, PKCS#1, SEC1, PKCS#8, PBES2, legacy DEK-Info), DER and
// known_hosts lines. Keys here are throwaway, built in the test.
module ssh

import crypto.ed25519
import encoding.base64
import os
import time
import quadra.native.cng
import quadra.testing.fuzz

fn der(tag u8, body []u8) []u8 {
	mut out := [tag]
	if body.len < 0x80 {
		out << u8(body.len)
	} else {
		out << [u8(0x82), u8(body.len >> 8), u8(body.len)]
	}
	out << body
	return out
}

fn der_seq(parts [][]u8) []u8 {
	mut body := []u8{}
	for p in parts {
		body << p
	}
	return der(0x30, body)
}

fn der_int(v int) []u8 {
	return der(0x02, [u8(v >> 24), u8(v >> 16), u8(v >> 8), u8(v)])
}

fn pem_text(label string, headers string, body []u8) string {
	return '-----BEGIN ${label}-----\n${headers}${base64.encode(body)}\n-----END ${label}-----\n'
}

// openssh_key builds an ed25519 OpenSSH key, encrypted with aes256-ctr and 1 bcrypt round
// when pass is set.
fn openssh_key(pass string) []u8 {
	seed := []u8{len: 32, init: u8(index + 3)}
	priv := ed25519.new_key_from_seed(seed)
	public := priv.public_key()
	mut p := new_writer(256)
	p.put_u32(0x01020304)
	p.put_u32(0x01020304)
	p.put_str('ssh-ed25519')
	p.put_bytes(public)
	p.put_bytes(priv)
	p.put_str('fixture')
	for i := 1; p.buf.len % 16 != 0; i++ {
		p.put_u8(u8(i))
	}
	mut data := p.buf.clone()
	mut opts := new_writer(32)
	salt := []u8{len: 16, init: u8(index)}
	opts.put_bytes(salt)
	opts.put_u32(1)
	if pass != '' {
		km := bcrypt_pbkdf(pass.bytes(), salt, 1, 48) or { panic(err) }
		mut c := cng.instance() or { panic(err) }
		h := c.aes_key('ChainingModeECB', km[..32]) or { panic(err) }
		mut iv := km[32..].clone()
		c.aes_ctr_xor(h, mut iv, mut data) or { panic(err) }
		c.destroy(h)
	}
	mut w := new_writer(512)
	w.put_raw(openssh_magic)
	w.put_str(if pass != '' { 'aes256-ctr' } else { 'none' })
	w.put_str(if pass != '' { 'bcrypt' } else { 'none' })
	w.put_bytes(if pass != '' { opts.buf } else { []u8{} })
	w.put_u32(1)
	w.put_bytes(ed_public_blob(public))
	w.put_bytes(data)
	return w.buf
}

struct KeyCase {
	label   string
	headers string
	body    []u8
}

fn key_corpus() []KeyCase {
	oid := fn (o []u8) []u8 {
		return der(0x06, o)
	}
	point := der(0xa1, der(0x03, [u8(0), 4]))
	sec1 := der_seq([der_int(1), der(0x04, []u8{len: 32, init: 7}),
		der(0xa0, oid(oid_p256)),
		der(0xa1, der(0x03, []u8{len: 66, init: u8(if index == 1 {
			4
		} else {
			index
		})}))])
	pkcs1 := der_seq([der_int(0), der_int(0x7fffffff), der_int(65537),
		der_int(5), der_int(7), der_int(11)])
	ed8 := der_seq([der_int(0), der_seq([oid(oid_ed25519)]), der(0x04, der(0x04, []u8{len: 32}))])
	pbes2 := der_seq([
		der_seq([oid(oid_pbes2),
			der_seq([
				der_seq([oid(oid_pbkdf2),
					der_seq([der(0x04, []u8{len: 8, init: 1}),
						der_int(1), der_seq([oid(oid_hmac_sha256)])])]),
				der_seq([oid(oid_aes128_cbc), der(0x04, []u8{len: 16})]),
			])]),
		der(0x04, []u8{len: 32, init: 9}),
	])
	return [
		KeyCase{'OPENSSH PRIVATE KEY', '', openssh_key('')},
		KeyCase{'OPENSSH PRIVATE KEY', '', openssh_key('pw')},
		KeyCase{'EC PRIVATE KEY', '', sec1},
		KeyCase{'EC PRIVATE KEY', '', der_seq([der_int(1), der(0x04, []u8{len: 32}), point])},
		KeyCase{'RSA PRIVATE KEY', '', pkcs1},
		KeyCase{'RSA PRIVATE KEY', 'Proc-Type: 4,ENCRYPTED\nDEK-Info: AES-128-CBC,00112233445566778899AABBCCDDEEFF\n\n', []u8{len: 32, init: 3}},
		KeyCase{'PRIVATE KEY', '', ed8},
		KeyCase{'ENCRYPTED PRIVATE KEY', '', pbes2},
	]
}

fn test_corpus_keys_decode() {
	corpus := key_corpus()
	pw := secret('pw')
	for k in [0, 1, 2, 6] {
		c := corpus[k]
		decode_private_key(pem_text(c.label, c.headers, c.body), pw) or { panic('${k}: ${err}') }
	}
}

fn test_broken_keys_are_errors() {
	// a SEC1 public point with an empty BIT STRING sliced bits[1..]
	corpus := key_corpus()
	if _ := decode_private_key(pem_text('EC PRIVATE KEY', '', corpus[3].body), none) {
		assert false
	}
	// work factors from the file are capped, not run: these returned after hours or never
	mut sw := time.new_stopwatch()
	if _ := bcrypt_pbkdf('pw'.bytes(), 'salt'.bytes(), 0x7fffffff, 48) {
		assert false
	}
	mut patched := corpus[7].body.clone() // PBES2 with 1 iteration: make it 0x7f000001
	for i in 0 .. patched.len - 5 {
		if patched[i] == 0x02 && patched[i + 1] == 4 && patched[i + 5] == 1 {
			patched[i + 2] = 0x7f
			break
		}
	}
	if _ := decode_private_key(pem_text('ENCRYPTED PRIVATE KEY', '', patched), secret('pw')) {
		assert false
	}
	assert sw.elapsed().milliseconds() < 2000
	// a hostile known_hosts pattern: the recursive glob took exponential time
	assert !glob_match('*'.repeat(10) + 'a*'.repeat(20) + 'x', 'a'.repeat(60))
	assert glob_match('a*b?c*', 'axxbycz') && !glob_match('a*b?c', 'abc')
	assert sw.elapsed().milliseconds() < 2000
}

fn test_fuzz_private_keys() {
	n := fuzz.iters(300)
	mut rng := fuzz.new_rng(0x6e7)
	corpus := key_corpus()
	bodies := corpus.map(it.body)
	mut w := fuzz.watch('ssh private keys', n)
	for i in 0 .. n {
		c := corpus[rng.intn(corpus.len)]
		mut text := ''
		if rng.chance(70) {
			text = pem_text(c.label, c.headers, rng.mutate(c.body, bodies))
		} else {
			text = rng.mutate(pem_text(c.label, c.headers, c.body).bytes(), []).bytestr()
		}
		pass := if rng.chance(50) { ?Secret(secret('pw')) } else { ?Secret(none) }
		w.start()
		if mut key := decode_private_key(text, pass) {
			key.sign(key.kind, 'x'.bytes()) or {}
		}
		w.done(i, text.len, 30000) // 1024 bcrypt rounds, the cap, take seconds in a debug build
		fuzz_ceiling(i)
	}
	w.finish(64)
}

fn test_fuzz_known_hosts() {
	n := fuzz.iters(300)
	mut rng := fuzz.new_rng(0x4b4)
	blob := base64.encode(ed_public_blob(ed25519.new_key_from_seed([]u8{len: 32}).public_key()))
	lines := ['h ssh-ed25519 ${blob}', '[h]:2222,*.example.com,!bad.example.com ssh-ed25519 ${blob}',
		'|1|95gNYggxa9oatHgUeraaa0nz+kU=|WM2yVWqTLohfVRMkRga7/fwAMFs= ssh-ed25519 ${blob}',
		'@revoked h ssh-ed25519 ${blob}', '@cert-authority *.example.com ssh-ed25519 ${blob}',
		'# comment', 'h*?* ssh-rsa AAAA', '@revoked'].map(it.bytes())
	dir := os.join_path(@DIR, '..', '..', '..', '.tmp', 'ssh-unit')
	os.mkdir_all(dir) or { panic(err) }
	path := os.join_path(dir, 'known_hosts_fuzz')
	mut w := fuzz.watch('ssh known_hosts', n)
	for i in 0 .. n {
		mut text := []u8{}
		for _ in 0 .. 1 + rng.intn(6) {
			text << rng.mutate(lines[rng.intn(lines.len)], lines)
			text << `\n`
		}
		os.write_file(path, text.bytestr()) or { panic(err) }
		host := ['h', '[h]:2222', 'a.example.com', 'bad.example.com', 'x'.repeat(200)][rng.intn(5)]
		w.start()
		known_keys(path, host) or {}
		for line in text.bytestr().split_into_lines() {
			f := line.fields()
			if f.len > 0 {
				patterns_match(f[0], host)
			}
		}
		w.done(i, text.len, 2000)
		fuzz_ceiling(i)
	}
	os.rm(path) or {}
	w.finish(64)
}

fn test_non_ascii_base64_is_an_error() {
	// vlib's base64 decoder panicked on any byte past `z` (a mangled PEM or known_hosts line)
	for bad in ['AAA{', 'AAAA\xff\xfeAA', 'QUJD€===', 'AA~~'] {
		assert b64_decode(bad) == []u8{}
		if _ := decode_private_key(pem_text('PRIVATE KEY', '', []).replace('\n\n', '\n${bad}\n'),
			none)
		{
			assert false
		}
	}
	assert b64_decode('QUJD') == 'ABC'.bytes()
	assert !hashed_matches('|1|QUJ{|QUJD', 'h')
}
