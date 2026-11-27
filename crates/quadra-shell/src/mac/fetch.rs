//! A GET over the system's HTTP (specs/16, "Fetching"): the `curl` macOS ships, so its
//! certificates and its TLS are the ones used, and no HTTP or TLS code is linked in. The
//! proxy is the one named by hand, or the one System Settings names.

use std::io::Read;
use std::process::{Command, Stdio};

use crate::Network;
use crate::net::{named, parts};

/// The proxy System Settings names for an address, secure or not, and the hosts it leaves
/// out, both as `curl` takes them.
/// ponytail: a proxy written in System Settings. One found by a script (a PAC file) is not
/// asked, and the request goes straight; `CFNetworkCopyProxiesForAutoConfigurationScript`
/// when a network of the kind is met.
fn system_proxy(secure: bool) -> Option<(String, String)> {
    let said = Command::new("/usr/sbin/scutil").arg("--proxy").output().ok()?;
    proxy_of(&String::from_utf8_lossy(&said.stdout), secure)
}

/// What `scutil --proxy` said, a `name : value` a line, as a proxy and what it leaves out.
fn proxy_of(said: &str, secure: bool) -> Option<(String, String)> {
    let kind = if secure { "HTTPS" } else { "HTTP" };
    let pairs = || said.lines().filter_map(|line| line.trim().split_once(" : "));
    let value = |name: String| pairs().find(|it| it.0 == name).map(|it| it.1.trim());
    if value(format!("{kind}Enable")) != Some("1") {
        return None;
    }
    let (host, port) = (value(format!("{kind}Proxy"))?, value(format!("{kind}Port")).unwrap_or("80"));
    // The hosts left out are an array's lines, named by their place: `0 : *.local`.
    let out: Vec<&str> = pairs()
        .filter(|it| it.0.parse::<u32>().is_ok())
        .map(|it| it.1.trim().trim_start_matches('*'))
        .collect();
    Some((format!("{host}:{port}"), out.join(",")))
}

/// GETs `url` with `headers` (the agent is the `User-Agent` among them) and gives `body` what
/// comes, piece by piece, until it answers `false`: the status, or why there is none. Redirects
/// are followed, never from https to http. `network` says the proxy and whether the
/// server's certificate is checked. It blocks: an app calls it on the pool. Connecting gives
/// up after 10 s, a silence of the server after 30 s.
pub fn http_get(
    url: &str,
    headers: &[(&str, &str)],
    network: &Network,
    body: &mut dyn FnMut(&[u8]) -> bool,
) -> Result<u16, String> {
    let (secure, ..) = parts(url).ok_or_else(|| format!("not an http address: {url}"))?;
    let mut curl = Command::new("/usr/bin/curl");
    // Silent but for an error; the address as it is written; the status last, apart from
    // the body.
    curl.args(["-sS", "-g", "-L", "-w", "%{stderr}%{http_code}"]);
    curl.args(["--proto-redir", if secure { "=https" } else { "=http,https" }]);
    curl.args(["--connect-timeout", "10", "--speed-time", "30", "--speed-limit", "1"]);
    // The proxy is said in full, whatever the environment's variables name.
    match network.proxy.as_deref().map(named).transpose()? {
        Some(proxy) => curl.args(["-x", &format!("http://{proxy}"), "--noproxy", ""]),
        None => match system_proxy(secure) {
            Some((proxy, but)) => curl.args(["-x", &format!("http://{proxy}"), "--noproxy", &but]),
            None => curl.args(["--noproxy", "*"]),
        },
    };
    if !network.verify_tls {
        curl.arg("-k");
    }
    if !headers.iter().any(|it| it.0.eq_ignore_ascii_case("user-agent")) {
        curl.args(["-A", "quadra"]);
    }
    for (name, value) in headers {
        curl.args(["-H", &format!("{name}: {value}")]);
    }
    curl.args(["--url", url]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = curl.spawn().map_err(|e| format!("no session: {e}"))?;
    let (mut out, mut piece) = (child.stdout.take().ok_or("no session")?, vec![0u8; 64 * 1024]);
    loop {
        let stopped = match out.read(&mut piece) {
            Ok(0) => break,
            Ok(read) if body(&piece[..read]) => continue,
            Ok(_) => "stopped".to_string(),
            Err(e) => format!("cut off: {e}"),
        };
        let _ = child.kill();
        let _ = child.wait();
        return Err(stopped);
    }
    let done = child.wait_with_output().map_err(|e| format!("cut off: {e}"))?;
    // An error is its words (`curl: (6) Could not resolve host`); the status is the last line.
    let said = String::from_utf8_lossy(&done.stderr);
    let status = said.lines().last().and_then(|it| it.trim().parse::<u16>().ok());
    match status.filter(|_| done.status.success()) {
        Some(status) => Ok(status),
        None => {
            let why = said.lines().find(|line| line.starts_with("curl: "));
            Err(why.map_or("no answer", |line| &line[6..]).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::proxy_of;

    #[test]
    fn the_system_s_proxy_is_read_from_what_it_says() {
        let said = "<dictionary> {\n  ExceptionsList : <array> {\n    0 : *.local\n    1 : 169.254/16\n  }\n  \
                    HTTPEnable : 0\n  HTTPSEnable : 1\n  HTTPSPort : 8080\n  HTTPSProxy : proxy.example\n}\n";
        let found = proxy_of(said, true);
        assert_eq!(found, Some(("proxy.example:8080".to_string(), ".local,169.254/16".to_string())));
        assert_eq!(proxy_of(said, false), None);
        assert_eq!(proxy_of("<dictionary> {\n}\n", true), None);
    }
}
