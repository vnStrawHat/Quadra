// OS services for applications: process facts, crash capture, agent transports and file
// operations the standard library lacks. One file per concern on Windows, one stub for the rest.
module sys

// ProcessCounters are this process's figures for a resource indicator.
pub struct ProcessCounters {
pub mut:
	cpu_time_100ns u64
	threads        int // -1: unknown
	private_ws     u64
	working_set    u64
	commit         u64
	peak           u64
	uptime_ms      u64 // since the process started; 0: unknown
}

// ChildProcess is a process this one started, directly or through its children.
pub struct ChildProcess {
pub:
	pid  u32
	name string
}

// LoadedModule is a DLL or the executable in this process, with its image size.
pub struct LoadedModule {
pub:
	name string
	size u32
}

// busiest_engine reads PDH `GPU Engine` instances (`pid_4242_luid_0x0_0xBEEF_phys_0_eng_1_engtype_3D`):
// the busiest single engine of `pid` on any GPU, clamped to 0-100, which is how Task Manager
// defines its GPU column. None when the process has no engine.
pub fn busiest_engine(pid u32, names []string, values []f64) ?f32 {
	owner := 'pid_${pid}_'
	mut best := f32(-1)
	for i, name in names {
		if i < values.len && name.starts_with(owner) {
			mut v := values[i]
			if v < 0 {
				v = 0
			} else if v > 100 {
				v = 100
			}
			if f32(v) > best {
				best = f32(v)
			}
		}
	}
	if best < 0 {
		return none
	}
	return best
}
