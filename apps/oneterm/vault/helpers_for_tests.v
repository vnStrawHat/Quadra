// Helpers shared by this module's tests; nothing else calls them.
module vault

import encoding.base64
import os
import rand

const test_root = os.join_path(@VMODROOT, '.tmp', 'oneterm-tests', 'vault')

// The cheapest parameters that derive quickly. Below the floor: only for a vault that is not
// re-read from disk, since load and a rewrap refuse headers below the floor.
const fast_params = KdfParams{64, 1, 1}

// The cheapest header the load rules accept.
const floor_params = kdf_floor

const ctx = SealContext{
	host:     '10.0.0.1'
	port:     22
	username: 'deploy'
}

const v1_vault_json = '{
  "schema_version": 1,
  "vault_id": "3f9c0a4e8b1d4c27a6e2f0b9c1d2e3f4",
  "kdf": {
    "algorithm": "argon2id",
    "version": 19,
    "memory_kib": 19456,
    "iterations": 2,
    "parallelism": 1,
    "salt": "ZGVmZ2hpamtsbW5vcHFycw=="
  },
  "wrapped_key": "otv1:P5wKTosdTCelpaWlpaWlpaWlpaUzyTFHXneCTy6+xl6CSv21I+nvOpS2v5BasAtrO8y3aUzVVxIC4l0LnfumEJMkr5c="
}'

const v1_password = 'correct horse battery staple'
const v1_sealed = 'otv1:P5wKTosdTCdcXFxcXFxcXFxcXFxbpgEB6ylBEMijZRhddr55R0kOe/uq'

fn test_dir(name string) string {
	dir := os.join_path(test_root, name, '${os.getpid()}-${rand.hex(8)}')
	os.rmdir_all(dir) or {} // rand repeats between runs: never inherit a leftover
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

fn vault_path(dir string) string {
	return os.join_path(dir, document_name)
}

fn fixed_vault(data_key []u8, vault_id []u8, kdf KdfParams) &UnlockedVault {
	return &UnlockedVault{
		header:   VaultHeader{
			vault_id:    vault_id
			kdf:         kdf
			salt:        []u8{len: 16, init: 7}
			wrapped_key: sealed('otv1:AA')
		}
		data_key: data_key
	}
}

fn fast_vault(dir string, password string) &UnlockedVault {
	return init_with(dir, password, fast_params, false) or { panic(err) }
}

fn floor_vault(dir string, password string) &UnlockedVault {
	return init_with(dir, password, floor_params, false) or { panic(err) }
}

fn payload_of(n int) string {
	return base64.encode([]u8{len: envelope_overhead + n})
}
