//! The engine against a `russh` server in this process on 127.0.0.1 (specs/19, "M7 in
//! detail"; never another host): host keys unknown, known and changed, the password and a
//! key file, typing, a resize, the exit status, a connection dropped. Every file is under
//! `.tmp/ssh-tests`; the user's agent and keys are never tried.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oneterm_ssh::{Auth, Options, Ssh, Target};
use russh::keys::known_hosts::learn_known_hosts_path;
use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::private::Ed25519Keypair;
use russh::keys::{PrivateKey, PublicKey};
use russh::server::{self, Auth as Verdict, Msg, Session, VerifiedPublicKey};
use russh::{Channel, ChannelId};

fn key(seed: u8) -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
}

#[derive(Clone)]
struct Shell {
    user_key: PublicKey,
    seen: Arc<Mutex<Vec<String>>>,
}

impl server::Handler for Shell {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Verdict, Self::Error> {
        Ok(if (user, password) == ("tester", "secret") { Verdict::Accept } else { Verdict::reject() })
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: VerifiedPublicKey<'_>,
    ) -> Result<Verdict, Self::Error> {
        Ok(if user == "tester" && *key.public_key == self.user_key {
            Verdict::Accept
        } else {
            Verdict::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        _: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        _: ChannelId,
        term: &str,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &[(russh::Pty, u32)],
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.seen.lock().unwrap().push(format!("pty {term} {cols}x{rows}"));
        Ok(())
    }

    async fn shell_request(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        session.data(channel, b"welcome\r\n".to_vec())?;
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.seen.lock().unwrap().push(format!("resize {cols}x{rows}"));
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        match data {
            b"exit" => {
                session.exit_status_request(channel, 7)?;
                session.close(channel)?;
            }
            b"drop" => return Err(russh::Error::Disconnect),
            _ => session.data(channel, [b"echo:", data].concat())?,
        }
        Ok(())
    }
}

/// A server on a free port with the host key of `seed`; it takes `tester` with the
/// password `secret` or the key of seed 9.
fn serve(seed: u8) -> (u16, Arc<Mutex<Vec<String>>>) {
    let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap()));
    let listener = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = Arc::new(server::Config {
        keys: vec![key(seed)],
        auth_rejection_time: Duration::from_millis(1),
        auth_rejection_time_initial: Some(Duration::ZERO),
        ..Default::default()
    });
    let seen = Arc::new(Mutex::new(Vec::new()));
    let shell = Shell { user_key: key(9).public_key().clone(), seen: seen.clone() };
    rt.spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let (config, shell) = (config.clone(), shell.clone());
            tokio::spawn(async move {
                if let Ok(running) = server::run_stream(config, socket, shell).await {
                    let _ = running.await;
                }
            });
        }
    });
    (port, seen)
}

struct Client {
    ssh: Ssh,
    out: Arc<Mutex<Vec<u8>>>,
    exited: Receiver<i32>,
}

/// A folder of its own under `.tmp/ssh-tests`, emptied.
fn folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/ssh-tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn options(port: u16, dir: &std::path::Path, auth: Auth) -> Options {
    let mut options = Options::new(Target { user: "tester".into(), host: "127.0.0.1".into(), port });
    (options.auth, options.known_hosts, options.keys) = (auth, dir.join("known_hosts"), dir.join("keys"));
    options.agent = r"\\.\pipe\oneterm-tests-no-agent".into();
    options
}

fn connect(options: Options) -> Client {
    let (out, (said, exited)) = (Arc::new(Mutex::new(Vec::new())), channel());
    let into = out.clone();
    let ssh = Ssh::connect(
        options,
        move |bytes| into.lock().unwrap().extend_from_slice(bytes),
        move |code| {
            let _ = said.send(code);
        },
    );
    Client { ssh, out, exited }
}

impl Client {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.out.lock().unwrap()).into_owned()
    }

    fn wait_for(&self, words: &str) {
        let end = Instant::now() + Duration::from_secs(20);
        while !self.text().contains(words) {
            assert!(Instant::now() < end, "never said {words:?}; said:\n{}", self.text());
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn code(&self) -> i32 {
        self.exited.recv_timeout(Duration::from_secs(20)).expect("the connection ended")
    }
}

#[test]
fn an_unknown_host_is_asked_then_the_password_opens_a_shell() {
    let ((port, seen), dir) = (serve(1), folder("unknown"));
    let c = connect(options(port, &dir, Auth::Password));
    c.wait_for("Are you sure you want to continue connecting (yes/no)? ");
    c.ssh.write(b"maybe\r");
    c.wait_for("Please type 'yes' or 'no': ");
    c.ssh.write(b"yes\r");
    c.wait_for("tester@127.0.0.1's password: ");
    c.ssh.write(b"secret\r");
    c.wait_for("welcome");
    assert!(!c.text().contains("secret"), "a password is not echoed");
    c.ssh.write(b"hi");
    c.wait_for("echo:hi");
    let end = Instant::now() + Duration::from_secs(10);
    while c.ssh.latency().is_none() {
        assert!(Instant::now() < end, "no ping came back");
        std::thread::sleep(Duration::from_millis(5));
    }
    c.ssh.resize(30, 100);
    c.ssh.write(b"x");
    c.wait_for("echo:x");
    assert_eq!(*seen.lock().unwrap(), ["pty xterm-256color 80x24", "resize 100x30"]);
    c.ssh.write(b"exit");
    assert_eq!(c.code(), 7);
    let known = std::fs::read_to_string(dir.join("known_hosts")).unwrap();
    assert!(known.trim_start().starts_with(&format!("[127.0.0.1]:{port} ssh-ed25519 ")), "{known}");
}

#[test]
fn a_known_host_goes_on_and_a_key_file_signs_in() {
    let ((port, _), dir) = (serve(2), folder("known"));
    learn_known_hosts_path("127.0.0.1", port, key(2).public_key(), dir.join("known_hosts")).unwrap();
    let file = dir.join("id");
    std::fs::write(&file, key(9).to_openssh(LineEnding::LF).unwrap().as_bytes()).unwrap();
    let c = connect(options(port, &dir, Auth::Key(file)));
    c.wait_for("welcome");
    assert!(!c.text().contains("yes/no"));
    c.ssh.write(b"exit");
    assert_eq!(c.code(), 7);
}

#[test]
fn a_changed_host_key_is_refused_without_asking() {
    let ((port, _), dir) = (serve(3), folder("changed"));
    learn_known_hosts_path("127.0.0.1", port, key(4).public_key(), dir.join("known_hosts")).unwrap();
    let c = connect(options(port, &dir, Auth::Password));
    assert_eq!(c.code(), 255);
    assert!(c.text().contains("HAS CHANGED") && c.text().contains("is on line 2 of"), "{}", c.text());
    assert!(c.text().contains("[connection closed: host key verification failed]"));
    assert!(!c.text().contains("password"));
}

#[test]
fn wrong_passwords_and_a_dropped_connection_end_with_255() {
    let ((port, _), dir) = (serve(5), folder("denied"));
    learn_known_hosts_path("127.0.0.1", port, key(5).public_key(), dir.join("known_hosts")).unwrap();
    let c = connect(options(port, &dir, Auth::Auto));
    for _ in 0..3 {
        c.wait_for("password: ");
        c.out.lock().unwrap().clear();
        c.ssh.write(b"wrong\r");
    }
    assert_eq!(c.code(), 255);
    c.wait_for("[connection closed: permission denied]");

    let c = connect(options(port, &dir, Auth::Password));
    c.wait_for("password: ");
    c.ssh.write(b"secret\r");
    c.wait_for("welcome");
    c.ssh.write(b"drop");
    assert_eq!(c.code(), 255);
    c.wait_for("[connection closed:");
}

#[test]
fn nobody_listening_is_said() {
    let dir = folder("refused");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let c = connect(options(port, &dir, Auth::Password));
    assert_eq!(c.code(), 255);
    c.wait_for("[connection closed:");
}
