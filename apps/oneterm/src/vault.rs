//! Saved passwords (specs/19, "Saved passwords"): a data key sealed under the master
//! password in `ssh.vault`, each host's secret sealed under the data key in `ssh.secrets`,
//! both as OneTerm writes them (`format.rs`). The data key is in memory only, once the master
//! password was typed; the master password itself is kept nowhere: it is the one that opens
//! the vault.

mod format;
mod unlock;

pub(crate) use format::Header;
pub(crate) use unlock::dialog;

use std::rc::Rc;

use oneterm_ssh::Target;
use quadra::Signal;
use quadra_shell::{Zeroize, Zeroizing};

use crate::model::Model;
use crate::shells::Remote;

/// A key of 32 bytes, wiped when dropped.
type Key = Zeroizing<[u8; 32]>;

/// A vault opened: which one, and its data key.
#[derive(Clone, PartialEq, Eq)]
pub struct Opened {
    id: [u8; 16],
    key: Key,
}

/// The saved passwords' data key in memory, shared; wiped when the last holder drops it.
pub type DataKey = Rc<Opened>;

/// A secret in memory, wiped when dropped, never shown by `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub(crate) fn new(text: String) -> Secret {
        Secret(Zeroizing::new(text))
    }

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

/// The key a master password derives for a vault: Argon2id (version 19) at the vault's cost,
/// its memory wiped after.
fn derive(password: &str, salt: &[u8], cost: [u32; 3]) -> Option<Key> {
    let params = argon2::Params::new(cost[0], cost[1], cost[2], Some(32)).ok()?;
    let mut blocks = Zeroizing::new(Vec::new());
    blocks.try_reserve_exact(params.block_count()).ok()?;
    blocks.resize(params.block_count(), argon2::Block::default());
    let mut key = Zeroizing::new([0u8; 32]);
    argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into_with_memory(password.as_bytes(), salt, key.as_mut(), blocks.as_mut_slice())
        .ok()?;
    Some(key)
}

/// A new vault of `password`: its document, and the vault opened.
pub(crate) fn make(password: &str) -> Option<(String, Opened)> {
    let opened = Opened { id: random_bytes()?, key: random_key()? };
    Some((reseal(&opened, password)?, opened))
}

/// The document of a vault under another master password: the same vault and data key, a new
/// salt, the cost of today.
pub(crate) fn reseal(opened: &Opened, password: &str) -> Option<String> {
    let (salt, cost) = (random_bytes::<16>()?, format::COST);
    let kek = derive(password, &salt, cost)?;
    let sealed = seal(&kek, &*opened.key, &format::bound("key-wrap", &opened.id, &[]))?;
    let wrapped = format::envelope(&opened.id, &sealed);
    Some(Header { id: opened.id, cost, salt: salt.to_vec(), wrapped }.write())
}

/// What `password` opens of a vault's document: nothing when it is not the vault's. On the
/// pool: it derives.
pub(crate) fn unseal(password: &str, vault: &str) -> Option<Opened> {
    let header = Header::parse(vault)?;
    let kek = derive(password, &header.salt, header.cost)?;
    let sealed = format::sealed(&header.wrapped, &header.id)?;
    let plain = open(&kek, &sealed, &format::bound("key-wrap", &header.id, &[]))?;
    let mut key = Zeroizing::new([0u8; 32]);
    (plain.len() == 32).then(|| key.copy_from_slice(&plain))?;
    Some(Opened { id: header.id, key })
}

/// What a host's secret is bound to: its target, so it opens for no other.
pub(crate) fn bound(remote: &Remote) -> Target {
    remote.target.clone()
}

fn bound_to(vault: &Opened, to: &Target) -> Vec<u8> {
    let fields = [to.host.as_bytes(), &to.port.to_be_bytes(), to.user.as_bytes()];
    format::bound("ssh-password", &vault.id, &fields)
}

/// A secret sealed under the data key for a target, as `ssh.secrets` keeps it.
pub(crate) fn seal_secret(vault: &Opened, plain: &str, to: &Target) -> Option<String> {
    let sealed = seal(&vault.key, plain.as_bytes(), &bound_to(vault, to))?;
    Some(format::envelope(&vault.id, &sealed))
}

/// A secret opened, if it was sealed by this vault for `to`.
pub(crate) fn open_secret(vault: &Opened, sealed: &str, to: &Target) -> Option<Secret> {
    let bytes = open(&vault.key, &format::sealed(sealed, &vault.id)?, &bound_to(vault, to))?;
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
use quadra_shell::{open, random_bytes, random_key, seal};

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

#[cfg(all(test, windows))]
pub(crate) mod tests {
    use super::*;

    /// OneTerm's own fixture of a vault and a password sealed under it, made by an
    /// implementation that is neither OneTerm's nor this one: what it pins is the format.
    pub(crate) const ONETERM_S: &str = include_str!("../tests/fixtures/vault.json");
    const SEALED: &str = "otv1:P5wKTosdTCdcXFxcXFxcXFxcXFxbpgEB6ylBEMijZRhddr55R0kOe/uq";

    fn target(text: &str) -> Target {
        Target::parse(text).unwrap()
    }

    #[test]
    fn what_oneterm_sealed_opens_with_its_master_password() {
        assert!(unseal("wrong", ONETERM_S).is_none());
        let vault = unseal("correct horse battery staple", ONETERM_S).expect("the vault opened");
        let opened = open_secret(&vault, SEALED, &target("deploy@10.0.0.1"));
        assert_eq!(opened.as_ref().map(Secret::as_str), Some("s3cret"));
        assert_eq!(open_secret(&vault, SEALED, &target("deploy@10.0.0.2")), None, "another target");
        assert_eq!(open_secret(&vault, SEALED, &target("deploy@10.0.0.1:2222")), None);
    }

    #[test]
    fn a_vault_opens_with_its_password_and_seals_secrets() {
        let (document, vault) = make("master").unwrap();
        assert!(unseal("wrong", &document).is_none() && !document.contains("master"));
        assert!(unseal("master", &document) == Some(vault.clone()), "the same key again");
        let to = target("me@box");
        let sealed = seal_secret(&vault, "hunter2", &to).unwrap();
        assert!(sealed.starts_with("otv1:") && !sealed.contains("hunter2"));
        assert_eq!(open_secret(&vault, &sealed, &to).as_ref().map(Secret::as_str), Some("hunter2"));
        assert_eq!(open_secret(&vault, &sealed, &target("me@evil:2222")), None, "another target");
        let other = make("master").unwrap().1;
        assert_eq!(open_secret(&other, &sealed, &to), None, "another vault");
        // A new master password seals the same vault and key.
        let moved = reseal(&vault, "other").unwrap();
        assert!(unseal("other", &moved) == Some(vault) && unseal("master", &moved).is_none());
    }
}
