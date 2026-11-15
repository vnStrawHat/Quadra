//! A connection from start to end (specs/19, "M7 in detail"): the socket, the host key, the
//! authentication, the shell, and what the shell's channel says until it closes.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use russh::ChannelMsg;
use russh::client::{self, Handler};
use russh::keys::known_hosts::{check_known_hosts_path, learn_known_hosts_path};
use russh::keys::{HashAlg, PublicKeyOrCertificate};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::Options;
use crate::prompt::{Cmd, Out, Prompt};

const CONNECT: Duration = Duration::from_secs(20);

pub(crate) async fn run(
    options: Options,
    input: UnboundedReceiver<Cmd>,
    out: Out,
    exited: Box<dyn FnOnce(i32) + Send>,
) {
    let prompt = Arc::new(Prompt::new(out, input, options.rows, options.cols));
    let code = match shell(&options, prompt.clone()).await {
        Ok(code) => code,
        Err(why) => {
            prompt.say(&format!("\n[connection closed: {why}]\n"));
            255
        }
    };
    exited(code);
}

async fn shell(options: &Options, prompt: Arc<Prompt>) -> Result<i32, String> {
    let target = &options.target;
    let address = (target.host.as_str(), target.port);
    let socket = match tokio::time::timeout(CONNECT, TcpStream::connect(address)).await {
        Ok(socket) => socket.map_err(|e| e.to_string())?,
        Err(_) => return Err("timed out".into()),
    };
    let _ = socket.set_nodelay(true);
    let config = client::Config {
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        ..Default::default()
    };
    let checker = Checker {
        host: target.host.clone(),
        port: target.port,
        file: options.known_hosts.clone(),
        prompt: prompt.clone(),
    };
    let refused = |e: russh::Error| match e {
        russh::Error::UnknownKey => "host key verification failed".to_string(),
        e => e.to_string(),
    };
    let mut handle = client::connect_stream(Arc::new(config), socket, checker).await.map_err(refused)?;
    crate::auth::authenticate(&mut handle, options, &prompt).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    let (rows, cols) = prompt.size();
    let fail = |e: russh::Error| e.to_string();
    channel.request_pty(false, "xterm-256color", cols.into(), rows.into(), 0, 0, &[]).await.map_err(fail)?;
    channel.set_env(false, "COLORTERM", "truecolor").await.map_err(fail)?;
    channel.request_shell(false).await.map_err(fail)?;
    let (mut read, write) = channel.split();
    // What is typed goes to the shell on a task of its own.
    let typing = prompt.clone();
    let writer = tokio::spawn(async move {
        let mut input = typing.input.lock().await;
        while let Some(cmd) = input.recv().await {
            let sent = match cmd {
                Cmd::Input(bytes) => write.data_bytes(bytes).await,
                Cmd::Resize(rows, cols) => write.window_change(cols.into(), rows.into(), 0, 0).await,
                Cmd::Close => break,
            };
            if sent.is_err() {
                break;
            }
        }
        let _ = write.close().await;
    });
    let mut code = None;
    while let Some(message) = read.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => prompt.write(&data),
            ChannelMsg::ExitStatus { exit_status } => code = Some(exit_status as i32),
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    writer.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "").await;
    code.ok_or_else(|| "the connection dropped".to_string())
}

/// Checks the server's key against the known hosts, asking about one it has never seen.
struct Checker {
    host: String,
    port: u16,
    file: PathBuf,
    prompt: Arc<Prompt>,
}

impl Handler for Checker {
    type Error = russh::Error;

    async fn check_server_key(&mut self, key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = key else {
            self.prompt.say("The host gave a certificate; 1Term does not check those yet.\n");
            return Ok(false);
        };
        let shown =
            if self.port == 22 { self.host.clone() } else { format!("[{}]:{}", self.host, self.port) };
        let print = key.fingerprint(HashAlg::Sha256);
        match check_known_hosts_path(&self.host, self.port, key, &self.file) {
            Ok(true) => Ok(true),
            Ok(false) => {
                let mut question = format!(
                    "The authenticity of host '{shown}' can't be established.\n{} key fingerprint is {print}.\nAre you sure you want to continue connecting (yes/no)? ",
                    key.algorithm()
                );
                loop {
                    match self.prompt.ask(&question, true).await.as_deref() {
                        Some("yes") => break,
                        Some("no") | None => return Ok(false),
                        Some(_) => question = "Please type 'yes' or 'no': ".into(),
                    }
                }
                match learn_known_hosts_path(&self.host, self.port, key, &self.file) {
                    Ok(()) => self
                        .prompt
                        .say(&format!("Warning: Permanently added '{shown}' to the list of known hosts.\n")),
                    Err(e) => self
                        .prompt
                        .say(&format!("Warning: could not add the key to {}: {e}\n", self.file.display())),
                }
                Ok(true)
            }
            Err(russh::keys::Error::KeyChanged { line }) => {
                self.prompt.say(&format!(
                    "WARNING: THE HOST KEY OF '{shown}' HAS CHANGED!\nSomeone could be eavesdropping on you right now.\nThe {} key sent is {print}.\nThe key known is on line {line} of {}.\nRemove it there to trust the new one.\n",
                    key.algorithm(),
                    self.file.display()
                ));
                Ok(false)
            }
            Err(e) => {
                self.prompt.say(&format!("Could not read {}: {e}\n", self.file.display()));
                Ok(false)
            }
        }
    }
}
