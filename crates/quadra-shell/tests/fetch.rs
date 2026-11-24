//! Fetching and hashing by the OS (specs/16, "Fetching"): a GET against a server of the test
//! on 127.0.0.1, which sees the headers asked with; SHA-256 by its known answers.
#![cfg(windows)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use quadra_shell::{http_get, sha256};

/// Serves `answers` (a status line's end and a body), one a connection, and gives what each
/// request said.
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

#[test]
fn a_get_gives_the_status_and_the_body_piece_by_piece() {
    let big: Vec<u8> = (0..300_000u32).map(|n| (n % 251) as u8).collect();
    let (at, served) = serve(vec![("200 OK", big.clone()), ("404 Not Found", b"none".to_vec())]);
    let mut got = Vec::new();
    let headers = [("User-Agent", "fetch-test/1.0"), ("Accept", "application/json")];
    let status = http_get(&format!("{at}/a/file?x=1"), &headers, &mut |piece| {
        got.extend_from_slice(piece);
        true
    });
    assert_eq!(status, Ok(200));
    assert!(got == big, "{} bytes of {}", got.len(), big.len());
    let mut none = Vec::new();
    let status = http_get(&format!("{at}/missing"), &[], &mut |piece| {
        none.extend_from_slice(piece);
        true
    });
    assert_eq!((status, none.as_slice()), (Ok(404), &b"none"[..]));
    let asked = served.join().unwrap();
    assert!(asked[0].starts_with("GET /a/file?x=1 HTTP/1.1\r\n"), "{}", asked[0]);
    assert!(
        asked[0].contains("User-Agent: fetch-test/1.0\r\n")
            && asked[0].contains("Accept: application/json\r\n")
    );
}

#[test]
fn a_body_refused_stops_the_get_and_no_server_is_an_error() {
    let (at, served) = serve(vec![("200 OK", vec![7; 500_000])]);
    let mut pieces = 0;
    let stopped = http_get(&at, &[], &mut |_| {
        pieces += 1;
        false
    });
    assert_eq!((stopped, pieces), (Err("stopped".to_string()), 1));
    served.join().unwrap();
    // The port of a listener that is gone: nothing answers there.
    let gone = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    assert!(http_get(&format!("http://{gone}/"), &[], &mut |_| true).is_err());
    assert!(http_get("file:///c:/windows/win.ini", &[], &mut |_| true).is_err());
}

#[test]
fn sha_256_by_its_known_answers() {
    let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(sha256(b"").map(hex).as_deref(), Some(empty));
    assert_eq!(sha256(b"abc").map(hex).as_deref(), Some(abc));
}
