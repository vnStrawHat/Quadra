//! A GET over the OS's HTTP (specs/16, "Fetching"): WinHTTP, so the system's proxy (or one
//! named by hand), its certificates and its TLS are the ones used, and no HTTP or TLS code is
//! linked in.

use core::ffi::c_void;

use windows::Win32::Networking::WinHttp::{
    SECURITY_FLAG_IGNORE_CERT_CN_INVALID, SECURITY_FLAG_IGNORE_CERT_DATE_INVALID,
    SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE, SECURITY_FLAG_IGNORE_UNKNOWN_CA,
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_NAMED_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_OPEN_REQUEST_FLAGS, WINHTTP_OPTION_SECURITY_FLAGS, WINHTTP_QUERY_FLAG_NUMBER,
    WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetOption,
    WinHttpSetTimeouts,
};
use windows::core::{Error, PCWSTR, w};

use crate::Network;
use crate::net::{named, parts};

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

/// What failed and why: the OS's words, or its code where it has none (WinHTTP's own errors).
fn failed(what: &str, why: &Error) -> String {
    match why.message() {
        words if words.is_empty() => format!("{what}: error {}", why.code().0 & 0xFFFF),
        words => format!("{what}: {words}"),
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// GETs `url` with `headers` (the agent is the `User-Agent` among them) and gives `body` what
/// comes, piece by piece, until it answers `false`: the status, or why there is none. Redirects
/// are followed, never from https to http. `network` says the proxy and whether the
/// server's certificate is checked. It blocks: an app calls it on the pool. Resolving
/// and connecting give up after 10 s, a silence of the server after 30 s.
/// ponytail: no address in brackets (`[::1]`); parsed when a server is named by one.
pub fn http_get(
    url: &str,
    headers: &[(&str, &str)],
    network: &Network,
    body: &mut dyn FnMut(&[u8]) -> bool,
) -> Result<u16, String> {
    let (secure, host, port, path) = parts(url).ok_or_else(|| format!("not an http address: {url}"))?;
    let proxy = network.proxy.as_deref().map(named).transpose()?.map(|it| wide(&it));
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
        let (how, through) = match &proxy {
            Some(proxy) => (WINHTTP_ACCESS_TYPE_NAMED_PROXY, PCWSTR(proxy.as_ptr())),
            None => (WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, none),
        };
        let opened = WinHttpOpen(PCWSTR(agent.as_ptr()), how, through, none, 0);
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
        if secure && !network.verify_tls {
            // Whoever signed the certificate, whatever name, date and use it says.
            let any = SECURITY_FLAG_IGNORE_UNKNOWN_CA
                | SECURITY_FLAG_IGNORE_CERT_CN_INVALID
                | SECURITY_FLAG_IGNORE_CERT_DATE_INVALID
                | SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE;
            WinHttpSetOption(Some(request.0), WINHTTP_OPTION_SECURITY_FLAGS, Some(&any.to_ne_bytes()))
                .map_err(|e| failed("certificates still checked", &e))?;
        }
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
