//! For the tests of the updater and of an app that updates (specs/22, "Tests"): a server on
//! 127.0.0.1 that answers as GitHub does, what GitHub says of a release, and a package as a
//! release carries it. Nothing here asks any other machine.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
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

/// A zip of `files` (a name with `/` between its folders, and the bytes), deflated where that
/// is smaller, as a release's package is.
pub fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let (mut out, mut list) = (Vec::new(), Vec::new());
    let put = |to: &mut Vec<u8>, numbers: &[(u32, usize)]| {
        numbers.iter().for_each(|(value, bytes)| to.extend_from_slice(&value.to_le_bytes()[..*bytes]));
    };
    for (name, plain) in files {
        let deflated = miniz_oxide::deflate::compress_to_vec(plain, 6);
        let (method, packed) = if deflated.len() < plain.len() { (8, &deflated[..]) } else { (0, *plain) };
        let (crc, at) = (crate::zip::crc32(plain), out.len() as u32);
        // Version 2.0, names in UTF-8, the first day a zip can say; then the sizes and the name.
        let said = [(20, 2), (0x0800, 2), (method, 2), (0, 2), (0x21, 2), (crc, 4)];
        let sizes = [(packed.len() as u32, 4), (plain.len() as u32, 4), (name.len() as u32, 2), (0, 2)];
        put(&mut out, &[(0x0403_4B50, 4)]);
        put(&mut out, &said);
        put(&mut out, &sizes);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(packed);
        put(&mut list, &[(0x0201_4B50, 4), (20, 2)]);
        put(&mut list, &said);
        put(&mut list, &sizes);
        put(&mut list, &[(0, 2), (0, 2), (0, 2), (0, 4), (at, 4)]);
        list.extend_from_slice(name.as_bytes());
    }
    let (count, listed, at) = (files.len() as u32, list.len() as u32, out.len() as u32);
    out.extend_from_slice(&list);
    put(&mut out, &[(0x0605_4B50, 4), (0, 2), (0, 2), (count, 2), (count, 2), (listed, 4), (at, 4), (0, 2)]);
    out
}

/// A package as a release carries it: the zip `<app>-<version>-<target>.zip` of a folder of
/// that name holding `files`. Its name and its bytes.
pub fn package(app: &str, version: &str, target: &str, files: &[(&str, &[u8])]) -> (String, Vec<u8>) {
    let name = format!("{app}-{version}-{target}");
    let inside: Vec<(String, &[u8])> =
        files.iter().map(|(file, bytes)| (format!("{name}/{file}"), *bytes)).collect();
    let inside: Vec<(&str, &[u8])> = inside.iter().map(|(file, bytes)| (file.as_str(), *bytes)).collect();
    (format!("{name}.zip"), zip(&inside))
}
