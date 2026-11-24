//! A GET over the OS's HTTP (specs/16, "Fetching"): WinHTTP, so the system's proxy, its
//! certificates and its TLS are the ones used, and no HTTP or TLS code is linked in.

use core::ffi::c_void;

use windows::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen,
    WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};
use windows::core::{Error, PCWSTR, w};

/// A handle of WinHTTP, closed when dropped.
struct Handle(*mut c_void);

impl Handle {
    /// The handle a call gave, or why it gave none.
    fn of(raw: *mut c_void, what: &str) -> Result<Handle, String> {
        if raw.is_null() { Err(failed(what, &Error::from_thread())) } else { Ok(Handle(raw)) }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle was given by WinHTTP and is closed once, here.
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn failed(what: &str, why: &Error) -> String {
    format!("{what}: {}", why.message())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// `http://host[:port]/path` or `https://...` as whether it is secure, its host, port and path.
fn parts(url: &str) -> Option<(bool, &str, u16, &str)> {
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

/// GETs `url` with `headers` (the agent is the `User-Agent` among them) and gives `body` what
/// comes, piece by piece, until it answers `false`: the status, or why there is none. Redirects
/// are followed, never from https to http. It blocks: an app calls it on the pool. Resolving
/// and connecting give up after 10 s, a silence of the server after 30 s.
/// ponytail: no address in brackets (`[::1]`); parsed when a server is named by one.
pub fn http_get(
    url: &str,
    headers: &[(&str, &str)],
    body: &mut dyn FnMut(&[u8]) -> bool,
) -> Result<u16, String> {
    let (secure, host, port, path) = parts(url).ok_or_else(|| format!("not an http address: {url}"))?;
    let agent = headers.iter().find(|it| it.0.eq_ignore_ascii_case("user-agent")).map_or("quadra", |it| it.1);
    let lines: String = headers.iter().map(|(name, value)| format!("{name}: {value}\r\n")).collect();
    let (agent, host, path) = (wide(agent), wide(host), wide(path));
    let lines: Vec<u16> = lines.encode_utf16().collect();
    let flags = if secure { WINHTTP_FLAG_SECURE } else { WINHTTP_OPEN_REQUEST_FLAGS(0) };
    // SAFETY: every string is a local that is terminated and outlives its call, every handle
    // is checked before it is used and closed by its guard, and the buffers read into are
    // locals told with their lengths.
    unsafe {
        let none = PCWSTR::null();
        let opened = WinHttpOpen(PCWSTR(agent.as_ptr()), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, none, none, 0);
        let session = Handle::of(opened, "no session")?;
        WinHttpSetTimeouts(session.0, 10_000, 10_000, 30_000, 30_000)
            .map_err(|e| failed("no timeouts", &e))?;
        let connect = Handle::of(WinHttpConnect(session.0, PCWSTR(host.as_ptr()), port, 0), "no connection")?;
        let asked = WinHttpOpenRequest(
            connect.0,
            w!("GET"),
            PCWSTR(path.as_ptr()),
            none,
            none,
            core::ptr::null(),
            flags,
        );
        let request = Handle::of(asked, "no request")?;
        WinHttpSendRequest(request.0, (!lines.is_empty()).then_some(&lines[..]), None, 0, 0, 0)
            .map_err(|e| failed("not sent", &e))?;
        WinHttpReceiveResponse(request.0, core::ptr::null_mut()).map_err(|e| failed("no answer", &e))?;
        let (mut status, mut size) = (0u32, size_of::<u32>() as u32);
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            none,
            Some((&raw mut status).cast()),
            &mut size,
            core::ptr::null_mut(),
        )
        .map_err(|e| failed("no status", &e))?;
        let mut piece = vec![0u8; 64 * 1024];
        loop {
            let mut read = 0u32;
            WinHttpReadData(request.0, piece.as_mut_ptr().cast(), piece.len() as u32, &mut read)
                .map_err(|e| failed("cut off", &e))?;
            if read == 0 {
                return Ok(status as u16);
            }
            if !body(&piece[..read as usize]) {
                return Err("stopped".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_address_is_its_scheme_host_port_and_path() {
        use super::parts;
        assert_eq!(
            parts("https://api.example.com/a/b?c=1"),
            Some((true, "api.example.com", 443, "/a/b?c=1"))
        );
        assert_eq!(parts("http://127.0.0.1:8080"), Some((false, "127.0.0.1", 8080, "/")));
        assert_eq!(parts("ftp://example.com/"), None);
        assert_eq!(parts("https:///nothing"), None);
        assert_eq!(parts("http://host:port/"), None);
    }
}
