//! Sealing, hashing and passwords (specs/16, "Passwords") are the same on every system: what
//! Windows sealed, macOS opens, and a password's hash made on one verifies on the other.
#![cfg(any(windows, target_os = "macos"))]

use quadra_shell::{derive_key, hash_password, open, random_key, seal, sha256, verify_password};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap()).collect()
}

/// Sealed by the crypto of Windows (2026-11-27), under the key 0, 1, .., 31, bound to
/// `me@box`: a nonce, twelve bytes of text, a tag.
const SEALED_ON_WINDOWS: &str =
    "bbe77f6063317c1701e7d10a2b4d2e691ab5099bdedf1513f894439f0dbcb91454be841f9587d414";

#[test]
fn what_windows_sealed_opens_on_every_system() {
    let key: [u8; 32] = std::array::from_fn(|i| i as u8);
    let plain = open(&key, &unhex(SEALED_ON_WINDOWS), b"me@box").expect("the key and what it was bound to");
    assert_eq!(std::str::from_utf8(&plain), Ok("mật khẩu"));
    assert_eq!(open(&key, &unhex(SEALED_ON_WINDOWS), b"me@evil"), None);
}

#[test]
fn sealed_opens_with_its_key_only_and_unchanged() {
    let (key, other) = ([7u8; 32], [8u8; 32]);
    let sealed = seal(&key, b"hunter2", b"me@box").unwrap();
    assert_eq!(sealed.len(), 12 + 7 + 16);
    assert_eq!(open(&key, &sealed, b"me@box").as_deref().map(|it| &it[..]), Some(&b"hunter2"[..]));
    assert_eq!(open(&other, &sealed, b"me@box"), None);
    assert_eq!(open(&key, &sealed, b""), None);
    let mut changed = sealed.clone();
    changed[14] ^= 1;
    assert_eq!(open(&key, &changed, b"me@box"), None);
    assert_ne!(seal(&key, b"hunter2", b"me@box").unwrap(), sealed, "a fresh nonce each time");
    assert_eq!(open(&key, &seal(&key, b"", b"").unwrap(), b"").map(|it| it.len()), Some(0));
    assert_ne!(*random_key().unwrap(), [0; 32]);
}

#[test]
fn a_hash_and_a_password_s_key_are_the_standard_ones() {
    // FIPS 180-2, "abc".
    let hash = sha256(b"abc").unwrap();
    assert_eq!(hex(&hash), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    // RFC 7914, section 11: P = "passwd", S = "salt", c = 1, the first 32 bytes.
    let key = derive_key("passwd", b"salt", 1).unwrap();
    assert_eq!(hex(&*key), "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc");
    let stored = hash_password("correct horse").expect("random bytes");
    assert!(stored.starts_with("pbkdf2-sha256$600000$"));
    assert!(verify_password("correct horse", &stored) && !verify_password("correct horsf", &stored));
}
