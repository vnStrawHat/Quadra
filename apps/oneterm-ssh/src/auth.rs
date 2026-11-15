//! Proving who the user is (specs/19, "M7 in detail"): the agent, the usual key files, the
//! password, by the option `Auth`.

use std::path::Path;
use std::sync::Arc;

use russh::client::{AuthResult, Handle, Handler};
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::AgentClient;
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, load_secret_key};

use crate::prompt::Prompt;
use crate::{Auth, Hop, Options};

const KEY_FILES: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];

pub(crate) async fn authenticate<H: Handler>(
    handle: &mut Handle<H>,
    hop: &Hop,
    options: &Options,
    prompt: &Prompt,
) -> Result<(), String> {
    let user = hop.target.user.as_str();
    // RSA signs with the hash the server names, SHA-512 when it names none.
    let rsa = handle.best_supported_rsa_hash().await.ok().flatten().unwrap_or(Some(HashAlg::Sha512));
    let auth = &hop.auth;
    if matches!(auth, Auth::Auto | Auth::Agent) && agent(handle, user, &options.agent, rsa).await {
        return Ok(());
    }
    let files: Vec<_> = match auth {
        Auth::Auto => {
            KEY_FILES.iter().map(|name| options.keys.join(name)).filter(|it| it.is_file()).collect()
        }
        Auth::Key(path) => vec![path.clone()],
        _ => Vec::new(),
    };
    for file in files {
        if key(handle, user, &file, rsa, prompt).await? {
            return Ok(());
        }
    }
    if matches!(auth, Auth::Auto | Auth::Password) {
        let question = format!("{user}@{}'s password: ", hop.target.host);
        for _ in 0..3 {
            let password = prompt.ask(&question, false).await.ok_or("cancelled")?;
            if handle.authenticate_password(user, password).await.map_err(|e| e.to_string())?.success() {
                return Ok(());
            }
            prompt.say("Permission denied, please try again.\n");
        }
    }
    Err("permission denied".into())
}

/// Each key the agent holds, six at most.
async fn agent<H: Handler>(handle: &mut Handle<H>, user: &str, pipe: &str, rsa: Option<HashAlg>) -> bool {
    let Ok(mut agent) = AgentClient::connect_named_pipe(pipe).await else { return false };
    let Ok(keys) = agent.request_identities().await else { return false };
    for identity in keys.into_iter().take(6) {
        let AgentIdentity::PublicKey { key, .. } = identity else { continue };
        let hash = if key.algorithm().is_rsa() { rsa } else { None };
        if let Ok(AuthResult::Success) = handle.authenticate_publickey_with(user, key, hash, &mut agent).await
        {
            return true;
        }
    }
    false
}

/// A key file, its passphrase asked when it has one.
async fn key<H: Handler>(
    handle: &mut Handle<H>,
    user: &str,
    file: &Path,
    rsa: Option<HashAlg>,
    prompt: &Prompt,
) -> Result<bool, String> {
    let key = match load_secret_key(file, None) {
        Ok(key) => key,
        Err(russh::keys::Error::KeyIsEncrypted) => {
            let question = format!("Enter passphrase for key '{}': ", file.display());
            let passphrase = prompt.ask(&question, false).await.ok_or("cancelled")?;
            match load_secret_key(file, Some(&passphrase)) {
                Ok(key) => key,
                Err(e) => {
                    prompt.say(&format!("Could not open {}: {e}\n", file.display()));
                    return Ok(false);
                }
            }
        }
        Err(e) => {
            prompt.say(&format!("Could not read {}: {e}\n", file.display()));
            return Ok(false);
        }
    };
    let key = PrivateKeyWithHashAlg::new(Arc::new(key), rsa);
    let said = handle.authenticate_publickey(user, key).await.map_err(|e| e.to_string())?;
    Ok(said.success())
}
