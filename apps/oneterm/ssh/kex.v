// Algorithm negotiation (RFC 4253 section 7.1) and the exchange hash inputs.
module ssh

import crypto.sha256
import crypto.sha512

const kex_strict_client = 'kex-strict-c-v00@openssh.com'
const kex_strict_server = 'kex-strict-s-v00@openssh.com'

const kex_algorithms = [
	'curve25519-sha256',
	'curve25519-sha256@libssh.org',
	'ecdh-sha2-nistp256',
	'ecdh-sha2-nistp384',
	'ecdh-sha2-nistp521',
	'diffie-hellman-group-exchange-sha256',
	'diffie-hellman-group16-sha512',
	'diffie-hellman-group18-sha512',
	'diffie-hellman-group14-sha256',
]

const host_key_algorithms = [
	'ssh-ed25519',
	'ecdsa-sha2-nistp256',
	'ecdsa-sha2-nistp384',
	'ecdsa-sha2-nistp521',
	'rsa-sha2-512',
	'rsa-sha2-256',
	'ssh-rsa',
]

const cipher_algorithms = [
	'chacha20-poly1305@openssh.com',
	'aes256-gcm@openssh.com',
	'aes128-gcm@openssh.com',
	'aes256-ctr',
	'aes192-ctr',
	'aes128-ctr',
]

const mac_algorithms = [
	'hmac-sha2-512-etm@openssh.com',
	'hmac-sha2-256-etm@openssh.com',
	'hmac-sha2-512',
	'hmac-sha2-256',
	'hmac-sha1-etm@openssh.com',
	'hmac-sha1',
]

struct KexInit {
	kex           []string
	host_key      []string
	enc_cs        []string
	enc_sc        []string
	mac_cs        []string
	mac_sc        []string
	comp_cs       []string
	comp_sc       []string
	first_follows bool
}

// Offer is the algorithm lists of a connection, most preferred first.
struct Offer {
	kex      []string = kex_algorithms
	host_key []string = host_key_algorithms
	cipher   []string = cipher_algorithms
	mac      []string = mac_algorithms
}

fn build_kexinit(offer Offer) []u8 {
	mut w := new_writer(512)
	w.put_u8(msg_kexinit)
	w.put_raw(random_bytes(16))
	mut kex := offer.kex.clone()
	kex << 'ext-info-c'
	kex << kex_strict_client
	w.put_names(kex)
	w.put_names(offer.host_key)
	w.put_names(offer.cipher)
	w.put_names(offer.cipher)
	w.put_names(offer.mac)
	w.put_names(offer.mac)
	w.put_str('none')
	w.put_str('none')
	w.put_str('')
	w.put_str('')
	w.put_bool(false)
	w.put_u32(0)
	return w.buf
}

fn parse_kexinit(pkt []u8) !KexInit {
	if pkt.len < 17 { // the message type and the 16-byte cookie
		return error('truncated KEXINIT')
	}
	mut r := new_reader(pkt[17..])
	kex := r.get_names()!
	host_key := r.get_names()!
	enc_cs := r.get_names()!
	enc_sc := r.get_names()!
	mac_cs := r.get_names()!
	mac_sc := r.get_names()!
	comp_cs := r.get_names()!
	comp_sc := r.get_names()!
	r.get_names()!
	r.get_names()!
	first_follows := r.get_bool()!
	return KexInit{kex, host_key, enc_cs, enc_sc, mac_cs, mac_sc, comp_cs, comp_sc, first_follows}
}

// pick returns the first client entry the server offers.
fn pick(what string, client []string, server []string) !string {
	for name in client {
		if name in server {
			return name
		}
	}
	return error('no common ${what}: the server offers ${server.join(', ')}')
}

// CipherInfo describes a negotiated cipher.
struct CipherInfo {
	key  int
	iv   int
	aead bool
}

fn cipher_info(name string) !CipherInfo {
	return match name {
		'chacha20-poly1305@openssh.com' { CipherInfo{64, 0, true} }
		'aes256-gcm@openssh.com' { CipherInfo{32, 12, true} }
		'aes128-gcm@openssh.com' { CipherInfo{16, 12, true} }
		'aes256-ctr' { CipherInfo{32, 16, false} }
		'aes192-ctr' { CipherInfo{24, 16, false} }
		'aes128-ctr' { CipherInfo{16, 16, false} }
		else { error('unknown cipher ${name}') }
	}
}

struct Negotiated {
	kex      string
	host_key string
	enc_cs   string
	enc_sc   string
	mac_cs   string
	mac_sc   string
}

fn negotiate(c KexInit, s KexInit) !Negotiated {
	return Negotiated{
		kex:      pick('key exchange', c.kex, s.kex)!
		host_key: pick('host key algorithm', c.host_key, s.host_key)!
		enc_cs:   pick('cipher', c.enc_cs, s.enc_cs)!
		enc_sc:   pick('cipher', c.enc_sc, s.enc_sc)!
		mac_cs:   pick('MAC', c.mac_cs, s.mac_cs)!
		mac_sc:   pick('MAC', c.mac_sc, s.mac_sc)!
	}
}

// kex_hash hashes with the algorithm of the negotiated key exchange.
fn kex_hash(kex string, data []u8) []u8 {
	if kex.ends_with('sha512') || kex == 'ecdh-sha2-nistp521' {
		return sha512.sum512(data)
	}
	if kex == 'ecdh-sha2-nistp384' {
		return sha512.sum384(data)
	}
	return sha256.sum(data)
}

// derive_key is RFC 4253 section 7.2: K1 = HASH(K || H || letter || session_id), extended by
// HASH(K || H || K1 .. Kn) until enough bytes.
fn derive_key(kex string, k_mpint []u8, h []u8, letter u8, session_id []u8, need int) []u8 {
	mut seed := []u8{cap: k_mpint.len + h.len + 1 + session_id.len}
	seed << k_mpint
	seed << h
	seed << letter
	seed << session_id
	mut out := kex_hash(kex, seed)
	for out.len < need {
		mut more := []u8{cap: k_mpint.len + h.len + out.len}
		more << k_mpint
		more << h
		more << out
		out << kex_hash(kex, more)
	}
	return out[..need]
}

// make_sealer builds the protection of one direction from the derived keys.
fn make_sealer(name string, mac string, key []u8, iv []u8, mac_key []u8) !Sealer {
	if name.starts_with('chacha20') {
		return new_chacha_sealer(key)!
	}
	if name.contains('-gcm@') {
		return new_gcm_sealer(key, iv)!
	}
	alg := mac_info(mac)!
	return new_ctr_sealer(key, iv, alg, mac_key)!
}
