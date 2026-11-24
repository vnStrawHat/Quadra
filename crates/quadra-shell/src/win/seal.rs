//! Secrets sealed with AES-256-GCM from the OS's crypto (specs/16, "Passwords"): a fresh
//! random nonce, the ciphertext, the tag; what it is bound to (the associated data) is checked
//! by the tag and not kept.

use windows::Win32::Security::Cryptography::{
    BCRYPT_AES_GCM_ALG_HANDLE, BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO,
    BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO_VERSION, BCRYPT_FLAGS, BCRYPT_KEY_HANDLE, BCRYPT_SHA256_ALG_HANDLE,
    BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptDecrypt, BCryptDestroyKey, BCryptEncrypt, BCryptGenRandom,
    BCryptGenerateSymmetricKey, BCryptHash,
};

use zeroize::Zeroizing;

const NONCE: usize = 12;
const TAG: usize = 16;

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
/// ponytail: the whole of it in memory; hashed piece by piece when what is hashed is too large
/// to hold.
pub fn sha256(bytes: &[u8]) -> Option<[u8; 32]> {
    let mut hash = [0u8; 32];
    // SAFETY: the pseudo handle needs no opening, and both slices are told with their lengths.
    unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, bytes, &mut hash) }.is_ok().then_some(hash)
}

fn fill(bytes: &mut [u8]) -> bool {
    // SAFETY: the buffer is borrowed for the call, which is told its length.
    unsafe { BCryptGenRandom(None, bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) }.is_ok()
}

/// `plain` sealed under `key` and bound to `bound`: the nonce, the ciphertext, the tag.
pub fn seal(key: &[u8; 32], plain: &[u8], bound: &[u8]) -> Option<Vec<u8>> {
    let mut nonce = random_bytes::<NONCE>()?;
    let mut out = vec![0u8; NONCE + plain.len() + TAG];
    let (head, rest) = out.split_at_mut(NONCE);
    head.copy_from_slice(&nonce);
    let (body, tag) = rest.split_at_mut(plain.len());
    crypt(key, &mut nonce, tag, bound, |hkey, info| {
        let mut done = 0u32;
        // SAFETY: the key is live, the info points at locals that outlive the call, and the
        // slices are told with their lengths.
        unsafe { BCryptEncrypt(hkey, Some(plain), Some(info), None, Some(body), &mut done, BCRYPT_FLAGS(0)) }
            .is_ok()
    })
    .then_some(out)
}

/// What `seal` sealed, if `key` and `bound` are the ones and nothing was changed; wiped when
/// dropped.
pub fn open(key: &[u8; 32], sealed: &[u8], bound: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if sealed.len() < NONCE + TAG {
        return None;
    }
    let (head, rest) = sealed.split_at(NONCE);
    let (body, tag) = rest.split_at(rest.len() - TAG);
    let (mut nonce, mut tag) = (<[u8; NONCE]>::try_from(head).ok()?, <[u8; TAG]>::try_from(tag).ok()?);
    let mut plain = Zeroizing::new(vec![0u8; body.len()]);
    crypt(key, &mut nonce, &mut tag, bound, |hkey, info| {
        let mut done = 0u32;
        // SAFETY: as in `seal`; a wrong tag is an error status, not a fault.
        unsafe {
            BCryptDecrypt(hkey, Some(body), Some(info), None, Some(&mut plain), &mut done, BCRYPT_FLAGS(0))
        }
        .is_ok()
    })
    .then_some(plain)
}

/// Runs `f` with a key object of `key` and the GCM info of the nonce, the tag and the
/// associated data, then destroys the key object.
fn crypt(
    key: &[u8; 32],
    nonce: &mut [u8; NONCE],
    tag: &mut [u8],
    bound: &[u8],
    f: impl FnOnce(BCRYPT_KEY_HANDLE, *const core::ffi::c_void) -> bool,
) -> bool {
    let mut hkey = BCRYPT_KEY_HANDLE::default();
    // SAFETY: the pseudo handle needs no opening; the key object is the OS's to allocate.
    if unsafe { BCryptGenerateSymmetricKey(BCRYPT_AES_GCM_ALG_HANDLE, &mut hkey, None, key, 0) }.is_err() {
        return false;
    }
    let info = BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO {
        cbSize: size_of::<BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO>() as u32,
        dwInfoVersion: BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO_VERSION,
        pbNonce: nonce.as_mut_ptr(),
        cbNonce: NONCE as u32,
        pbTag: tag.as_mut_ptr(),
        cbTag: tag.len() as u32,
        // Read only: the OS takes a mutable pointer for both ways.
        pbAuthData: bound.as_ptr().cast_mut(),
        cbAuthData: bound.len() as u32,
        ..Default::default()
    };
    let done = f(hkey, (&raw const info).cast());
    // SAFETY: the key object was made above and is not used after.
    let _ = unsafe { BCryptDestroyKey(hkey) };
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_opens_with_its_key_only_and_unchanged() {
        let (key, other) = ([7u8; 32], [8u8; 32]);
        let sealed = seal(&key, b"hunter2", b"me@box").unwrap();
        assert_eq!(sealed.len(), 12 + 7 + 16);
        assert_eq!(open(&key, &sealed, b"me@box").as_deref().map(|it| &it[..]), Some(&b"hunter2"[..]));
        assert_eq!(open(&other, &sealed, b"me@box"), None);
        assert_eq!(open(&key, &sealed, b"me@evil"), None, "bound to what it was sealed for");
        assert_eq!(open(&key, &sealed, b""), None);
        let mut changed = sealed.clone();
        changed[14] ^= 1;
        assert_eq!(open(&key, &changed, b"me@box"), None);
        assert_ne!(seal(&key, b"hunter2", b"me@box").unwrap(), sealed, "a fresh nonce each time");
        assert_eq!(open(&key, &seal(&key, b"", b"").unwrap(), b"").map(|it| it.len()), Some(0));
        assert_ne!(*random_key().unwrap(), [0; 32]);
    }
}
