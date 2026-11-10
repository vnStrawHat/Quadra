// Windows CNG (bcrypt.dll), loaded at run time: AES in constant time, RSA and the NIST curves
// (specs/23 D2). This file has the loader and AES; cng_keys_windows.c.v the asymmetric keys.
module cng

import sync
import quadra.platform.win

type CngOpen = fn (alg &voidptr, id &u16, impl voidptr, flags u32) int

type CngSetProp = fn (obj voidptr, prop &u16, value voidptr, size u32, flags u32) int

type CngGenSym = fn (alg voidptr, key &voidptr, obj voidptr, size u32, secret voidptr, len u32, flags u32) int

type CngCrypt = fn (key voidptr, input voidptr, n u32, pad voidptr, iv voidptr, ivn u32, out voidptr, outn u32, got &u32, flags u32) int

type CngDestroy = fn (handle voidptr) int

type CngImport = fn (alg voidptr, imp voidptr, blob &u16, key &voidptr, input voidptr, n u32, flags u32) int

type CngExport = fn (key voidptr, exp voidptr, blob &u16, out voidptr, n u32, got &u32, flags u32) int

type CngGenPair = fn (alg voidptr, key &voidptr, bits u32, flags u32) int

type CngFinalize = fn (key voidptr, flags u32) int

type CngSign = fn (key voidptr, pad voidptr, hash voidptr, n u32, sig voidptr, m u32, got &u32, flags u32) int

type CngVerify = fn (key voidptr, pad voidptr, hash voidptr, n u32, sig voidptr, m u32, flags u32) int

type CngAgree = fn (priv voidptr, pub_key voidptr, secret &voidptr, flags u32) int

type CngDerive = fn (secret voidptr, kdf &u16, params voidptr, out voidptr, n u32, got &u32, flags u32) int

// Cng holds the bcrypt.dll entry points and the open providers.
@[heap]
pub struct Cng {
mut:
	mu          &sync.Mutex = sync.new_mutex()
	loaded      bool
	providers   map[string]voidptr
	open        CngOpen     = unsafe { nil }
	set_prop    CngSetProp  = unsafe { nil }
	gen_sym     CngGenSym   = unsafe { nil }
	encrypt     CngCrypt    = unsafe { nil }
	decrypt     CngCrypt    = unsafe { nil }
	destroy_key CngDestroy  = unsafe { nil }
	import_key  CngImport   = unsafe { nil }
	export_key  CngExport   = unsafe { nil }
	gen_pair    CngGenPair  = unsafe { nil }
	finalize    CngFinalize = unsafe { nil }
	sign        CngSign     = unsafe { nil }
	verify      CngVerify   = unsafe { nil }
	agree       CngAgree    = unsafe { nil }
	derive      CngDerive   = unsafe { nil }
	destroy_s   CngDestroy  = unsafe { nil }
}

// the process-wide instance; bcrypt.dll loads on first use, not at start-up.
const cng_instance = &Cng{}

// instance returns the process-wide engine.
pub fn instance() !&Cng {
	mut c := unsafe { cng_instance }
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	if !c.loaded {
		lib := win.load_system_dll('bcrypt.dll')!
		c.open = unsafe { CngOpen(win.dll_proc(lib, 'BCryptOpenAlgorithmProvider')!) }
		c.set_prop = unsafe { CngSetProp(win.dll_proc(lib, 'BCryptSetProperty')!) }
		c.gen_sym = unsafe { CngGenSym(win.dll_proc(lib, 'BCryptGenerateSymmetricKey')!) }
		c.encrypt = unsafe { CngCrypt(win.dll_proc(lib, 'BCryptEncrypt')!) }
		c.decrypt = unsafe { CngCrypt(win.dll_proc(lib, 'BCryptDecrypt')!) }
		c.destroy_key = unsafe { CngDestroy(win.dll_proc(lib, 'BCryptDestroyKey')!) }
		c.import_key = unsafe { CngImport(win.dll_proc(lib, 'BCryptImportKeyPair')!) }
		c.export_key = unsafe { CngExport(win.dll_proc(lib, 'BCryptExportKey')!) }
		c.gen_pair = unsafe { CngGenPair(win.dll_proc(lib, 'BCryptGenerateKeyPair')!) }
		c.finalize = unsafe { CngFinalize(win.dll_proc(lib, 'BCryptFinalizeKeyPair')!) }
		c.sign = unsafe { CngSign(win.dll_proc(lib, 'BCryptSignHash')!) }
		c.verify = unsafe { CngVerify(win.dll_proc(lib, 'BCryptVerifySignature')!) }
		c.agree = unsafe { CngAgree(win.dll_proc(lib, 'BCryptSecretAgreement')!) }
		c.derive = unsafe { CngDerive(win.dll_proc(lib, 'BCryptDeriveKey')!) }
		c.destroy_s = unsafe { CngDestroy(win.dll_proc(lib, 'BCryptDestroySecret')!) }
		c.loaded = true
	}
	return c
}

// provider opens (once) the algorithm `name`, with an AES chaining mode when `mode` is set.
fn (mut c Cng) provider(name string, mode string) !voidptr {
	key := '${name}/${mode}'
	c.mu.@lock()
	defer {
		c.mu.unlock()
	}
	if h := c.providers[key] {
		return h
	}
	mut h := unsafe { nil }
	mut st := c.open(&h, name.to_wide(), unsafe { nil }, 0)
	if st != 0 {
		return error('BCryptOpenAlgorithmProvider(${name}) failed: ${u32(st).hex()}')
	}
	if mode != '' {
		value := mode.to_wide()
		st = c.set_prop(h, 'ChainingMode'.to_wide(), voidptr(value), u32(2 * (mode.len + 1)), 0)
		if st != 0 {
			return error('BCryptSetProperty(${mode}) failed: ${u32(st).hex()}')
		}
	}
	c.providers[key] = h
	return h
}

// destroy releases a key handle.
pub fn (c &Cng) destroy(key voidptr) {
	c.destroy_key(key)
}

// aes_key imports a raw AES key for one chaining mode.
pub fn (mut c Cng) aes_key(mode string, key []u8) !voidptr {
	alg := c.provider('AES', mode)!
	mut h := unsafe { nil }
	st := c.gen_sym(alg, &h, unsafe { nil }, 0, voidptr(key.data), u32(key.len), 0)
	if st != 0 {
		return error('BCryptGenerateSymmetricKey failed: ${u32(st).hex()}')
	}
	return h
}

// aes_ecb encrypts whole blocks in place of dst. Used for the CTR keystream.
fn (mut c Cng) aes_ecb(key voidptr, mut out []u8) ! {
	mut got := u32(0)
	st := c.encrypt(key, out.data, u32(out.len), unsafe { nil }, unsafe { nil }, 0, out.data,
		u32(out.len), &got, 0)
	if st != 0 {
		return error('AES failed: ${u32(st).hex()}')
	}
}

// aes_ctr_xor applies AES-CTR to whole blocks of data and advances the big-endian counter.
pub fn (mut c Cng) aes_ctr_xor(key voidptr, mut iv []u8, mut data []u8) ! {
	if data.len % 16 != 0 {
		return error('AES-CTR needs whole blocks')
	}
	mut stream := []u8{len: data.len}
	for pos := 0; pos < data.len; pos += 16 {
		for i in 0 .. 16 {
			stream[pos + i] = iv[i]
		}
		for j := 15; j >= 0; j-- {
			iv[j]++
			if iv[j] != 0 {
				break
			}
		}
	}
	c.aes_ecb(key, mut stream)!
	for i in 0 .. data.len {
		data[i] ^= stream[i]
	}
}

// aes_cbc_decrypt decrypts whole blocks (private key files).
pub fn (mut c Cng) aes_cbc_decrypt(key []u8, iv []u8, mut data []u8) ! {
	h := c.aes_key('ChainingModeCBC', key)!
	defer {
		c.destroy(h)
	}
	mut ivc := iv.clone()
	mut got := u32(0)
	st := c.decrypt(h, data.data, u32(data.len), unsafe { nil }, ivc.data, u32(ivc.len), data.data,
		u32(data.len), &got, 0)
	if st != 0 {
		return error('AES-CBC failed: ${u32(st).hex()}')
	}
}

// GcmInfo mirrors BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO (x64 layout).
struct GcmInfo {
mut:
	cb_size       u32
	version       u32
	nonce         voidptr
	nonce_len     u32
	auth_data     voidptr
	auth_data_len u32
	tag           voidptr
	tag_len       u32
	mac_ctx       voidptr
	mac_ctx_len   u32
	aad_done      u32
	data_len      u64
	flags         u32
}

// aes_gcm_seal encrypts data in place and writes the 16-byte tag.
pub fn (mut c Cng) aes_gcm_seal(key voidptr, nonce []u8, aad []u8, mut data []u8, mut tag []u8) ! {
	info := GcmInfo{
		cb_size:       u32(sizeof(GcmInfo))
		version:       1
		nonce:         nonce.data
		nonce_len:     u32(nonce.len)
		auth_data:     aad.data
		auth_data_len: u32(aad.len)
		tag:           tag.data
		tag_len:       u32(tag.len)
	}
	mut got := u32(0)
	st := c.encrypt(key, data.data, u32(data.len), voidptr(&info), unsafe { nil }, 0, data.data,
		u32(data.len), &got, 0)
	if st != 0 {
		return error('AES-GCM failed: ${u32(st).hex()}')
	}
}

// aes_gcm_open decrypts data in place after checking the tag.
pub fn (mut c Cng) aes_gcm_open(key voidptr, nonce []u8, aad []u8, mut data []u8, tag []u8) ! {
	info := GcmInfo{
		cb_size:       u32(sizeof(GcmInfo))
		version:       1
		nonce:         nonce.data
		nonce_len:     u32(nonce.len)
		auth_data:     aad.data
		auth_data_len: u32(aad.len)
		tag:           tag.data
		tag_len:       u32(tag.len)
	}
	mut got := u32(0)
	st := c.decrypt(key, data.data, u32(data.len), voidptr(&info), unsafe { nil }, 0, data.data,
		u32(data.len), &got, 0)
	if st != 0 {
		return error('bad packet authentication tag')
	}
}
