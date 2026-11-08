// System appearance: dark mode, accent color, high contrast and text scale.
module win

import quadra.prim
import quadra.platform

type FnRegGetValueW = fn (key voidptr, sub &u16, value &u16, flags u32, typ voidptr, data voidptr, size &u32) int

type FnDwmGetColorizationColor = fn (color &u32, opaque &int) int

fn C.SystemParametersInfoW(action u32, param u32, pv voidptr, ini u32) int

struct HighContrast {
	size   u32
	flags  u32
	scheme voidptr
}

const hkey_current_user = voidptr(usize(0x80000001))
const rrf_rt_dword = u32(0x00000010)

fn reg_dword(path string, name string) ?u32 {
	lib := load_system_dll('advapi32.dll') or { return none }
	get := unsafe { FnRegGetValueW(dll_proc(lib, 'RegGetValueW') or { return none }) }
	mut v := u32(0)
	mut size := u32(4)
	if get(hkey_current_user, path.to_wide(), name.to_wide(), rrf_rt_dword, unsafe { nil }, &v,
		&size) != 0 {
		return none
	}
	return v
}

pub fn (p &WinPlatform) appearance() platform.Appearance {
	light := reg_dword(r'Software\Microsoft\Windows\CurrentVersion\Themes\Personalize',
		'AppsUseLightTheme') or { 1 }
	scale := reg_dword(r'Software\Microsoft\Accessibility', 'TextScaleFactor') or { 100 }
	mut accent := prim.rgb(0x2563EB)
	if lib := load_system_dll('dwmapi.dll') {
		if f := dll_proc(lib, 'DwmGetColorizationColor') {
			get := unsafe { FnDwmGetColorizationColor(f) }
			mut argb, mut opaque := u32(0), 0
			if get(&argb, &opaque) == 0 {
				accent = prim.rgb(argb & 0xFFFFFF)
			}
		}
	}
	hc := HighContrast{
		size: u32(sizeof(HighContrast))
	}
	C.SystemParametersInfoW(0x0042, u32(sizeof(HighContrast)), &hc, 0) // SPI_GETHIGHCONTRAST
	return platform.Appearance{
		dark:          light == 0
		accent:        accent
		high_contrast: hc.flags & 1 != 0
		text_scale:    f32(scale) / 100
	}
}

// font_smoothing reports whether the user has ClearType on and whether the panel's subpixels
// run blue-green-red (SPI_GETFONTSMOOTHING, ..TYPE, ..ORIENTATION).
pub fn font_smoothing() (bool, bool) {
	mut on, mut kind, mut orient := 0, u32(0), u32(1)
	C.SystemParametersInfoW(0x004A, 0, &on, 0)
	C.SystemParametersInfoW(0x200A, 0, &kind, 0)
	C.SystemParametersInfoW(0x2012, 0, &orient, 0)
	return on != 0 && kind == 2, orient == 0
}
