//! File types on Windows (specs/16, "File types"): the values of the registration written
//! under `HKEY_CURRENT_USER`, the shell's answer to which program opens a kind, and the
//! system's Default apps page.

use std::path::PathBuf;

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW,
    RegSetValueExW,
};
use windows::Win32::UI::Shell::{
    ASSOCF_INIT_IGNOREUNKNOWN, ASSOCSTR_EXECUTABLE, AssocQueryStringW, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
    SHChangeNotify,
};
use windows::core::{HSTRING, PCWSTR, PWSTR};

use super::{WinShell, apartment, shell_open};
use crate::files::{Associations, FileTypes, RegValue, default_apps_page, registration};

/// Writes one string value, making its key where it is missing.
fn write(it: &RegValue) -> Result<(), String> {
    let (key, name) = (HSTRING::from(it.key.as_str()), HSTRING::from(it.name.as_str()));
    let data: Vec<u8> = it.data.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
    let mut opened = HKEY::default();
    // SAFETY: the strings live through the calls, `opened` is closed once made, and the data
    // is a whole UTF-16 string with its end.
    unsafe {
        let options = (REG_OPTION_NON_VOLATILE, KEY_SET_VALUE);
        RegCreateKeyExW(HKEY_CURRENT_USER, &key, None, None, options.0, options.1, None, &mut opened, None)
            .ok()
            .map_err(|e| format!("{}: {e}", it.key))?;
        let set = RegSetValueExW(opened, &name, None, REG_SZ, Some(&data)).ok();
        let _ = RegCloseKey(opened);
        set.map_err(|e| format!("{}\\{}: {e}", it.key, it.name))
    }
}

impl Associations for WinShell {
    fn register(&self, types: &FileTypes) -> Result<(), String> {
        registration(types).iter().try_for_each(write)?;
        // SAFETY: no items are passed with this event.
        unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
        Ok(())
    }

    fn opener_of(&self, extension: &str) -> Option<PathBuf> {
        let mut out = vec![0u16; 1024];
        let mut length = out.len() as u32;
        let ext = HSTRING::from(extension);
        // SAFETY: `out` holds `length` characters, which the call is told.
        let asked = unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN,
                ASSOCSTR_EXECUTABLE,
                &ext,
                PCWSTR::null(),
                Some(PWSTR(out.as_mut_ptr())),
                &mut length,
            )
        };
        asked.ok().ok()?;
        let end = out.iter().position(|&c| c == 0).unwrap_or(out.len());
        Some(PathBuf::from(String::from_utf16_lossy(&out[..end])))
    }

    fn choose_default(&self, types: &FileTypes) -> bool {
        apartment();
        shell_open(&HSTRING::from(default_apps_page(&types.name)), None)
    }
}
