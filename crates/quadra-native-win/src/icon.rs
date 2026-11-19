//! A window's icon (specs/10, "Win32 behaviour"): the executable's icon resource 1, at the
//! system's sizes for an icon and a small one; none where it has no such resource.

use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, HICON, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW, SM_CXICON, SM_CXSMICON, SM_CYICON,
    SM_CYSMICON, SYSTEM_METRICS_INDEX,
};
use windows::core::PCWSTR;

/// The icon and the small icon of the executable `module`, if it has them.
pub(crate) fn exe_icons(module: HINSTANCE) -> (HICON, HICON) {
    let load = |wide: SYSTEM_METRICS_INDEX, high: SYSTEM_METRICS_INDEX| {
        // SAFETY: resource 1 is asked for by number (MAKEINTRESOURCE), which is no pointer;
        // a missing one is an error, given as no icon.
        unsafe {
            let (wide, high) = (GetSystemMetrics(wide), GetSystemMetrics(high));
            let name = PCWSTR(std::ptr::without_provenance(1));
            LoadImageW(Some(module), name, IMAGE_ICON, wide, high, LR_DEFAULTCOLOR)
                .map(|it| HICON(it.0))
                .unwrap_or_default()
        }
    };
    (load(SM_CXICON, SM_CYICON), load(SM_CXSMICON, SM_CYSMICON))
}
