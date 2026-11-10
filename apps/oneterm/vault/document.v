// vault.json, schema 1: reading it into a VaultStatus and the one function every write goes
// through. Unlike every other document it is never quarantined and never replaced by a default.
module vault

import encoding.base64
import os
import store
import x.json2

pub const document_name = 'vault.json'
const schema_version = u32(1)

// VaultHeader is everything vault.json holds. Nothing in it reveals the master password.
pub struct VaultHeader {
pub:
	vault_id    []u8
	kdf         KdfParams
	salt        []u8
	wrapped_key SealedSecret
}

// str leaves the wrapped key and the salt out.
pub fn (h VaultHeader) str() string {
	return 'VaultHeader{vault_id: ${h.vault_id.hex()}, kdf: ${h.kdf}}'
}

pub enum StatusKind {
	uninitialised // no vault.json: no master password has been set up
	ready         // a valid schema-1 header
	unsupported   // newer schema, unknown algorithm or parameters out of bounds
	damaged       // present but not parseable; only Reset removes it
}

// VaultStatus is what vault.json says about the vault.
pub struct VaultStatus {
pub:
	kind   StatusKind
	header VaultHeader
	reason string
}

// load reads `<dir>/vault.json`. Nothing is ever written by a load.
pub fn load(dir string) !VaultStatus {
	path := os.join_path(dir, document_name)
	if !os.exists(path) {
		return VaultStatus{
			kind: .uninitialised
		}
	}
	return parse(store.read_text(path)!)
}

fn damaged(reason string) VaultStatus {
	return VaultStatus{
		kind:   .damaged
		reason: reason
	}
}

fn unsupported(reason string) VaultStatus {
	return VaultStatus{
		kind:   .unsupported
		reason: reason
	}
}

fn parse(raw string) VaultStatus {
	value := store.parse_json(raw) or { return damaged('not JSON') }
	return parse_value(value)
}

fn u32_field(fields map[string]json2.Any, key string) ?u32 {
	number := store.u64_of(fields[key] or { return none }) or { return none }
	return if number <= max_u32 { u32(number) } else { none }
}

fn text_field(fields map[string]json2.Any, key string) ?string {
	return store.string_of(fields[key] or { return none })
}

// parse_value is the load rules on a parsed document; a rewrap checks the file against the same
// rules inside the lock, so it never writes over anything load would not call ready.
fn parse_value(value json2.Any) VaultStatus {
	version := store.schema_version(value) or { return damaged('schema_version is not an integer') }
	if version == 0 {
		return damaged('no schema_version')
	}
	if version != schema_version {
		return unsupported('schema version ${version} is newer than supported version ${schema_version}')
	}
	fields := store.object_of(value) or { return damaged('missing or mistyped fields') }
	id_text := text_field(fields, 'vault_id') or { return damaged('missing or mistyped fields') }
	kdf := store.object_of(fields['kdf'] or { return damaged('missing or mistyped fields') }) or {
		return damaged('missing or mistyped fields')
	}
	wrapped_text := text_field(fields, 'wrapped_key') or {
		return damaged('missing or mistyped fields')
	}
	algorithm := text_field(kdf, 'algorithm') or { return damaged('missing or mistyped fields') }
	kdf_version := u32_field(kdf, 'version') or { return damaged('missing or mistyped fields') }
	memory := u32_field(kdf, 'memory_kib') or { return damaged('missing or mistyped fields') }
	iterations := u32_field(kdf, 'iterations') or { return damaged('missing or mistyped fields') }
	parallelism := u32_field(kdf, 'parallelism') or { return damaged('missing or mistyped fields') }
	salt_text := text_field(kdf, 'salt') or { return damaged('missing or mistyped fields') }

	if algorithm != algorithm_name || kdf_version != algorithm_version {
		return unsupported('key derivation ${algorithm} version ${kdf_version}')
	}
	params := KdfParams{memory, iterations, parallelism}
	if !params.is_within_bounds() {
		return unsupported('Argon2id parameters outside the accepted bounds: ${params}')
	}
	vault_id := parse_hex_id(id_text) or {
		return damaged('vault_id is not 32 lowercase hex digits')
	}
	salt := strict_base64(salt_text) or { return damaged('salt is not base64') }
	if salt.len < salt_len || salt.len > salt_len_max {
		return unsupported('salt of ${salt.len} bytes')
	}
	wrapped := sealed(wrapped_text)
	envelope := decode_envelope(wrapped, key_len) or {
		if err is OpenError && err.kind == .newer_format {
			return unsupported('wrapped_key uses a newer envelope')
		}
		return damaged('wrapped_key is not an envelope')
	}
	if envelope.kid != vault_id[..kid_len] {
		return damaged('wrapped_key belongs to another vault')
	}
	if envelope.sealed.len != key_len + tag_len {
		return damaged('wrapped_key has the wrong length')
	}
	return VaultStatus{
		kind:   .ready
		header: VaultHeader{
			vault_id:    vault_id
			kdf:         params
			salt:        salt
			wrapped_key: wrapped
		}
	}
}

fn kdf_fields(h VaultHeader) map[string]json2.Any {
	mut kdf := map[string]json2.Any{}
	kdf['algorithm'] = json2.Any(algorithm_name)
	kdf['version'] = json2.Any(i64(algorithm_version))
	kdf['memory_kib'] = json2.Any(i64(h.kdf.memory_kib))
	kdf['iterations'] = json2.Any(i64(h.kdf.iterations))
	kdf['parallelism'] = json2.Any(i64(h.kdf.parallelism))
	kdf['salt'] = json2.Any(base64.encode(h.salt))
	return kdf
}

fn to_document(h VaultHeader) json2.Any {
	mut doc := map[string]json2.Any{}
	doc[store.schema_field] = json2.Any(i64(schema_version))
	doc['vault_id'] = json2.Any(h.vault_id.hex())
	doc['kdf'] = json2.Any(kdf_fields(h))
	doc['wrapped_key'] = json2.Any(h.wrapped_key.raw())
	return json2.Any(doc)
}

fn parse_hex_id(text string) ?[]u8 {
	if text.len != 32 || !text.contains_only('0123456789abcdef') {
		return none
	}
	mut id := []u8{len: 16}
	for i in 0 .. 16 {
		id[i] = u8(text[i * 2..i * 2 + 2].parse_uint(16, 8) or { return none })
	}
	return id
}

fn refuse_read_only(read_only bool) ! {
	if read_only {
		return VaultError{
			kind: .read_only
		}
	}
}

// write_create: set-up creates the document, refused when any file is there, whatever it holds.
fn write_create(dir string, header VaultHeader, read_only bool) ! {
	refuse_read_only(read_only)!
	path := os.join_path(dir, document_name)
	if os.is_file(path) {
		return VaultError{
			kind: .already_initialised
		}
	}
	store.update_secret_json_file(path, fn [path, header] (mut doc json2.Any) ! {
		if os.is_file(path) {
			return VaultError{
				kind: .already_initialised
			}
		}
		doc = to_document(header)
	}) or { return io_error(err) }
}

// write_rewrap replaces the KDF fields and the wrapped key, only when the file loads as ready
// and is still exactly the header this process unlocked from. Unknown fields are preserved.
fn write_rewrap(dir string, from VaultHeader, to VaultHeader, read_only bool) ! {
	refuse_read_only(read_only)!
	path := os.join_path(dir, document_name)
	store.update_secret_json_file(path, fn [from, to] (mut doc json2.Any) ! {
		on_disk := parse_value(doc)
		if on_disk.kind != .ready || on_disk.header.vault_id != from.vault_id {
			return VaultError{
				kind: .vault_replaced
			}
		}
		if on_disk.header.salt != from.salt
			|| on_disk.header.wrapped_key.value != from.wrapped_key.value {
			return VaultError{
				kind: .changed_elsewhere
			}
		}
		mut fields := store.object_of(doc) or { return error('not an object') }
		mut kdf := store.object_of(fields['kdf'] or { json2.Any('') }) or {
			map[string]json2.Any{}
		}
		for key, value in kdf_fields(to) {
			kdf[key] = value
		}
		fields['kdf'] = json2.Any(kdf)
		fields['wrapped_key'] = json2.Any(to.wrapped_key.raw())
		doc = json2.Any(fields)
	}) or { return rewrap_error(err) }
}

// A document that is no longer JSON is not the vault this process unlocked; a directory in its
// place is an I/O failure.
fn rewrap_error(err IError) VaultError {
	if err is store.InvalidData {
		return VaultError{
			kind: .vault_replaced
		}
	}
	return io_error(err)
}

// write_remove: Reset. Refused for a file written by a newer OneTerm.
fn write_remove(dir string, read_only bool) ! {
	refuse_read_only(read_only)!
	status := load(dir) or { return io_error(err) }
	if status.kind == .unsupported {
		return VaultError{
			kind:   .unsupported
			detail: status.reason
		}
	}
	store.remove_document(os.join_path(dir, document_name), false) or { return io_error(err) }
}
