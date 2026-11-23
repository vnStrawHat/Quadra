//! The user's environment as the registry says it now (specs/19, "M4 in detail"): what a
//! new shell gets, so a variable set after the app started is in it.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{TOKEN_DUPLICATE, TOKEN_IMPERSONATE, TOKEN_QUERY};
use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// The system's and the user's variables, expanded, without the process's own; empty when
/// the OS does not make them.
pub fn user_environment() -> Vec<(String, String)> {
    let mut token = HANDLE::default();
    // SAFETY: the process's own token, opened for what `CreateEnvironmentBlock` needs and
    // closed below.
    if unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY | TOKEN_IMPERSONATE | TOKEN_DUPLICATE, &mut token)
    }
    .is_err()
    {
        return Vec::new();
    }
    let mut block = std::ptr::null_mut();
    // SAFETY: `block` is a local the call writes; the token is open.
    let made = unsafe { CreateEnvironmentBlock(&mut block, Some(token), false) };
    // SAFETY: opened above, not used after.
    let _ = unsafe { CloseHandle(token) };
    if made.is_err() || block.is_null() {
        return Vec::new();
    }
    let mut all = Vec::new();
    let mut at = block.cast::<u16>().cast_const();
    loop {
        // SAFETY: the block is `name=value` strings, each NUL-ended, the last followed by a NUL.
        let len = (0..).take_while(|&i| unsafe { *at.add(i) } != 0).count();
        if len == 0 {
            break;
        }
        // SAFETY: `len` units were read just now.
        let entry = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(at, len) });
        // `=C:=C:\..`: a drive's directory, the process's, not the user's.
        if let Some((name, value)) = entry.split_once('=').filter(|(name, _)| !name.is_empty()) {
            all.push((name.to_string(), value.to_string()));
        }
        // SAFETY: past this string's NUL, still in the block.
        at = unsafe { at.add(len + 1) };
    }
    // SAFETY: made by `CreateEnvironmentBlock`, read above.
    let _ = unsafe { DestroyEnvironmentBlock(block) };
    all
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_user_s_variables_are_read() {
        let all = super::user_environment();
        assert!(all.iter().any(|(name, _)| name.eq_ignore_ascii_case("PATH")), "{all:?}");
        assert!(all.iter().all(|(name, _)| !name.is_empty() && !name.starts_with('=')));
    }
}
