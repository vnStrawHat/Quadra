// Notification area icon and notifications (Shell_NotifyIconW, version 4). The icon's
// callbacks go to the platform's message window and reach the app as TrayEvent.
module win

import quadra.platform

// NotifyIconData is NOTIFYICONDATAW (shellapi.h), 976 bytes on x64.
struct NotifyIconData {
mut:
	size         u32
	hwnd         voidptr
	id           u32
	flags        u32
	callback     u32
	icon         voidptr
	tip          [128]u16
	state        u32
	state_mask   u32
	info         [256]u16
	version      u32 // the union of uTimeout and uVersion
	info_title   [64]u16
	info_flags   u32
	guid         Guid
	balloon_icon voidptr
}

type FnShellNotifyIcon = fn (msg u32, data &NotifyIconData) int

fn C.LoadIconW(inst voidptr, name voidptr) voidptr

const nim_add = u32(0)
const nim_modify = u32(1)
const nim_delete = u32(2)
const nim_setversion = u32(4)
const nif_message = u32(0x1)
const nif_icon = u32(0x2)
const nif_tip = u32(0x4)
const nif_info = u32(0x10)
const nif_showtip = u32(0x80)
const wm_tray = wm_app + 2

fn put_wide(mut dst []u16, s string) {
	w := s.to_wide()
	mut i := 0
	for i < dst.len - 1 && unsafe { w[i] } != 0 {
		dst[i] = unsafe { w[i] }
		i++
	}
	dst[i] = 0
}

fn (mut p WinPlatform) tray_data(flags u32) NotifyIconData {
	mut d := NotifyIconData{
		size:     u32(sizeof(NotifyIconData))
		hwnd:     p.msg_hwnd
		id:       1
		flags:    flags
		callback: wm_tray
	}
	if flags & nif_icon != 0 {
		inst := C.GetModuleHandleW(unsafe { nil })
		d.icon = C.LoadIconW(inst, voidptr(usize(1))) // the exe's first icon, if it has one
		if d.icon == unsafe { nil } {
			d.icon = C.LoadIconW(unsafe { nil }, voidptr(usize(32512))) // IDI_APPLICATION
		}
	}
	return d
}

fn (mut p WinPlatform) shell_notify(msg u32, d &NotifyIconData) bool {
	f := p.shell.notify_icon
	return f != unsafe { nil } && f(msg, d) != 0
}

pub fn (mut p WinPlatform) set_tray(tooltip string) bool {
	if tooltip == '' {
		if !p.tray_on {
			return true
		}
		p.tray_on = false
		d := p.tray_data(0)
		return p.shell_notify(nim_delete, &d)
	}
	mut d := p.tray_data(nif_message | nif_icon | nif_tip | nif_showtip)
	mut tip := []u16{len: 128}
	put_wide(mut tip, tooltip)
	for i in 0 .. 128 {
		d.tip[i] = tip[i]
	}
	if p.tray_on {
		return p.shell_notify(nim_modify, &d)
	}
	if !p.shell_notify(nim_add, &d) {
		return false
	}
	p.tray_on = true
	d.version = 4 // NOTIFYICON_VERSION_4: the event in lParam, the point in wParam
	return p.shell_notify(nim_setversion, &d)
}

pub fn (mut p WinPlatform) notify(title string, body string) bool {
	if !p.tray_on && !p.set_tray(title) {
		return false
	}
	mut d := p.tray_data(nif_info)
	mut info := []u16{len: 256}
	put_wide(mut info, body)
	for i in 0 .. 256 {
		d.info[i] = info[i]
	}
	mut head := []u16{len: 64}
	put_wide(mut head, title)
	for i in 0 .. 64 {
		d.info_title[i] = head[i]
	}
	d.info_flags = 0x1 // NIIF_INFO
	return p.shell_notify(nim_modify, &d)
}

// tray_event decodes a version 4 callback; none for the events the app does not see.
fn tray_event(wparam usize, lparam isize) ?platform.TrayEvent {
	action := match u32(lparam) & 0xFFFF {
		0x0400, 0x0401 { platform.TrayAction.click } // NIN_SELECT, NIN_KEYSELECT
		0x0203 { .double_click } // WM_LBUTTONDBLCLK
		0x007B { .menu } // WM_CONTEXTMENU
		else { return none }
	}

	return platform.TrayEvent{action, int(i16(wparam & 0xFFFF)), int(i16((wparam >> 16) & 0xFFFF))}
}
