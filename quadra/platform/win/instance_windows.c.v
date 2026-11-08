// Single instance: a named mutex decides the first instance; later launches find its
// message window by title and forward their command line with WM_COPYDATA.
module win

import quadra.platform

@[typedef]
pub struct C.COPYDATASTRUCT {
pub mut:
	dwData usize
	cbData u32
	lpData voidptr
}

fn C.CreateMutexW(attrs voidptr, owner int, name &u16) voidptr
fn C.FindWindowExW(parent voidptr, after voidptr, class &u16, title &u16) voidptr
fn C.SendMessageW(hwnd voidptr, msg u32, wparam usize, lparam isize) isize
fn C.SetWindowTextW(hwnd voidptr, text &u16) int

const wm_copydata = u32(0x004A)
const error_already_exists = u32(183)
const instance_magic = usize(0x51554144) // 'QUAD'

// claim_single_instance returns true for the first instance of `key` in this session.
// Other instances forward args to it and get false (they should exit).
pub fn (mut p WinPlatform) claim_single_instance(key string, args []string) bool {
	title := 'quadra-instance:${key}'
	name := 'Local' + r'\' + 'quadra-${key}' // per logon session
	C.CreateMutexW(unsafe { nil }, 0, name.to_wide())
	if C.GetLastError() != error_already_exists {
		C.SetWindowTextW(p.msg_hwnd, title.to_wide())
		return true
	}
	target := C.FindWindowExW(voidptr(-3), unsafe { nil }, msg_class_name.to_wide(),
		title.to_wide())
	if target != unsafe { nil } {
		payload := args.join('\n')
		cds := C.COPYDATASTRUCT{
			dwData: instance_magic
			cbData: u32(payload.len)
			lpData: payload.str
		}
		C.SendMessageW(target, wm_copydata, 0, isize(voidptr(&cds)))
	}
	return false
}

// forwarded_args decodes a WM_COPYDATA sent by claim_single_instance.
fn forwarded_args(lparam isize) ?platform.InstanceArgs {
	cds := unsafe { &C.COPYDATASTRUCT(voidptr(lparam)) }
	if cds.dwData != instance_magic {
		return none
	}
	text := unsafe { tos(&u8(cds.lpData), int(cds.cbData)) }.clone()
	return platform.InstanceArgs{if text == '' { []string{} } else { text.split('\n') }}
}
