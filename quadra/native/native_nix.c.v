// Linux and macOS backends are not implemented yet (see specs/13-apps-roadmap.md).
// Headless backends from quadra.testing work on every OS.
module native

import quadra.core

@[params]
pub struct Options {
pub:
	force_warp bool
	gpu_debug  bool
}

pub fn backends(opts Options) !core.Backends {
	return error('quadra: no native backends for this OS yet')
}

pub fn working_set_bytes() u64 {
	return 0 // ponytail: read /proc/self/statm when the Linux backend lands
}

pub fn process_stats() (u64, u64, u64) {
	return 0, 0, 0 // ponytail: getrusage and /proc/self/status when the Linux backend lands
}
