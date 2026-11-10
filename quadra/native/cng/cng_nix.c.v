// CNG is Windows-only; other OSes have no engine yet (see specs/13-apps-roadmap.md).
module cng

const no_engine = 'no crypto engine for this OS yet'

pub struct Cng {}

pub fn instance() !&Cng {
	return error(no_engine)
}

pub fn (c &Cng) destroy(key voidptr) {}

pub fn (mut c Cng) aes_key(mode string, key []u8) !voidptr {
	return error(no_engine)
}

pub fn (mut c Cng) aes_ctr_xor(key voidptr, mut iv []u8, mut data []u8) ! {
	return error(no_engine)
}

pub fn (mut c Cng) aes_cbc_decrypt(key []u8, iv []u8, mut data []u8) ! {
	return error(no_engine)
}

pub fn (mut c Cng) aes_gcm_seal(key voidptr, nonce []u8, aad []u8, mut data []u8, mut tag []u8) ! {
	return error(no_engine)
}

pub fn (mut c Cng) aes_gcm_open(key voidptr, nonce []u8, aad []u8, mut data []u8, tag []u8) ! {
	return error(no_engine)
}

pub fn (mut c Cng) rsa_verify(key RsaParts, alg string, digest []u8, sig []u8) ! {
	return error(no_engine)
}

pub fn (mut c Cng) rsa_sign(key RsaParts, alg string, digest []u8) ![]u8 {
	return error(no_engine)
}

pub fn (mut c Cng) ecdsa_verify(curve Curve, point []u8, digest []u8, r []u8, s []u8) ! {
	return error(no_engine)
}

pub fn (mut c Cng) ecdsa_sign(curve Curve, point []u8, d []u8, digest []u8) !([]u8, []u8) {
	return error(no_engine)
}

pub fn (mut c Cng) ecdh_new(curve Curve) !(voidptr, []u8) {
	return error(no_engine)
}

pub fn (mut c Cng) ecdh_finish(curve Curve, local voidptr, peer []u8) ![]u8 {
	return error(no_engine)
}
