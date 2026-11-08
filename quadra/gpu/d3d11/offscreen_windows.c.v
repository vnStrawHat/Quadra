// Offscreen render target with CPU readback: no window, no swap chain.
// Used for pixel snapshots in headless tests (works without a desktop session).
module d3d11

import quadra.platform.win

pub struct Offscreen {
pub:
	width  u32
	height u32
	rtv    voidptr
mut:
	tex     voidptr
	staging voidptr
}

pub fn new_offscreen(d &Device, width u32, height u32) !Offscreen {
	tex := d.create_texture2d(&Texture2dDesc{
		width:      width
		height:     height
		format:     format_b8g8r8a8_unorm
		usage:      usage_default
		bind_flags: bind_render_target | bind_shader_resource
	}, unsafe { nil })!
	return Offscreen{
		width:   width
		height:  height
		tex:     tex
		rtv:     d.create_rtv(tex)!
		staging: d.create_texture2d(&Texture2dDesc{
			width:            width
			height:           height
			format:           format_b8g8r8a8_unorm
			usage:            usage_staging
			cpu_access_flags: cpu_access_read
		}, unsafe { nil })!
	}
}

// read_rgba copies the rendered image back as tightly packed RGBA8 rows.
pub fn (o &Offscreen) read_rgba(d &Device) ![]u8 {
	d.copy_resource(o.staging, o.tex)
	m := d.map_read(o.staging)!
	defer {
		d.unmap(o.staging)
	}
	w, h := int(o.width), int(o.height)
	mut out := []u8{len: w * h * 4}
	for y in 0 .. h {
		row := unsafe { &u8(m.data) + y * int(m.row_pitch) }
		for x in 0 .. w {
			unsafe {
				out[(y * w + x) * 4 + 0] = row[x * 4 + 2]
				out[(y * w + x) * 4 + 1] = row[x * 4 + 1]
				out[(y * w + x) * 4 + 2] = row[x * 4 + 0]
				out[(y * w + x) * 4 + 3] = row[x * 4 + 3]
			}
		}
	}
	return out
}

pub fn (mut o Offscreen) release() {
	win.release(o.rtv)
	win.release(o.tex)
	win.release(o.staging)
}

fn C.GetDC(hwnd voidptr) voidptr
fn C.ReleaseDC(hwnd voidptr, hdc voidptr) int

// gdi32 is loaded at run time: linking it makes V build a GUI-subsystem program (no
// console output).
type FnSetDIBits = fn (hdc voidptr, x int, y int, w u32, h u32, sx int, sy int, start u32, lines u32, bits voidptr, bmi voidptr, usage u32) int

// BitmapInfoHeader is BITMAPINFOHEADER (40 bytes).
struct BitmapInfoHeader {
	size          u32
	width         int
	height        int
	planes        u16
	bit_count     u16
	compression   u32
	size_image    u32
	x_ppm         int
	y_ppm         int
	clr_used      u32
	clr_important u32
}

// blit copies the rendered image to a window's client area with GDI (BGRA rows as they
// are mapped: the row pitch is the bitmap width).
pub fn (o &Offscreen) blit(d &Device, hwnd voidptr) ! {
	gdi := win.load_system_dll('gdi32.dll')!
	set_bits := unsafe { FnSetDIBits(win.dll_proc(gdi, 'SetDIBitsToDevice')!) }
	d.copy_resource(o.staging, o.tex)
	m := d.map_read(o.staging)!
	defer {
		d.unmap(o.staging)
	}
	bmi := BitmapInfoHeader{
		size:      40
		width:     int(m.row_pitch / 4)
		height:    -int(o.height) // top-down
		planes:    1
		bit_count: 32
	}
	dc := C.GetDC(hwnd)
	set_bits(dc, 0, 0, o.width, o.height, 0, 0, 0, o.height, m.data, &bmi, 0)
	C.ReleaseDC(hwnd, dc)
}
