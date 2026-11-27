//! Secrets sealed with AES-256-GCM, keys of passwords, hashes and random bytes from the
//! system's crypto (specs/16, "Passwords"): CommonCrypto, in the form the Windows module
//! writes (a fresh random nonce, the ciphertext, the tag), so what one system sealed the
//! other opens.

use std::ffi::c_void;
use std::io::Read;

use zeroize::Zeroizing;

const NONCE: usize = 12;
const TAG: usize = 16;
/// `kCCAlgorithmAES`, `kCCPBKDF2` and `kCCPRFHmacAlgSHA256`, as CommonCrypto names them.
const AES: u32 = 0;
const PBKDF2: u32 = 2;
const HMAC_SHA256: u32 = 3;

unsafe extern "C" {
    fn CC_SHA256(data: *const c_void, len: u32, hash: *mut u8) -> *mut u8;
    fn CCKeyDerivationPBKDF(
        algorithm: u32,
        password: *const c_void,
        password_len: usize,
        salt: *const u8,
        salt_len: usize,
        prf: u32,
        rounds: u32,
        key: *mut u8,
        key_len: usize,
    ) -> i32;
    // A whole message in one call. Declared in the header of what Apple's own code uses
    // (`CommonCryptorSPI.h`), in the system's library since macOS 10.13.
    // ponytail: if a system takes them away, GCM over the public `CCCrypt` (AES in counter
    // mode and the tag's hash written here).
    fn CCCryptorGCMOneshotEncrypt(
        algorithm: u32,
        key: *const c_void,
        key_len: usize,
        nonce: *const c_void,
        nonce_len: usize,
        bound: *const c_void,
        bound_len: usize,
        plain: *const c_void,
        len: usize,
        sealed: *mut c_void,
        tag: *mut c_void,
        tag_len: usize,
    ) -> i32;
    fn CCCryptorGCMOneshotDecrypt(
        algorithm: u32,
        key: *const c_void,
        key_len: usize,
        nonce: *const c_void,
        nonce_len: usize,
        bound: *const c_void,
        bound_len: usize,
        sealed: *const c_void,
        len: usize,
        plain: *mut c_void,
        tag: *const c_void,
        tag_len: usize,
    ) -> i32;
}

fn fill(bytes: &mut [u8]) -> bool {
    std::fs::File::open("/dev/urandom").and_then(|mut random| random.read_exact(bytes)).is_ok()
}

/// `N` random bytes from the OS.
pub fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0u8; N];
    fill(&mut bytes).then_some(bytes)
}

/// A key of 32 random bytes, made in place and wiped when dropped.
pub fn random_key() -> Option<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    fill(&mut *key).then_some(key)
}

/// The SHA-256 of `bytes`, by the OS's crypto.
/// ponytail: the whole of it in memory, and under 4 GB (the length the call takes); hashed
/// piece by piece when what is hashed is larger.
pub fn sha256(bytes: &[u8]) -> Option<[u8; 32]> {
    let (len, mut hash) = (u32::try_from(bytes.len()).ok()?, [0u8; 32]);
    // SAFETY: the bytes are told with their length, and the hash is 32 bytes, as it writes.
    unsafe { CC_SHA256(bytes.as_ptr().cast(), len, hash.as_mut_ptr()) };
    Some(hash)
}

/// The PBKDF2-HMAC-SHA256 key of 32 bytes of a password, a salt and a count of rounds; wiped
/// when dropped.
pub fn derive_key(password: &str, salt: &[u8], rounds: u64) -> Option<Zeroizing<[u8; 32]>> {
    let (rounds, mut key) = (u32::try_from(rounds).ok()?, Zeroizing::new([0u8; 32]));
    // SAFETY: every slice is told with its length; the key is written in full or not at all.
    let status = unsafe {
        CCKeyDerivationPBKDF(
            PBKDF2,
            password.as_ptr().cast(),
            password.len(),
            salt.as_ptr(),
            salt.len(),
            HMAC_SHA256,
            rounds,
            key.as_mut_ptr(),
            key.len(),
        )
    };
    (status == 0).then_some(key)
}

/// `plain` sealed under `key` and bound to `bound`: the nonce, the ciphertext, the tag.
pub fn seal(key: &[u8; 32], plain: &[u8], bound: &[u8]) -> Option<Vec<u8>> {
    let nonce = random_bytes::<NONCE>()?;
    let mut out = vec![0u8; NONCE + plain.len() + TAG];
    let (head, rest) = out.split_at_mut(NONCE);
    head.copy_from_slice(&nonce);
    let (body, tag) = rest.split_at_mut(plain.len());
    // SAFETY: every slice is told with its length; the ciphertext is as long as the plain
    // text and the tag 16 bytes, which is what is written.
    let status = unsafe {
        CCCryptorGCMOneshotEncrypt(
            AES,
            key.as_ptr().cast(),
            key.len(),
            nonce.as_ptr().cast(),
            NONCE,
            bound.as_ptr().cast(),
            bound.len(),
            plain.as_ptr().cast(),
            plain.len(),
            body.as_mut_ptr().cast(),
            tag.as_mut_ptr().cast(),
            TAG,
        )
    };
    (status == 0).then_some(out)
}

/// What `seal` sealed, if `key` and `bound` are the ones and nothing was changed; wiped when
/// dropped.
pub fn open(key: &[u8; 32], sealed: &[u8], bound: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if sealed.len() < NONCE + TAG {
        return None;
    }
    let (nonce, rest) = sealed.split_at(NONCE);
    let (body, tag) = rest.split_at(rest.len() - TAG);
    let mut plain = Zeroizing::new(vec![0u8; body.len()]);
    // SAFETY: as in `seal`; a tag that is not the message's is a status, not a fault.
    let status = unsafe {
        CCCryptorGCMOneshotDecrypt(
            AES,
            key.as_ptr().cast(),
            key.len(),
            nonce.as_ptr().cast(),
            NONCE,
            bound.as_ptr().cast(),
            bound.len(),
            body.as_ptr().cast(),
            body.len(),
            plain.as_mut_ptr().cast(),
            tag.as_ptr().cast(),
            TAG,
        )
    };
    (status == 0).then_some(plain)
}
