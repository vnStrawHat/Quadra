//! How a fetch reaches a server (specs/16, "Fetching"), and the parts of the addresses it is
//! given: the same on every system, whose own module does the fetching.

/// How a fetch reaches a server (specs/16, "Fetching").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    /// An HTTP proxy named by hand, `host:port` (an `http://` before it is taken off). None is
    /// the system's, found as the OS finds it.
    pub proxy: Option<String>,
    /// Whether a server's certificate is checked. Off, whoever stands on the way can answer in
    /// the server's place: for a network that opens TLS on the way, and nothing else.
    pub verify_tls: bool,
}

impl Default for Network {
    fn default() -> Network {
        Network { proxy: None, verify_tls: true }
    }
}

/// `http://host[:port]/path` or `https://...` as whether it is secure, its host, port and path.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) fn parts(url: &str) -> Option<(bool, &str, u16, &str)> {
    let (secure, rest) = match url.split_once("://")? {
        ("https", rest) => (true, rest),
        ("http", rest) => (false, rest),
        _ => return None,
    };
    let (authority, path) = rest.find('/').map_or((rest, "/"), |at| rest.split_at(at));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse().ok()?),
        None => (authority, if secure { 443 } else { 80 }),
    };
    (!host.is_empty()).then_some((secure, host, port, path))
}

/// A proxy named by hand as the systems name one: `host:port`.
/// ponytail: an HTTP proxy that asks no name: no SOCKS, no `user:password@`. A proxy that
/// asks for them answers 407; its credentials given to the system's HTTP when one is met.
#[cfg_attr(not(any(windows, target_os = "macos")), allow(dead_code))]
pub(crate) fn named(proxy: &str) -> Result<String, String> {
    let name = proxy.trim().trim_end_matches('/');
    let name = name.strip_prefix("http://").unwrap_or(name);
    let fits = !name.is_empty() && !name.contains(['/', '@', '=', ';', ' ']);
    fits.then(|| name.to_string()).ok_or_else(|| format!("not an HTTP proxy's address (host:port): {proxy}"))
}

#[cfg(test)]
mod tests {
    use super::{named, parts};

    #[test]
    fn an_address_is_its_scheme_host_port_and_path() {
        assert_eq!(
            parts("https://api.example.com/a/b?c=1"),
            Some((true, "api.example.com", 443, "/a/b?c=1"))
        );
        assert_eq!(parts("http://127.0.0.1:8080"), Some((false, "127.0.0.1", 8080, "/")));
        assert_eq!(parts("ftp://example.com/"), None);
        assert_eq!(parts("https:///nothing"), None);
        assert_eq!(parts("http://host:port/"), None);
    }

    #[test]
    fn a_proxy_is_a_host_and_a_port() {
        assert_eq!(named(" http://proxy.example:8080/ ").as_deref(), Ok("proxy.example:8080"));
        assert_eq!(named("10.0.0.1:3128").as_deref(), Ok("10.0.0.1:3128"));
        for no in
            ["", "socks5://127.0.0.1:1080", "https://proxy:443", "user:secret@proxy:8080", "http=a;https=b"]
        {
            assert!(named(no).is_err(), "{no}");
        }
    }
}
