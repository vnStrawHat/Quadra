//! For the tests of the updater and of an app that updates (specs/22, "Tests"): a server on
//! 127.0.0.1 that answers as GitHub does, what GitHub says of a release, and a package as a
//! release carries it. Nothing here asks any other machine.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread::JoinHandle;

/// Serves each answer (a status as `200 OK`, and a body) to one connection, in order, and
/// gives the server's address and, once joined, what each request said. `answers` is told
/// the address, for answers that name it.
pub fn serve(
    answers: impl FnOnce(&str) -> Vec<(&'static str, Vec<u8>)>,
) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port on this machine");
    let at = format!("http://{}", listener.local_addr().expect("its address"));
    let answers = answers(&at);
    let served = std::thread::spawn(move || {
        let mut asked = Vec::new();
        for (status, body) in answers {
            let Ok((mut stream, _)) = listener.accept() else { break };
            let (mut said, mut reader) = (String::new(), BufReader::new(&stream));
            while reader.read_line(&mut said).is_ok_and(|read| read > 2) {}
            asked.push(said);
            let head =
                format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&body));
        }
        asked
    });
    (at, served)
}

/// What GitHub says of a release: its tag, and its assets (a name, and its bytes when GitHub
/// has a digest for it), fetched from under `at`.
pub fn release(tag: &str, assets: &[(&str, Option<&[u8]>)], at: &str) -> Vec<u8> {
    let assets: Vec<String> = assets
        .iter()
        .map(|(name, bytes)| {
            let digest = bytes.map_or("null".to_string(), |it| format!("\"sha256:{}\"", digest(it)));
            let size = bytes.map_or(0, <[u8]>::len);
            format!(
                r#"{{"name":"{name}","size":{size},"digest":{digest},"browser_download_url":"{at}/get/{name}"}}"#
            )
        })
        .collect();
    let page = format!("https://github.com/owner/app/releases/tag/{tag}");
    format!(r#"{{"tag_name":"{tag}","html_url":"{page}","draft":false,"assets":[{}]}}"#, assets.join(","))
        .into()
}

/// The SHA-256 of bytes, in hex, as a digest is written.
pub fn digest(bytes: &[u8]) -> String {
    crate::sha256(bytes).unwrap_or_default().iter().map(|b| format!("{b:02x}")).collect()
}

/// A package as a release carries it: the zip `<app>-<version>-<target>.zip` of a folder of
/// that name holding `files`, made under `root`. Its name and its bytes.
pub fn package(
    root: &Path,
    app: &str,
    version: &str,
    target: &str,
    files: &[(&str, &[u8])],
) -> (String, Vec<u8>) {
    let name = format!("{app}-{version}-{target}");
    for (file, bytes) in files {
        let path = root.join("made").join(&name).join(file);
        std::fs::create_dir_all(path.parent().expect("a file in a folder")).expect("the folder");
        std::fs::write(path, bytes).expect("the file");
    }
    let zip = root.join(format!("{name}.zip"));
    let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let mut tar = std::process::Command::new(Path::new(&system).join("System32").join("tar.exe"));
    tar.arg("-a").arg("-c").arg("-f").arg(&zip).arg("-C").arg(root.join("made")).arg(&name);
    let made = tar.output().expect("the OS's tar");
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    (format!("{name}.zip"), std::fs::read(zip).expect("the zip"))
}
