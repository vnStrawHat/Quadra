//! Saved passwords (specs/19, "Saved passwords"): a data key sealed under the master
//! password in `ssh.vault`, each host's secret sealed under the data key in `ssh.secrets`.
//! The data key is in memory only, once the master password was typed.

mod unlock;

pub(crate) use unlock::dialog;

use std::rc::Rc;

use quadra::Signal;
use quadra_shell::{Zeroize, Zeroizing};

use crate::model::Model;
use crate::shells::Remote;

const ROUNDS: u64 = 600_000;

/// A key of 32 bytes, wiped when dropped.
type Key = Zeroizing<[u8; 32]>;

/// The saved passwords' data key in memory, shared; wiped when the last holder drops it.
pub type DataKey = Rc<Key>;

/// A secret in memory, wiped when dropped, never shown by `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Empties a field that held a password, wiping what it held.
pub(crate) fn wipe(field: Signal<String>) {
    if !field.with_untracked(String::is_empty) {
        field.update(|it| it.zeroize());
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(..)")
    }
}

/// A new data key.
pub(crate) fn new_key() -> Option<Key> {
    random_key()
}

/// A new vault of `password`: what is written, and the data key.
fn make(password: &str) -> Option<(String, Key)> {
    let key = new_key()?;
    Some((reseal(&key, password)?, key))
}

/// The data key sealed under a new salt's key of `password`.
pub(crate) fn reseal(key: &[u8; 32], password: &str) -> Option<String> {
    let salt = random_bytes::<16>()?;
    let kek = derive_key(password, &salt, ROUNDS)?;
    let sealed = seal(&kek, key, b"")?;
    Some(format!("v1${ROUNDS}${}${}", hex(&salt), hex(&sealed)))
}

/// The data key a vault holds, if `password` opens it.
fn unseal(password: &str, vault: &str) -> Option<Key> {
    let parts: Vec<&str> = vault.split('$').collect();
    let ["v1", rounds, salt, sealed] = parts[..] else { return None };
    let (rounds, salt, sealed) = (rounds.parse().ok()?, unhex(salt)?, unhex(sealed)?);
    let kek = derive_key(password, &salt, rounds)?;
    let plain = open(&kek, &sealed, b"")?;
    let mut key = Zeroizing::new([0u8; 32]);
    (plain.len() == 32).then(|| key.copy_from_slice(&plain))?;
    Some(key)
}

/// What the master password opens, if it is the one `hash` was made from: the data key, and
/// a new vault to write when there was none (or one it does not open: what it sealed is lost).
/// On the pool: it derives twice.
pub(crate) fn open_with(password: &str, hash: &str, vault: &str) -> Option<(Key, Option<String>)> {
    if !verify_password(password, hash) {
        return None;
    }
    if let Some(key) = unseal(password, vault) {
        return Some((key, None));
    }
    let (vault, key) = make(password)?;
    Some((key, Some(vault)))
}

/// What a host's secret is bound to: its target, so it opens for no other.
pub(crate) fn bound(remote: &Remote) -> String {
    remote.target.label()
}

/// A secret sealed under the data key for a target, as `ssh.secrets` keeps it.
pub(crate) fn seal_secret(key: &[u8; 32], plain: &str, bound: &str) -> Option<String> {
    Some(format!("sv2:{}", hex(&seal(key, plain.as_bytes(), bound.as_bytes())?)))
}

/// A secret opened, if it was sealed for `bound` (or in the first form, bound to nothing).
pub(crate) fn open_secret(key: &[u8; 32], sealed: &str, bound: &str) -> Option<Secret> {
    let bytes = match sealed.split_once(':')? {
        ("sv2", hex) => open(key, &unhex(hex)?, bound.as_bytes())?,
        ("sv1", hex) => open(key, &unhex(hex)?, b"")?,
        _ => return None,
    };
    std::str::from_utf8(&bytes).ok().map(|it| Secret(Zeroizing::new(it.to_string())))
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

impl Model {
    /// The sealed secret saved for a host.
    pub(crate) fn sealed(&self, name: &str) -> Option<String> {
        let rows = self.prefs().secrets;
        rows.with_untracked(|rows| {
            rows.iter().find(|it| it.first().map(String::as_str) == Some(name))?.get(1).cloned()
        })
    }

    /// Saves a host's sealed secret, or forgets it.
    pub(crate) fn set_sealed(&self, name: &str, sealed: Option<String>) {
        self.prefs().secrets.update(|rows| {
            rows.retain(|it| it.first().map(String::as_str) != Some(name));
            rows.extend(sealed.map(|it| vec![name.to_string(), it]));
        });
    }

    /// The sealed secret of the saved host a connection goes to.
    fn sealed_for(&self, remote: &Remote) -> Option<String> {
        let host = self.hosts().into_iter().find(|it| it.remote.target == remote.target)?;
        self.sealed(&host.name)
    }

    /// Connects to each remote in a tab of its own, or in a space here: with its saved secret,
    /// after the master password is asked for when one is saved and the data key is not in
    /// memory.
    pub(crate) fn connect_all(&self, remotes: Vec<Remote>, here: bool) {
        let key = self.lock().key.get_untracked();
        let needs = remotes.iter().any(|it| self.sealed_for(it).is_some());
        if needs && key.is_none() && self.lock().can_lock() {
            return self.ui().unlocking.set(Some((remotes, here)));
        }
        for remote in remotes {
            let secret = self.saved_secret(&remote);
            self.connect_with(remote, here, secret);
        }
    }

    /// The saved secret of the host a connection goes to, opened, if the data key is in memory.
    pub(crate) fn saved_secret(&self, remote: &Remote) -> Option<Secret> {
        let key = self.lock().key.get_untracked()?;
        open_secret(&key, &self.sealed_for(remote)?, &bound(remote))
    }
}

#[cfg(windows)]
use quadra_shell::{derive_key, open, random_bytes, random_key, seal, verify_password};

#[cfg(not(windows))]
fn derive_key(_: &str, _: &[u8], _: u64) -> Option<Key> {
    None
}
#[cfg(not(windows))]
fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    None
}
#[cfg(not(windows))]
fn random_key() -> Option<Key> {
    None
}
#[cfg(not(windows))]
fn seal(_: &[u8; 32], _: &[u8], _: &[u8]) -> Option<Vec<u8>> {
    None
}
#[cfg(not(windows))]
fn open(_: &[u8; 32], _: &[u8], _: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    None
}
#[cfg(not(windows))]
fn verify_password(_: &str, _: &str) -> bool {
    false
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_vault_opens_with_its_password_and_seals_secrets() {
        let hash = quadra_shell::hash_password("master").unwrap();
        assert!(open_with("wrong", &hash, "").is_none());
        let (key, Some(vault)) = open_with("master", &hash, "").unwrap() else { panic!("a new vault") };
        assert_eq!(
            open_with("master", &hash, &vault).map(|it| it.0),
            Some(key.clone()),
            "the same key again"
        );
        let sealed = seal_secret(&key, "hunter2", "me@box").unwrap();
        assert!(sealed.starts_with("sv2:") && !sealed.contains("hunter2"));
        assert_eq!(open_secret(&key, &sealed, "me@box").as_ref().map(Secret::as_str), Some("hunter2"));
        assert_eq!(open_secret(&[0; 32], &sealed, "me@box"), None);
        assert_eq!(open_secret(&key, &sealed, "me@evil:2222"), None, "another target");
        // The first form, bound to nothing, still opens.
        let first = format!("sv1:{}", hex(&seal(&key, b"old", b"").unwrap()));
        assert_eq!(open_secret(&key, &first, "any").as_ref().map(Secret::as_str), Some("old"));
        // A new master password seals the same key.
        let moved = reseal(&key, "other").unwrap();
        assert_eq!(unseal("other", &moved), Some(key.clone()));
        assert_eq!(unseal("master", &moved), None);
    }
}
