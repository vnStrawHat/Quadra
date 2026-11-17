//! Secrets sealed with AES-256-GCM from the OS's crypto (specs/16, "Passwords"): a fresh
//! random nonce, the ciphertext, the tag.

use windows::Win32::Security::Cryptography::{
    BCRYPT_AES_GCM_ALG_HANDLE, BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO,
    BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO_VERSION, BCRYPT_FLAGS, BCRYPT_KEY_HANDLE,
    BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptDecrypt, BCryptDestroyKey, BCryptEncrypt, BCryptGenRandom,
    BCryptGenerateSymmetricKey,
};

const NONCE: usize = 12;
const TAG: usize = 16;

/// `N` random bytes from the OS.
pub fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0u8; N];
    // SAFETY: the buffer is a local whose length the call is told.
    unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) }.ok().ok()?;
    Some(bytes)
}

/// `plain` sealed under `key`: the nonce, the ciphertext, the tag.
pub fn seal(key: &[u8; 32], plain: &[u8]) -> Option<Vec<u8>> {
    let mut nonce = random_bytes::<NONCE>()?;
    let mut out = vec![0u8; NONCE + plain.len() + TAG];
    let (head, rest) = out.split_at_mut(NONCE);
    head.copy_from_slice(&nonce);
    let (body, tag) = rest.split_at_mut(plain.len());
    crypt(key, &mut nonce, tag, |hkey, info| {
        let mut done = 0u32;
        // SAFETY: the key is live, the info points at locals that outlive the call, and the
        // slices are told with their lengths.
        unsafe { BCryptEncrypt(hkey, Some(plain), Some(info), None, Some(body), &mut done, BCRYPT_FLAGS(0)) }
            .is_ok()
    })
    .then_some(out)
}

/// What `seal` sealed, if `key` is the one and nothing was changed.
pub fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < NONCE + TAG {
        return None;
    }
    let (head, rest) = sealed.split_at(NONCE);
    let (body, tag) = rest.split_at(rest.len() - TAG);
    let (mut nonce, mut tag) = (<[u8; NONCE]>::try_from(head).ok()?, <[u8; TAG]>::try_from(tag).ok()?);
    let mut plain = vec![0u8; body.len()];
    crypt(key, &mut nonce, &mut tag, |hkey, info| {
        let mut done = 0u32;
        // SAFETY: as in `seal`; a wrong tag is an error status, not a fault.
        unsafe {
            BCryptDecrypt(hkey, Some(body), Some(info), None, Some(&mut plain), &mut done, BCRYPT_FLAGS(0))
        }
        .is_ok()
    })
    .then_some(plain)
}

/// Runs `f` with a key object of `key` and the GCM info of the nonce and the tag, then
/// destroys the key object.
fn crypt(
    key: &[u8; 32],
    nonce: &mut [u8; NONCE],
    tag: &mut [u8],
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
        let sealed = seal(&key, b"hunter2").unwrap();
        assert_eq!(sealed.len(), 12 + 7 + 16);
        assert_eq!(open(&key, &sealed).as_deref(), Some(&b"hunter2"[..]));
        assert_eq!(open(&other, &sealed), None);
        let mut changed = sealed.clone();
        changed[14] ^= 1;
        assert_eq!(open(&key, &changed), None);
        assert_ne!(seal(&key, b"hunter2").unwrap(), sealed, "a fresh nonce each time");
        assert_eq!(open(&key, &seal(&key, b"").unwrap()).as_deref(), Some(&b""[..]));
    }
}
