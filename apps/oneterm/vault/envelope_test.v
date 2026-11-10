module vault

import encoding.base64
import encoding.hex

fn open_kind(value string) OpenErrorKind {
	decode_envelope(sealed(value), 1024) or { return (err as OpenError).kind }
	panic('expected a failure')
}

fn test_parser_table() {
	decode_envelope(sealed('otv1:${payload_of(4)}'), 1024)!
	assert open_kind('plain text password') == .malformed
	assert open_kind('') == .malformed
	assert open_kind('OTV1:${payload_of(4)}') == .malformed
	assert open_kind('otv01:${payload_of(4)}') == .malformed
	assert open_kind('otv0:${payload_of(4)}') == .malformed
	assert open_kind('otv1:not base64!') == .malformed
	assert open_kind('otv1:${payload_of(4).trim_right('=')}') == .malformed
	short := base64.encode([]u8{len: envelope_overhead - 1})
	assert open_kind('otv1:${short}') == .malformed
	assert open_kind('otv1:${payload_of(1025)}') == .malformed
	assert open_kind('otv2:${payload_of(4)}') == .newer_format
	assert open_kind('otv12:anything') == .newer_format
}

fn test_encode_then_decode_keeps_every_part() {
	e := Envelope{
		kid:    []u8{len: kid_len, init: 1}
		nonce:  []u8{len: nonce_len, init: 2}
		sealed: []u8{len: 20, init: 3}
	}
	back := decode_envelope(e.encode(), 1024)!
	assert back.kid == e.kid
	assert back.nonce == e.nonce
	assert back.sealed == e.sealed
}

fn test_unknown_versions_round_trip_untouched() {
	for value in ['otv2:AAAA', 'otv99:whatever', 'hand-edited plain text'] {
		assert sealed(value).raw() == value
	}
}

fn test_str_never_shows_the_payload() {
	assert sealed('otv1:c2VjcmV0LXBheWxvYWQ=').str() == 'otv1:<sealed>'
	assert sealed('plain').str() == '<sealed>'
	assert '${sealed('otv1:c2VjcmV0')}' == 'otv1:<sealed>'
}

fn test_associated_data_layout_is_frozen() {
	aad := associated_data('ssh-password', []u8{len: 16, init: 0xab}, [
		'h'.bytes(),
		[u8(0), 22],
		[]u8{},
	])
	mut expected := 'oneterm-vault'.bytes()
	expected << [u8(0), 1, 12]
	expected << 'ssh-password'.bytes()
	expected << []u8{len: 16, init: 0xab}
	expected << [u8(0), 0, 0, 1, `h`, 0, 0, 0, 2, 0, 22, 0, 0, 0, 0]
	assert aad == expected
}

// NIST AES-256-GCM vector through our seal/open internals.
fn test_nist_vector_through_seal_and_open() {
	key := hex.decode('92e11dcdaa866f5ce790fd24501f92509aacf4cb8b1339d50c9c1240935dd08b')!
	nonce := hex.decode('ac93a1a6145299bde902f21a')!
	aad := hex.decode('1e0889016f67601c8ebea4943bc23ad6')!
	plaintext := hex.decode('2d71bcfa914e4ac045b2aa60955fad24')!
	value := seal_bytes(key, []u8{len: 16, init: 0x5a}, nonce, aad, plaintext)!
	e := decode_envelope(value, 1024)!
	assert e.kid == []u8{len: kid_len, init: 0x5a}
	assert e.nonce == nonce
	assert e.sealed.hex() == '8995ae2e6df3dbf96fac7b7137bae67feca5aa77d51d4a0a14d9c51e1da474ab'
	assert open_bytes(key, e, aad)! == plaintext
}

// The whole layout for an SSH password, frozen against the Rust output.
fn test_ssh_password_envelope_layout_is_frozen() {
	v := fixed_vault([]u8{len: 32, init: 0x11}, []u8{len: 16, init: 0x22}, kdf_default)
	context := ctx
	aad := context.associated_data(.ssh_password, v.header.vault_id)
	value := seal_bytes(v.data_key, []u8{len: 16, init: 0x22}, []u8{len: nonce_len, init: 0x33},
		aad, 'hunter2'.bytes())!
	assert value.raw() == 'otv1:IiIiIiIiIiIzMzMzMzMzMzMzMzPs9jwyr+BHOpGnRuzAuJsLqjg/AzEKSw=='
	assert v.open(.ssh_password, context, value)! == 'hunter2'
}

fn test_nonces_never_repeat_and_an_empty_secret_round_trips() {
	v := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, kdf_default)
	first := v.seal(.ssh_password, ctx, '')!
	second := v.seal(.ssh_password, ctx, '')!
	assert first.raw() != second.raw()
	assert v.open(.ssh_password, ctx, first)! == ''
}

fn open_err(v &UnlockedVault, context SealContext, value SealedSecret) OpenErrorKind {
	v.open(.ssh_password, context, value) or { return (err as OpenError).kind }
	panic('expected a failure')
}

fn test_seal_refuses_a_secret_longer_than_its_purpose_allows() {
	v := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, kdf_default)
	v.seal(.ssh_password, ctx, 'x'.repeat(1025)) or {
		assert (err as VaultError).kind == .secret_too_long
		return
	}
	assert false
}

fn test_a_sealed_password_opens_only_for_its_own_address_and_vault() {
	v := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, kdf_default)
	value := v.seal(.ssh_password, ctx, 'pw')!
	assert v.open(.ssh_password, ctx, value)! == 'pw'
	assert open_err(v, SealContext{ ...ctx, host: 'other' }, value) == .rejected
	assert open_err(v, SealContext{ ...ctx, port: 23 }, value) == .rejected
	assert open_err(v, SealContext{ ...ctx, username: 'x' }, value) == .rejected
	other := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 9}, kdf_default)
	assert open_err(other, ctx, value) == .other_vault
}

fn test_tampered_envelopes_never_open() {
	v := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, kdf_default)
	value := v.seal(.ssh_password, ctx, 'pw')!
	payload := base64.decode(value.raw()[5..])
	for i in [kid_len, kid_len + nonce_len, payload.len - 1] {
		mut bad := payload.clone()
		bad[i] ^= 0x01
		assert open_err(v, ctx, sealed('otv1:${base64.encode(bad)}')) == .rejected
	}
}

fn test_newer_and_plain_text_values_are_reported_never_used() {
	v := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, kdf_default)
	assert open_err(v, ctx, sealed('plain')) == .malformed
	assert open_err(v, ctx, sealed('otv2:AAAA')) == .newer_format
}

fn test_the_wrapped_key_never_opens_as_an_item() {
	mut v := fixed_vault([]u8{len: 32, init: 1}, []u8{len: 16, init: 2}, kdf_default)
	aad := associated_data(key_wrap_purpose, v.header.vault_id, [])
	wrapped := seal_bytes(v.data_key, v.header.vault_id, []u8{len: nonce_len, init: 3}, aad,
		[]u8{len: 32, init: 4})!
	assert open_err(v, ctx, wrapped) == .rejected
}

fn test_str_of_a_vault_and_header_carry_no_key_material() {
	v := fixed_vault([]u8{len: 32, init: 0x11}, []u8{len: 16, init: 0x22}, kdf_default)
	assert !'${v}'.contains('1111')
	h := VaultHeader{
		vault_id:    []u8{len: 16, init: 1}
		salt:        []u8{len: 16, init: 0xab}
		wrapped_key: sealed('otv1:SECRETSECRET')
	}
	shown := '${h}'
	assert !shown.contains('SECRET')
	assert !shown.contains('abab')
}
