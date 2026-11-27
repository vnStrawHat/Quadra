//! The key of a password (specs/16, "Passwords"): PBKDF2 with HMAC-SHA256 from the OS's
//! crypto. The hash a password is kept as is made of it in `crate::secret`.

use zeroize::Zeroizing;

use windows::Win32::Security::Cryptography::{BCRYPT_HMAC_SHA256_ALG_HANDLE, BCryptDeriveKeyPBKDF2};

/// The PBKDF2-HMAC-SHA256 key of 32 bytes of a password, a salt and a count of rounds; wiped
/// when dropped.
pub fn derive_key(password: &str, salt: &[u8], rounds: u64) -> Option<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    // SAFETY: the pseudo handle needs no opening or closing; the slices are told with their
    // lengths.
    unsafe {
        BCryptDeriveKeyPBKDF2(
            BCRYPT_HMAC_SHA256_ALG_HANDLE,
            Some(password.as_bytes()),
            Some(salt),
            rounds,
            &mut *key,
            0,
        )
    }
    .ok()
    .ok()?;
    Some(key)
}
