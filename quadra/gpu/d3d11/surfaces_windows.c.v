// gpu.Surface implementations: a window swap chain and an offscreen texture.
module d3d11

// WindowSurface draws into the window's swap chain or, while the first frames draw on WARP
// (gdi, upgrade_windows.c.v), into an offscreen texture copied to the window with GDI: the
// swap chain is then made once, on the hardware device (replacing a window's swap chain
// left frames paced late for good).
@[heap]
pub struct WindowSurface {
mut:
	dev  &Device = unsafe { nil }
	sc   SwapChain
	hwnd voidptr
	gdi  bool
	off  Offscreen
}

pub fn (mut s WindowSurface) resize(width int, height int) ! {
	if s.gdi {
		if width <= 0 || height <= 0 || (u32(width) == s.off.width && u32(height) == s.off.height) {
			return
		}
		s.off.release()
		s.off = new_offscreen(s.dev, u32(width), u32(height))!
		return
	}
	s.sc.resize(s.dev, u32(width), u32(height))!
}

pub fn (mut s WindowSurface) present() ! {
	if s.gdi {
		s.off.blit(s.dev, s.hwnd)!
		return
	}
	s.sc.present()!
}

pub fn (s &WindowSurface) waitable() voidptr {
	if s.gdi {
		return unsafe { nil }
	}
	return s.sc.waitable
}

fn (s &WindowSurface) rtv() voidptr {
	if s.gdi {
		return s.off.rtv
	}
	return s.sc.rtv
}

pub fn (s &WindowSurface) read_rgba() ![]u8 {
	return error('window surfaces cannot be read back')
}

pub fn (s &WindowSurface) width() int {
	return int(if s.gdi {
		s.off.width
	} else {
		s.sc.width
	})
}

pub fn (s &WindowSurface) height() int {
	return int(if s.gdi {
		s.off.height
	} else {
		s.sc.height
	})
}

@[heap]
pub struct OffscreenSurface {
mut:
	dev &Device = unsafe { nil }
	off Offscreen
}

pub fn (mut s OffscreenSurface) resize(width int, height int) ! {
	if u32(width) == s.off.width && u32(height) == s.off.height {
		return
	}
	s.off.release()
	s.off = new_offscreen(s.dev, u32(width), u32(height))!
}

pub fn (mut s OffscreenSurface) present() ! {
	s.dev.flush()
}

pub fn (s &OffscreenSurface) waitable() voidptr {
	return unsafe { nil }
}

pub fn (s &OffscreenSurface) read_rgba() ![]u8 {
	return s.off.read_rgba(s.dev)
}

pub fn (s &OffscreenSurface) width() int {
	return int(s.off.width)
}

pub fn (s &OffscreenSurface) height() int {
	return int(s.off.height)
}
