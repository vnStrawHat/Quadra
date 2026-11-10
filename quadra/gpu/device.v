// GPU device and surface interfaces implemented by every backend (d3d11, null, ...).
module gpu

import quadra.prim

pub struct RenderStats {
pub mut:
	frames           int
	draw_calls       int // in the last frame
	instances        int // in the last frame
	upload_bytes     i64 // total
	upload_instances int // instances sent in the last frame (all, or a patched range)
	resident_bytes   i64 // atlas pages and instance buffers the backend holds (not swap chains)
}

// Device owns GPU resources shared by all windows: atlas textures and pipelines.
pub interface Device {
	stats() RenderStats
	is_software() bool
	adapter_name() string // the adapter drawing (DXGI's description on D3D11)
	upgrading() bool      // a faster device is on its way (try_upgrade)
mut:
	// try_upgrade moves to that device once it is ready; true when it did (every surface
	// must draw again and may have a new waitable).
	try_upgrade() bool
	upload(u Upload) !
	new_window_surface(native_handle voidptr, width int, height int) !Surface
	new_offscreen_surface(width int, height int) !Surface
	draw(mut surface Surface, scene &Scene, clear prim.Color) !
}

// Surface is a render target: a window swap chain or an offscreen texture.
pub interface Surface {
	waitable() voidptr // frame-latency handle, nil when not applicable
	read_rgba() ![]u8  // offscreen surfaces only
	width() int
	height() int
mut:
	resize(width int, height int) !
	present() !
}
