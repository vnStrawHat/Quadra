//! Saved passwords (specs/19, "Saved passwords"): a data key sealed under the master
//! password in `ssh.vault`, each host's secret sealed under the data key in `ssh.secrets`.
//! The data key is in memory only, once the master password was typed.

mod unlock;

pub(crate) use unlock::dialog;

use crate::model::Model;
use crate::shells::Remote;

const ROUNDS: u64 = 600_000;

/// A secret in memory, never shown by `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(..)")
    }
}

/// A new data key.
pub(crate) fn new_key() -> Option<[u8; 32]> {
    random_bytes::<32>()
}

/// A new vault of `password`: what is written, and the data key.
fn make(password: &str) -> Option<(String, [u8; 32])> {
    let key = new_key()?;
    Some((reseal(&key, password)?, key))
}

/// The data key sealed under a new salt's key of `password`.
pub(crate) fn reseal(key: &[u8; 32], password: &str) -> Option<String> {
    let salt = random_bytes::<16>()?;
    let sealed = seal(&derive_key(password, &salt, ROUNDS)?, key)?;
    Some(format!("v1${ROUNDS}${}${}", hex(&salt), hex(&sealed)))
}

/// The data key a vault holds, if `password` opens it.
fn unseal(password: &str, vault: &str) -> Option<[u8; 32]> {
    let parts: Vec<&str> = vault.split('$').collect();
    let ["v1", rounds, salt, sealed] = parts[..] else { return None };
    let (rounds, salt, sealed) = (rounds.parse().ok()?, unhex(salt)?, unhex(sealed)?);
    open(&derive_key(password, &salt, rounds)?, &sealed)?.try_into().ok()
}

/// What the master password opens, if it is the one `hash` was made from: the data key, and
/// a new vault to write when there was none (or one it does not open: what it sealed is lost).
/// On the pool: it derives twice.
pub(crate) fn open_with(password: &str, hash: &str, vault: &str) -> Option<([u8; 32], Option<String>)> {
    if !verify_password(password, hash) {
        return None;
    }
    if let Some(key) = unseal(password, vault) {
        return Some((key, None));
    }
    let (vault, key) = make(password)?;
    Some((key, Some(vault)))
}

/// A secret sealed under the data key, as `ssh.secrets` keeps it.
pub(crate) fn seal_secret(key: &[u8; 32], plain: &str) -> Option<String> {
    Some(format!("sv1:{}", hex(&seal(key, plain.as_bytes())?)))
}

fn open_secret(key: &[u8; 32], sealed: &str) -> Option<String> {
    let bytes = open(key, &unhex(sealed.strip_prefix("sv1:")?)?)?;
    String::from_utf8(bytes).ok()
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
        open_secret(&key, &self.sealed_for(remote)?).map(Secret)
    }
}

#[cfg(windows)]
use quadra_shell::{derive_key, open, random_bytes, seal, verify_password};

#[cfg(not(windows))]
fn derive_key(_: &str, _: &[u8], _: u64) -> Option<[u8; 32]> {
    None
}
#[cfg(not(windows))]
fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    None
}
#[cfg(not(windows))]
fn seal(_: &[u8; 32], _: &[u8]) -> Option<Vec<u8>> {
    None
}
#[cfg(not(windows))]
fn open(_: &[u8; 32], _: &[u8]) -> Option<Vec<u8>> {
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
        assert_eq!(open_with("master", &hash, &vault).map(|it| it.0), Some(key), "the same key again");
        let sealed = seal_secret(&key, "hunter2").unwrap();
        assert!(sealed.starts_with("sv1:") && !sealed.contains("hunter2"));
        assert_eq!(open_secret(&key, &sealed).as_deref(), Some("hunter2"));
        assert_eq!(open_secret(&[0; 32], &sealed), None);
        // A new master password seals the same key.
        let moved = reseal(&key, "other").unwrap();
        assert_eq!(unseal("other", &moved), Some(key));
        assert_eq!(unseal("master", &moved), None);
    }
}
