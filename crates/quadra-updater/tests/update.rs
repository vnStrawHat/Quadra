//! An update from the check to the files in place (specs/22), against a server of the test on
//! 127.0.0.1 that answers as GitHub does, in folders of the test under `.tmp`.
#![cfg(windows)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use quadra_updater::{Error, Release, Updater, swap, swept};

const TARGET: &str = "x86_64-pc-windows-msvc";

fn folder(name: &str) -> PathBuf {
    let folder = std::path::absolute(".tmp/update").unwrap().join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// Serves each answer (a status and a body) to one connection, and gives what was asked.
fn serve(answers: Vec<(&'static str, Vec<u8>)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let at = format!("http://{}", listener.local_addr().unwrap());
    let served = std::thread::spawn(move || {
        let mut asked = Vec::new();
        for (status, body) in answers {
            let (mut stream, _) = listener.accept().unwrap();
            let mut said = String::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            while reader.read_line(&mut said).unwrap() > 2 {}
            asked.push(said);
            let head =
                format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&body));
        }
        asked
    });
    (at, served)
}

fn updater(api: &str, root: &Path) -> Updater {
    let mut updater = Updater::new("owner/app", "app", "0.7.0", &root.join("cache"));
    (updater.api, updater.home) = (api.into(), root.join("home"));
    assert_eq!(updater.target, TARGET);
    updater
}

/// What GitHub says of a release: its tag and its assets (a name, a digest when it has one).
fn release(tag: &str, assets: &[(&str, Option<&str>)]) -> Vec<u8> {
    let at = "https://github.com/owner/app/releases/download";
    let assets: Vec<String> = assets
        .iter()
        .map(|(name, digest)| {
            let digest = digest.map_or("null".to_string(), |it| format!("\"sha256:{it}\""));
            format!(
                r#"{{"name":"{name}","size":3,"digest":{digest},"browser_download_url":"{at}/get/{name}"}}"#
            )
        })
        .collect();
    let page = "https://github.com/owner/app/releases/tag/".to_string() + tag;
    format!(r#"{{"tag_name":"{tag}","html_url":"{page}","draft":false,"assets":[{}]}}"#, assets.join(","))
        .into()
}

#[test]
fn a_check_gives_a_newer_release_with_this_build_s_package_and_its_digest() {
    let root = folder("check");
    let (name, digest) = (format!("app-0.8.0-{TARGET}.zip"), "AB".repeat(32));
    let other = "app-0.8.0-aarch64-unknown-linux-gnu.tar.gz";
    let (at, served) = serve(vec![
        ("200 OK", release("v0.8.0", &[(other, Some(&digest)), (&name, Some(&digest))])),
        ("200 OK", release("v0.7.0", &[(&name, Some(&digest))])),
        ("200 OK", release("v0.6.9", &[(&name, Some(&digest))])),
        ("200 OK", release("v0.8.0-rc.1", &[(&name, Some(&digest))])),
        ("200 OK", release("v0.8.0", &[(other, Some(&digest))])),
        ("200 OK", release("v0.8.0", &[(&name, None)])),
        ("404 Not Found", b"{}".to_vec()),
        ("403 Forbidden", b"{}".to_vec()),
        ("500 Oops", b"down".to_vec()),
        ("200 OK", b"<html>a proxy's page</html>".to_vec()),
    ]);
    let u = updater(&at, &root);
    let found = u.check().unwrap().expect("a newer release");
    assert_eq!((found.version.as_str(), found.name.as_str(), found.size), ("0.8.0", name.as_str(), 3));
    assert_eq!(found.sha256, "ab".repeat(32), "the digest in lower case");
    assert_eq!(found.notes, "https://github.com/owner/app/releases/tag/v0.8.0");
    assert_eq!(found.url, format!("https://github.com/owner/app/releases/download/get/{name}"));
    assert_eq!(u.check(), Ok(None), "the same version");
    assert_eq!(u.check(), Ok(None), "an older one");
    assert_eq!(u.check(), Ok(None), "a pre-release tag");
    assert_eq!(u.check(), Err(Error::NoPackage("0.8.0".into())), "no package for this build");
    assert_eq!(u.check(), Err(Error::NoPackage("0.8.0".into())), "a package without a digest");
    assert_eq!(u.check(), Ok(None), "no release yet");
    assert_eq!(u.check(), Err(Error::Limited));
    assert!(matches!(u.check(), Err(Error::Network(_))), "a server's error");
    assert!(matches!(u.check(), Err(Error::Network(_))), "what is no JSON");
    let asked = served.join().unwrap();
    assert!(asked[0].starts_with("GET /repos/owner/app/releases/latest HTTP/1.1\r\n"), "{}", asked[0]);
    assert!(asked[0].contains("User-Agent: app/0.7.0\r\n"), "{}", asked[0]);
    // An API that is not https, on another machine, is not asked at all.
    let mut far = updater("http://example.invalid", &root);
    assert!(matches!(far.check(), Err(Error::Network(why)) if why.starts_with("not asked")));
    far.api = at;
    assert!(far.check().is_err(), "the server is gone");
}

/// A package as a release carries it: a zip of the folder named as the package is.
fn package(root: &Path, version: &str, files: &[(&str, &[u8])]) -> (String, Vec<u8>) {
    let name = format!("app-{version}-{TARGET}");
    for (file, bytes) in files {
        let path = root.join("made").join(&name).join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let zip = root.join(format!("{name}.zip"));
    let tar = Path::new(&std::env::var_os("SystemRoot").unwrap()).join("System32").join("tar.exe");
    let mut made = std::process::Command::new(tar);
    made.arg("-a").arg("-c").arg("-f").arg(&zip).arg("-C").arg(root.join("made")).arg(&name);
    let made = made.output().unwrap();
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    (format!("{name}.zip"), std::fs::read(zip).unwrap())
}

fn hex(bytes: &[u8]) -> String {
    quadra_shell::sha256(bytes).unwrap().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn a_package_is_fetched_checked_against_its_digest_and_unpacked() {
    let root = folder("fetch");
    let (name, zip) = package(&root, "0.8.0", &[("app.exe", b"new app"), ("x64/host.exe", b"new host")]);
    let (at, served) = serve(vec![("200 OK", zip.clone()), ("200 OK", zip.clone()), ("200 OK", zip.clone())]);
    let u = updater(&at, &root);
    let size = zip.len() as u64;
    let of = |sha256: String| Release {
        version: "0.8.0".into(),
        notes: String::new(),
        name: name.clone(),
        url: format!("{at}/get/{name}"),
        size,
        sha256,
    };
    let mut told = Vec::new();
    let found = u.fetch(&of(hex(&zip)), &mut |got, all| {
        told.push((got, all));
        true
    });
    let found = found.unwrap();
    assert_eq!(found, root.join("cache").join("app-0.8.0").join(name.trim_end_matches(".zip")));
    assert_eq!(std::fs::read(found.join("app.exe")).unwrap(), b"new app");
    assert_eq!(std::fs::read(found.join("x64/host.exe")).unwrap(), b"new host");
    assert_eq!(told.last(), Some(&(size, size)), "{told:?}");
    // What came is not what was published: nothing of it is unpacked, and what a fetch before
    // unpacked is not taken for it.
    assert_eq!(u.fetch(&of("0".repeat(64)), &mut |_, _| true), Err(Error::Mismatch));
    assert_eq!(u.fetch(&of(hex(&zip)), &mut |_, _| false), Err(Error::Cancelled));
    let asked = served.join().unwrap();
    assert!(asked[0].starts_with(&format!("GET /get/{name} ")), "{}", asked[0]);
}

#[test]
fn a_running_program_s_files_are_replaced_and_the_next_start_removes_the_old() {
    let root = folder("swap");
    let (home, new) = (root.join("home"), root.join("new"));
    for folder in [home.join("x64"), new.join("x64"), new.join("more")] {
        std::fs::create_dir_all(folder).unwrap();
    }
    // The app that runs is a program of the OS under the app's name; it waits for a line.
    let system = Path::new(&std::env::var_os("SystemRoot").unwrap()).join("System32");
    std::fs::copy(system.join("sort.exe"), home.join("app.exe")).unwrap();
    std::fs::write(home.join("x64/host.exe"), b"old host").unwrap();
    std::fs::write(home.join("kept.txt"), b"the user's").unwrap();
    let mut running = std::process::Command::new(home.join("app.exe"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    for (file, bytes) in [("app.exe", "new app"), ("x64/host.exe", "new host"), ("more/notes.txt", "notes")] {
        std::fs::write(new.join(file), bytes).unwrap();
    }
    swap(&new, &home).unwrap();
    assert_eq!(std::fs::read(home.join("app.exe")).unwrap(), b"new app");
    assert_eq!(std::fs::read(home.join("x64/host.exe")).unwrap(), b"new host");
    assert_eq!(std::fs::read(home.join("more/notes.txt")).unwrap(), b"notes");
    assert_eq!(std::fs::read(home.join("kept.txt")).unwrap(), b"the user's", "what no package has stays");
    assert!(running.try_wait().unwrap().is_none(), "the app goes on running");
    assert_eq!(swept(&home), 1, "the running program's file cannot go yet");
    assert!(home.join("app.exe.quadra-old").exists() && !home.join("x64/host.exe.quadra-old").exists());
    // A second update before any restart: the file moved aside is in the way, and nothing changes.
    std::fs::write(new.join("app.exe"), b"newer app").unwrap();
    assert!(swap(&new, &home).is_err());
    assert_eq!(std::fs::read(home.join("app.exe")).unwrap(), b"new app");
    assert_eq!(std::fs::read(home.join("x64/host.exe")).unwrap(), b"new host");
    running.kill().unwrap();
    running.wait().unwrap();
    assert_eq!(swept(&home), 0, "once it ended, nothing is left");
    assert!(!home.join("app.exe.quadra-old").exists());

    // A file that cannot be put takes the others back: a file is where its folder should be.
    std::fs::remove_dir_all(home.join("more")).unwrap();
    std::fs::write(home.join("more"), b"in the way").unwrap();
    std::fs::write(new.join("app.exe"), b"newest app").unwrap();
    assert!(swap(&new, &home).is_err());
    assert_eq!(std::fs::read(home.join("app.exe")).unwrap(), b"new app", "taken back");
    assert_eq!(swept(&home), 0, "and nothing left aside");
}
