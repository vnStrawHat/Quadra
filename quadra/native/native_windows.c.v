// Default backends for Windows: Win32 windows, D3D11, DirectWrite.
module native

import quadra.core
import quadra.gpu.d3d11
import quadra.platform.win
import quadra.text.dwrite

@[params]
pub struct Options {
pub:
	force_warp bool
	gpu_debug  bool
}

pub fn backends(opts Options) !core.Backends {
	// The device starts first, on a worker thread: its driver start-up overlaps with the
	// rest of start-up (platform, fonts, window, first layout).
	// The first frames draw on WARP (about 10 ms to start) until the hardware device is up.
	device := d3d11.new_backend_async(
		force_warp: opts.force_warp
		warp_first: true
		debug:      opts.gpu_debug
	)
	return core.Backends{
		platform: win.new_platform()!
		device:   device
		text:     dwrite.new_backend()!
	}
}

// working_set_bytes reports the process's memory use (startup measurements).
pub fn working_set_bytes() u64 {
	return win.working_set_bytes()
}

// process_stats reports CPU time (100 ns units), working set and private working set
// (bytes) for live diagnostics such as the gallery's performance overlay.
pub fn process_stats() (u64, u64, u64) {
	return win.process_stats()
}
