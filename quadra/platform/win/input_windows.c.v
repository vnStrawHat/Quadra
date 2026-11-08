// Keyboard and mouse translation from Win32 messages to platform events.
module win

import quadra.platform

fn current_mods() platform.Mods {
	mut m := platform.Mods.zero()
	if C.GetKeyState(0x10) < 0 {
		m.set(.shift)
	}
	if C.GetKeyState(0x11) < 0 {
		m.set(.ctrl)
	}
	if C.GetKeyState(0x12) < 0 {
		m.set(.alt)
	}
	if C.GetKeyState(0x5B) < 0 || C.GetKeyState(0x5C) < 0 {
		m.set(.super)
	}
	return m
}

fn key_from_vk(vk u32) platform.Key {
	if vk >= 0x41 && vk <= 0x5A {
		return unsafe { platform.Key(int(platform.Key.a) + int(vk - 0x41)) }
	}
	if vk >= 0x30 && vk <= 0x39 {
		return unsafe { platform.Key(int(platform.Key.digit0) + int(vk - 0x30)) }
	}
	if vk >= 0x70 && vk <= 0x7B {
		return unsafe { platform.Key(int(platform.Key.f1) + int(vk - 0x70)) }
	}
	return match vk {
		0x0D { .enter }
		0x1B { .escape }
		0x08 { .backspace }
		0x09 { .tab }
		0x20 { .space }
		0x2E { .delete }
		0x2D { .insert }
		0x24 { .home }
		0x23 { .end }
		0x21 { .page_up }
		0x22 { .page_down }
		0x25 { .left }
		0x27 { .right }
		0x26 { .up }
		0x28 { .down }
		0x10 { .shift }
		0x11 { .ctrl }
		0x12 { .alt }
		0x5B, 0x5C { .super }
		0x14 { .caps_lock }
		0x5D { .menu }
		0xBD { .minus }
		0xBB { .equal }
		0xDB { .bracket_left }
		0xDD { .bracket_right }
		0xDC { .backslash }
		0xBA { .semicolon }
		0xDE { .quote }
		0xBC { .comma }
		0xBE { .period }
		0xBF { .slash }
		0xC0 { .backquote }
		else { .unknown }
	}
}

fn button_of(msg u32, wparam usize) platform.MouseButton {
	return match msg {
		wm_lbuttondown, wm_lbuttonup {
			platform.MouseButton.left
		}
		wm_rbuttondown, wm_rbuttonup {
			platform.MouseButton.right
		}
		wm_mbuttondown, wm_mbuttonup {
			platform.MouseButton.middle
		}
		wm_xbuttondown, wm_xbuttonup {
			if (wparam >> 16) & 0xFFFF == 1 {
				platform.MouseButton.back
			} else {
				platform.MouseButton.forward
			}
		}
		else {
			platform.MouseButton.none
		}
	}
}

// utf16_to_string converts `n` UTF-16 units to UTF-8.
fn utf16_to_string(p &u16, n int) string {
	if n <= 0 {
		return ''
	}
	return unsafe { string_from_wide2(p, n) }
}

// utf8_offset_of_utf16 maps a UTF-16 index inside `units` to a UTF-8 byte offset.
fn utf8_offset_of_utf16(units []u16, idx int) int {
	if idx <= 0 {
		return 0
	}
	end := if idx > units.len { units.len } else { idx }
	return utf16_to_string(&u16(units.data), end).len
}
