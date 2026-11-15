//! The system's appearance (specs/10, "Win32 behaviour"): read from the user's settings.

use quadra_platform::Appearance;
use quadra_platform::SystemColors;
use quadra_prim::Color;
use windows::Win32::Graphics::Gdi::{
    COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_HOTLIGHT,
    COLOR_WINDOW, COLOR_WINDOWTEXT, GetSysColor, SYS_COLOR_INDEX,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::{HSTRING, PCWSTR};

/// A number of the user's settings, if it is there.
fn dword(key: &str, value: &str) -> Option<u32> {
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    let (mut data, mut size) = (0u32, 4u32);
    // SAFETY: the strings live through the call, and `data` holds the four bytes asked for.
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    read.is_ok().then_some(data)
}

pub(crate) fn appearance() -> Appearance {
    let themes = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
    let dark = dword(themes, "AppsUseLightTheme") == Some(0);
    // The window manager's accent is 0xAABBGGRR.
    let accent = dword(r"Software\Microsoft\Windows\DWM", "AccentColor")
        .map(|abgr| Color::rgb(((abgr & 0xFF) << 16) | (abgr & 0xFF00) | ((abgr >> 16) & 0xFF)));
    let mut contrast = HIGHCONTRASTW { cbSize: size_of::<HIGHCONTRASTW>() as u32, ..Default::default() };
    // SAFETY: `contrast` is the structure this setting fills, its size given.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some(&mut contrast as *mut HIGHCONTRASTW as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    let high_contrast = read.is_ok() && contrast.dwFlags.contains(HCF_HIGHCONTRASTON);
    // Percent, from 100 to 225; not there at 100.
    let text = dword(r"Software\Microsoft\Accessibility", "TextScaleFactor").unwrap_or(100);
    let text_scale = text.clamp(100, 225) as f32 / 100.0;
    Appearance { dark, high_contrast, accent, colors: Some(system_colors()), text_scale }
}

/// The colours of the system's controls; a colour of it is 0x00BBGGRR.
fn system_colors() -> SystemColors {
    let get = |index: SYS_COLOR_INDEX| {
        // SAFETY: a plain call with one of the indices the system knows.
        let bgr = unsafe { GetSysColor(index) };
        Color::rgb(((bgr & 0xFF) << 16) | (bgr & 0xFF00) | ((bgr >> 16) & 0xFF))
    };
    SystemColors {
        window: get(COLOR_WINDOW),
        text: get(COLOR_WINDOWTEXT),
        highlight: get(COLOR_HIGHLIGHT),
        highlight_text: get(COLOR_HIGHLIGHTTEXT),
        link: get(COLOR_HOTLIGHT),
        disabled: get(COLOR_GRAYTEXT),
        button: get(COLOR_BTNFACE),
        button_text: get(COLOR_BTNTEXT),
    }
}
