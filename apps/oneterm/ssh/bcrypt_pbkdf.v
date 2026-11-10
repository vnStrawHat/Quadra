// bcrypt_pbkdf, the key derivation of OpenSSH private keys (OpenBSD).
module ssh

import crypto.blowfish
import crypto.sha512

const bcrypt_magic = 'OxychromaticBlowfishSwatDynamite'.bytes()

// The work factors a key file may ask for. ssh-keygen writes 16 bcrypt rounds (-a 100 is the
// usual hardening); a round costs about 7 ms in a release build, so a key file asking for 2^31
// would hang the unlock for months. PBKDF2 (PKCS#8) costs about 2 us an iteration.
const max_bcrypt_rounds = 1024
const max_pbkdf2_iterations = 2_000_000

fn bcrypt_hash(sha2pass []u8, sha2salt []u8) ![]u8 {
	mut bf := blowfish.new_salted_cipher(sha2pass, sha2salt)!
	for _ in 0 .. 64 {
		blowfish.expand_key(sha2salt, mut bf)
		blowfish.expand_key(sha2pass, mut bf)
	}
	mut data := bcrypt_magic.clone()
	mut block := []u8{len: 8}
	for _ in 0 .. 64 {
		for i := 0; i < 32; i += 8 {
			bf.encrypt(mut block, data[i..i + 8])
			copy(mut data[i..], block)
		}
	}
	// the output words are little-endian, the block cipher big-endian
	mut out := []u8{len: 32}
	for i := 0; i < 32; i += 4 {
		out[i + 3] = data[i]
		out[i + 2] = data[i + 1]
		out[i + 1] = data[i + 2]
		out[i] = data[i + 3]
	}
	return out
}

fn bcrypt_pbkdf(password []u8, salt []u8, rounds int, keylen int) ![]u8 {
	if rounds < 1 || password.len == 0 || salt.len == 0 || keylen == 0 || keylen > 1024 {
		return error('invalid bcrypt_pbkdf parameters')
	}
	if rounds > max_bcrypt_rounds {
		return error('the key asks for ${rounds} bcrypt rounds (at most ${max_bcrypt_rounds})')
	}
	sha2pass := sha512.sum512(password)
	stride := (keylen + 31) / 32
	mut amt := (keylen + stride - 1) / stride
	mut key := []u8{len: keylen}
	mut remaining := keylen
	for count := 1; remaining > 0; count++ {
		mut countsalt := salt.clone()
		countsalt << u8(count >> 24)
		countsalt << u8(count >> 16)
		countsalt << u8(count >> 8)
		countsalt << u8(count)
		mut sha2salt := sha512.sum512(countsalt)
		mut tmp := bcrypt_hash(sha2pass, sha2salt)!
		mut out := tmp.clone()
		for _ in 1 .. rounds {
			sha2salt = sha512.sum512(tmp)
			tmp = bcrypt_hash(sha2pass, sha2salt)!
			for j in 0 .. 32 {
				out[j] ^= tmp[j]
			}
		}
		if amt > remaining {
			amt = remaining
		}
		mut i := 0
		for i < amt {
			dest := i * stride + (count - 1)
			if dest >= keylen {
				break
			}
			key[dest] = out[i]
			i++
		}
		remaining -= i
	}
	return key
}
