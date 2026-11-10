// The vault lifecycle (set up, unlock, change, reset) and sealing (specs/25).
// Locked is holding a VaultHeader and nothing else; unlocked is holding an UnlockedVault.
// unlock verifies against the header the caller holds, never vault.json, so replacing the file
// cannot open a locked window.
module vault

import crypto.aes
import crypto.rand
import encoding.utf8

const key_wrap_purpose = 'key-wrap'

pub enum SecretPurpose {
	ssh_password // an SSH login password, bound to host, port and user
}

fn (p SecretPurpose) label() string {
	return match p {
		.ssh_password { 'ssh-password' }
	}
}

fn (p SecretPurpose) max_plaintext() int {
	return match p {
		.ssh_password { 1024 }
	}
}

// SealContext is the address a sealed SSH password belongs to. The session id is deliberately
// not part of it, so a duplicated session keeps a working copy.
pub struct SealContext {
pub:
	host     string
	port     u16
	username string
}

fn (c SealContext) associated_data(purpose SecretPurpose, vault_id []u8) []u8 {
	return associated_data(purpose.label(), vault_id, [c.host.bytes(),
		[u8(c.port >> 8), u8(c.port)], c.username.bytes()])
}

// UnlockedVault holds the header and the data key. lock() zeroes the key.
@[heap]
pub struct UnlockedVault {
mut:
	header   VaultHeader
	data_key []u8
}

// init sets up a new vault in `dir` under `password` and returns it unlocked. Create-only.
// Costs one Argon2id run; call it off the UI thread.
pub fn init(dir string, password string, read_only bool) !&UnlockedVault {
	return init_with(dir, password, kdf_default, read_only)
}

fn init_with(dir string, password string, params KdfParams, read_only bool) !&UnlockedVault {
	if password == '' {
		return VaultError{
			kind: .empty_password
		}
	}
	vault_id := random(16)!
	mut data_key := random(key_len)!
	defer {
		wipe(mut data_key)
	}
	header := wrap_data_key(vault_id, data_key, password, params)!
	write_create(dir, header, read_only)!
	return &UnlockedVault{
		header:   header
		data_key: data_key.clone()
	}
}

// unlock verifies `password` against the header in memory. Costs one Argon2id run.
pub fn unlock(header VaultHeader, password string) !&UnlockedVault {
	if password == '' {
		return VaultError{
			kind: .empty_password
		}
	}
	return &UnlockedVault{
		header:   header
		data_key: unwrap_data_key(header, password)!
	}
}

// reset removes vault.json and its backup. Owners of sealed values must wipe them first.
pub fn reset(dir string, read_only bool) ! {
	write_remove(dir, read_only)!
}

// vault_id is the 16-byte id of this vault generation.
pub fn (v &UnlockedVault) vault_id() []u8 {
	return v.header.vault_id
}

// header is what a later unlock verifies against; keep it when locking.
pub fn (v &UnlockedVault) header() VaultHeader {
	return v.header
}

// needs_rehash reports a header written with cheaper parameters than the defaults.
pub fn (v &UnlockedVault) needs_rehash() bool {
	return v.header.kdf.is_below_default()
}

// str carries no key material.
pub fn (v &UnlockedVault) str() string {
	return 'UnlockedVault{${v.header.vault_id.hex()}}'
}

// lock zeroes the data key. The vault cannot seal or open afterwards.
pub fn (mut v UnlockedVault) lock() {
	wipe(mut v.data_key)
	v.data_key = []u8{}
}

// seal encrypts `plaintext` for `purpose` at `context` under a fresh random nonce.
pub fn (v &UnlockedVault) seal(purpose SecretPurpose, context SealContext, plaintext string) !SealedSecret {
	if plaintext.len > purpose.max_plaintext() {
		return VaultError{
			kind: .secret_too_long
		}
	}
	aad := context.associated_data(purpose, v.header.vault_id)
	return seal_bytes(v.data_key, v.header.vault_id, random(nonce_len)!, aad, plaintext.bytes())
}

// open decrypts a value sealed by seal for the same purpose and context. Errors are OpenError.
pub fn (v &UnlockedVault) open(purpose SecretPurpose, context SealContext, value SealedSecret) !string {
	envelope := decode_envelope(value, purpose.max_plaintext())!
	if envelope.kid != v.header.vault_id[..kid_len] {
		return OpenError{
			kind: .other_vault
		}
	}
	aad := context.associated_data(purpose, v.header.vault_id)
	mut plain := open_bytes(v.data_key, envelope, aad)!
	defer {
		wipe(mut plain)
	}
	text := plain.bytestr()
	if !utf8.validate_str(text) {
		return OpenError{
			kind: .rejected
		}
	}
	return text
}

// change_password rewraps the same data key under `new_password`, with a fresh salt and the
// default parameters, and writes vault.json. Refused when the file is not this vault any more.
pub fn (mut v UnlockedVault) change_password(dir string, old_password string, new_password string, read_only bool) ! {
	v.change_password_with(dir, old_password, new_password, kdf_default, read_only)!
}

fn (mut v UnlockedVault) change_password_with(dir string, old_password string, new_password string, params KdfParams, read_only bool) ! {
	if new_password == '' {
		return VaultError{
			kind: .empty_password
		}
	}
	mut data_key := unwrap_data_key(v.header, old_password)!
	defer {
		wipe(mut data_key)
	}
	header := wrap_data_key(v.header.vault_id, data_key, new_password, params)!
	write_rewrap(dir, v.header, header, read_only)!
	v.header = header
}

fn random(n int) ![]u8 {
	return rand.bytes(n) or { return VaultError{
		kind: .randomness
	} }
}

fn wrap_data_key(vault_id []u8, data_key []u8, password string, params KdfParams) !VaultHeader {
	salt := random(salt_len)!
	mut kek := derive_kek(password, salt, params)!
	defer {
		wipe(mut kek)
	}
	aad := associated_data(key_wrap_purpose, vault_id, [])
	return VaultHeader{
		vault_id:    vault_id
		kdf:         params
		salt:        salt
		wrapped_key: seal_bytes(kek, vault_id, random(nonce_len)!, aad, data_key)!
	}
}

// The GCM tag of the wrap is the password check.
fn unwrap_data_key(header VaultHeader, password string) ![]u8 {
	mut kek := derive_kek(password, header.salt, header.kdf)!
	defer {
		wipe(mut kek)
	}
	envelope := decode_envelope(header.wrapped_key, key_len) or {
		return VaultError{
			kind:   .unsupported
			detail: 'wrapped_key is not a v1 envelope'
		}
	}
	aad := associated_data(key_wrap_purpose, header.vault_id, [])
	data_key := open_bytes(kek, envelope, aad) or { return VaultError{
		kind: .wrong_password
	} }
	if data_key.len != key_len {
		return VaultError{
			kind: .wrong_password
		}
	}
	return data_key
}

fn seal_bytes(key []u8, vault_id []u8, nonce []u8, aad []u8, plaintext []u8) !SealedSecret {
	gcm := aes.new_aes_gcm(key) or {
		return VaultError{
			kind:   .unsupported
			detail: 'AES-256 key length'
		}
	}
	out := gcm.encrypt(plaintext, nonce, aad) or { return VaultError{
		kind: .secret_too_long
	} }
	return Envelope{
		kid:    vault_id[..kid_len].clone()
		nonce:  nonce
		sealed: out
	}.encode()
}

fn open_bytes(key []u8, envelope Envelope, aad []u8) ![]u8 {
	gcm := aes.new_aes_gcm(key) or { return OpenError{
		kind: .rejected
	} }
	return gcm.decrypt(envelope.sealed, envelope.nonce, aad) or {
		return OpenError{
			kind: .rejected
		}
	}
}
