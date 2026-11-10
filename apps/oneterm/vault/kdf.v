// Argon2id: the master password to the key-encryption key.
module vault

import crypto.argon2

// KdfParams are the Argon2id costs stored in vault.json beside the salt.
pub struct KdfParams {
pub:
	memory_kib  u32
	iterations  u32
	parallelism u32
}

// kdf_default: what set-up and a password change write (64 MiB, 3 passes, 1 lane).
pub const kdf_default = KdfParams{64 * 1024, 3, 1}

// kdf_floor: the least a header may ask for; below it the header reads as unsupported.
pub const kdf_floor = KdfParams{19 * 1024, 2, 1}

// kdf_ceiling: the most a header may ask for, so a tampered header cannot be a memory bomb.
pub const kdf_ceiling = KdfParams{1024 * 1024, 16, 4}

const salt_len = 16
const salt_len_max = 64
const key_len = 32
const algorithm_name = 'argon2id'
const algorithm_version = u32(0x13)

fn (p KdfParams) is_within_bounds() bool {
	return p.memory_kib >= kdf_floor.memory_kib && p.memory_kib <= kdf_ceiling.memory_kib
		&& p.iterations >= kdf_floor.iterations && p.iterations <= kdf_ceiling.iterations
		&& p.parallelism >= kdf_floor.parallelism && p.parallelism <= kdf_ceiling.parallelism
}

fn (p KdfParams) is_below_default() bool {
	return p.memory_kib < kdf_default.memory_kib || p.iterations < kdf_default.iterations
}

// derive_kek derives the key-encryption key from the password bytes exactly as typed.
fn derive_kek(password string, salt []u8, params KdfParams) ![]u8 {
	mut pw := password.bytes()
	defer {
		wipe(mut pw)
	}
	return argon2.id_key(pw, salt, params.iterations, params.memory_kib, u8(params.parallelism),
		u32(key_len)) or {
		return VaultError{
			kind:   .unsupported
			detail: 'Argon2id: ${err.msg()}'
		}
	}
}

// wipe zeroes key material in place.
fn wipe(mut buf []u8) {
	if buf.len > 0 {
		unsafe { vmemset(buf.data, 0, buf.len) }
	}
}
