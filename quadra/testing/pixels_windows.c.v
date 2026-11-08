// Pixel snapshots on Windows: the headless platform with real D3D11 (forced WARP) and
// DirectWrite. Needs no desktop or interactive session; WARP output is deterministic.
module testing

import stbi
import quadra.core
import quadra.gpu.d3d11
import quadra.platform.headless
import quadra.text
import quadra.text.dwrite

@[params]
pub struct PixelOptions {
pub:
	cleartype ?bool // none: follow the Windows setting
}

// new_pixel_harness is like new_harness but renders real pixels.
pub fn new_pixel_harness(opts PixelOptions) !&PixelHarness {
	plat := headless.new()
	dev := d3d11.new_backend(force_warp: true)!
	mut tb := dwrite.new_backend()!
	if ct := opts.cleartype {
		tb.cleartype = ct
	}
	app := core.new_app(core.Backends{
		platform: plat
		device:   dev
		text:     tb
	}, core.AppOptions{
		text_style: text.TextStyle{
			size:        14
			line_height: 1.5
		}
	})!
	return &PixelHarness{
		Harness: Harness{
			app:  app
			plat: plat
			dev:  unsafe { nil }
		}
	}
}

@[heap]
pub struct PixelHarness {
	Harness
}

// rgba returns the window's last frame as RGBA8 rows (device px).
pub fn (h &PixelHarness) rgba() ![]u8 {
	w := h.window()
	return w.surface.read_rgba()
}

pub fn (h &PixelHarness) save_png(path string) ! {
	w := h.window()
	px := w.surface.read_rgba()!
	stbi.stbi_write_png(path, w.surface.width(), w.surface.height(), 4, px.data,
		w.surface.width() * 4)!
}
