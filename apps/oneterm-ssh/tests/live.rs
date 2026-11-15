//! The engine against a `russh` server in this process on 127.0.0.1 (specs/19, "M7 in
//! detail"; never another host): host keys unknown, known and changed, the password and a
//! key file, typing, a resize, the exit status, a connection dropped. Every file is under
//! `.tmp/ssh-tests`; the user's agent and keys are never tried.

mod support;

use std::time::{Duration, Instant};

use oneterm_ssh::Auth;
use russh::keys::known_hosts::learn_known_hosts_path;
use russh::keys::ssh_key::LineEnding;
use support::{connect, folder, key, options, serve};

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
