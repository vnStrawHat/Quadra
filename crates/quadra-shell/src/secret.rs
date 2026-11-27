//! Passwords kept as a salted hash (specs/16, "Passwords"): PBKDF2 with HMAC-SHA256 from
//! the OS's crypto (`derive_key`, of the system's module), written the same on every system.

use crate::{derive_key, random_bytes};

const KIND: &str = "pbkdf2-sha256";
const ROUNDS: u64 = 600_000;

/// The password as `pbkdf2-sha256$<rounds>$<salt hex>$<key hex>`, with a new random salt.
pub fn hash_password(password: &str) -> Option<String> {
    let salt = random_bytes::<16>()?;
    let key = derive_key(password, &salt, ROUNDS)?;
    Some(format!("{KIND}${ROUNDS}${}${}", hex(&salt), hex(&*key)))
}

/// Whether a password is the one `stored` was made from.
pub fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    let [KIND, rounds, salt, key] = parts[..] else { return false };
    let (Ok(rounds), Some(salt), Some(key)) = (rounds.parse::<u64>(), unhex(salt), unhex(key)) else {
        return false;
    };
    let Some(found) = derive_key(password, &salt, rounds).filter(|_| key.len() == 32) else { return false };
    // Every byte is looked at, wherever the first difference is.
    found.iter().zip(&key).fold(0u8, |differ, (a, b)| differ | (a ^ b)) == 0
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len()).step_by(2).map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_verifies_against_its_hash_and_no_other() {
        let stored = hash_password("correct horse").expect("random bytes");
        assert!(stored.starts_with("pbkdf2-sha256$600000$"));
        assert!(verify_password("correct horse", &stored));
        assert!(!verify_password("correct horsf", &stored));
        assert_ne!(hash_password("correct horse").unwrap(), stored, "each hash has its own salt");
        assert!(!verify_password("", "pbkdf2-sha256$1$zz$00"));
        assert!(!verify_password("", ""));
    }

    #[test]
    fn the_key_is_pbkdf2_with_hmac_sha256() {
        // RFC 7914, section 11: P = "passwd", S = "salt", c = 1, the first 32 bytes.
        let key = derive_key("passwd", b"salt", 1).unwrap();
        assert_eq!(hex(&*key), "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc");
    }
}
