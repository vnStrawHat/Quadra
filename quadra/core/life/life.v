// Thin wrapper over V's closure lifetimes, used by quadra.core (tracked.v). It is its own module
// because vfmt 0.5.2 drops `import builtin.closure` from quadra.core's files.
module life

import builtin.closure

@[heap]
pub struct Life {
mut:
	l closure.Lifetime
}

pub fn new() &Life {
	return &Life{}
}

// frame runs work; closures it creates belong to this lifetime until reclaimed.
pub fn (mut x Life) frame(work fn ()) ! {
	x.l.frame(work)!
}

// reclaim frees the closures of all frames but the newest `keep`.
pub fn (mut x Life) reclaim(keep int) {
	x.l.reclaim(keep) or {}
}

// dispose frees every closure of this lifetime.
pub fn (mut x Life) dispose() {
	x.l.dispose() or {}
}

// live_closures counts the closures alive in the process (leak probes).
pub fn live_closures() int {
	return closure.live_closures()
}

// trace prints a backtrace at every closure creation while on (QUADRA_TRACE_CLOSURES).
pub fn trace(on bool) {
	closure.trace(on)
}
