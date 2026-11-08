// A GPU device that draws nothing and remembers the last scene: the default for
// headless tests (no driver, no window, any OS).
module null

import quadra.prim
import quadra.gpu

@[heap]
pub struct NullDevice {
pub mut:
	last    gpu.Scene
	uploads []gpu.Upload
	st      gpu.RenderStats
mut:
	held     voidptr // the scene whose instances `last` mirrors
	held_rev u64
}

pub fn new() &NullDevice {
	return &NullDevice{}
}

pub fn (mut d NullDevice) upload(u gpu.Upload) ! {
	d.uploads << u
	d.st.upload_bytes += u.pixels.len
}

pub fn (mut d NullDevice) new_window_surface(native_handle voidptr, width int, height int) !gpu.Surface {
	return &NullSurface{
		w: width
		h: height
	}
}

pub fn (mut d NullDevice) new_offscreen_surface(width int, height int) !gpu.Surface {
	return &NullSurface{
		w: width
		h: height
	}
}

pub fn (mut d NullDevice) draw(mut surface gpu.Surface, scene &gpu.Scene, clear prim.Color) ! {
	// Behaves like a GPU instance buffer: a scene it already holds only gets its dirty range.
	start, end := scene.dirty_range()
	if voidptr(scene) == d.held && scene.revision == d.held_rev
		&& d.last.instances.len == scene.instances.len {
		for i in start .. end {
			d.last.instances[i] = scene.instances[i]
		}
		d.st.upload_instances = end - start
	} else {
		d.last = gpu.Scene{
			instances: scene.instances.clone()
			xforms:    scene.xforms.clone()
		}
		d.held, d.held_rev = voidptr(scene), scene.revision
		d.st.upload_instances = scene.instances.len
	}
	d.st.frames++
	d.st.draw_calls = scene.batches().len
	d.st.instances = scene.instances.len
}

pub fn (d &NullDevice) stats() gpu.RenderStats {
	return d.st
}

pub fn (d &NullDevice) is_software() bool {
	return true
}

pub fn (d &NullDevice) upgrading() bool {
	return false
}

pub fn (mut d NullDevice) try_upgrade() bool {
	return false
}

@[heap]
pub struct NullSurface {
mut:
	w        int
	h        int
	presents int
}

pub fn (mut s NullSurface) resize(width int, height int) ! {
	s.w, s.h = width, height
}

pub fn (mut s NullSurface) present() ! {
	s.presents++
}

pub fn (s &NullSurface) waitable() voidptr {
	return unsafe { nil }
}

pub fn (s &NullSurface) read_rgba() ![]u8 {
	return error('the null device has no pixels')
}

pub fn (s &NullSurface) width() int {
	return s.w
}

pub fn (s &NullSurface) height() int {
	return s.h
}
