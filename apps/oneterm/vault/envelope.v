// The persisted envelope: `otv1:` + base64( kid[8] | nonce[12] | ciphertext | tag[16] ), and the
// associated data every seal authenticates (specs/25). Byte-compatible with the Rust app.
module vault

import encoding.base64

const v1_prefix = 'otv1:'
const kid_len = 8
const nonce_len = 12
const tag_len = 16
const envelope_overhead = kid_len + nonce_len + tag_len
const aad_layout_version = u8(0x01)

// SealedSecret is a sealed value exactly as persisted. Any string is accepted and written back
// byte for byte, so an `otv2:` value survives; whether it opens is decided only by open().
pub struct SealedSecret {
pub:
	value string
}

// sealed wraps a persisted value without judging it.
pub fn sealed(value string) SealedSecret {
	return SealedSecret{value}
}

// raw is the text to persist.
pub fn (s SealedSecret) raw() string {
	return s.value
}

// str shows the version tag only, never the payload.
pub fn (s SealedSecret) str() string {
	if tag := version_tag(s.value) {
		return '${tag}:<sealed>'
	}
	return '<sealed>'
}

// `otvN` when the value starts with a canonical `otvN:` tag (N >= 1, no leading zero).
fn version_tag(value string) ?string {
	colon := value.index(':') or { return none }
	tag := value[..colon]
	if !tag.starts_with('otv') {
		return none
	}
	digits := tag[3..]
	if digits == '' || !digits.contains_only('0123456789') || digits[0] == `0` {
		return none
	}
	return tag
}

struct Envelope {
	kid    []u8
	nonce  []u8
	sealed []u8 // ciphertext followed by the 16-byte tag
}

fn (e Envelope) encode() SealedSecret {
	mut payload := []u8{cap: kid_len + nonce_len + e.sealed.len}
	payload << e.kid
	payload << e.nonce
	payload << e.sealed
	return SealedSecret{'${v1_prefix}${base64.encode(payload)}'}
}

// strict_base64 refuses anything but canonical, padded base64.
fn strict_base64(text string) ?[]u8 {
	if text.len % 4 != 0 {
		return none
	}
	decoded := base64.decode(text)
	return if base64.encode(decoded) == text { decoded } else { none }
}

fn decode_envelope(value SealedSecret, max_plaintext int) !Envelope {
	if !value.value.starts_with(v1_prefix) {
		return OpenError{
			kind: if version_tag(value.value) != none { .newer_format } else { .malformed }
		}
	}
	payload := strict_base64(value.value[v1_prefix.len..]) or {
		return OpenError{
			kind: .malformed
		}
	}
	if payload.len < envelope_overhead || payload.len > envelope_overhead + max_plaintext {
		return OpenError{
			kind: .malformed
		}
	}
	return Envelope{
		kid:    payload[..kid_len].clone()
		nonce:  payload[kid_len..kid_len + nonce_len].clone()
		sealed: payload[kid_len + nonce_len..].clone()
	}
}

// associated_data is the frozen layout:
// "oneterm-vault" 0x00 | 0x01 | u8 len(purpose) | purpose | vault_id[16]
//     | for each context field: u32 big-endian length | bytes
fn associated_data(purpose string, vault_id []u8, context [][]u8) []u8 {
	mut aad := []u8{cap: 64}
	aad << 'oneterm-vault'.bytes()
	aad << u8(0)
	aad << aad_layout_version
	aad << u8(purpose.len)
	aad << purpose.bytes()
	aad << vault_id
	for field in context {
		aad << u8(field.len >> 24)
		aad << u8(field.len >> 16)
		aad << u8(field.len >> 8)
		aad << u8(field.len)
		aad << field
	}
	return aad
}
