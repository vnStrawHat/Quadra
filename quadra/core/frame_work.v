// The work functions of closure lifetime frames (tracked.v).
module core

// FrameWork is what the work functions handed to Lifetime.frame run: plain functions, not a
// closure per window or app (a window's would keep the closed window alive, V frees no
// closure). Frames run on the main thread only, one at a time.
struct FrameWork {
mut:
	t &Tracking  = unsafe { nil }
	j &RenderJob = unsafe { nil }
}

const frame_work = &FrameWork{}

fn run_build() {
	mut t := frame_work.t
	if mut p := t.pending {
		t.out = p.build_tree(mut t.cx)
	}
}

fn run_render() {
	mut j := frame_work.j
	mut a := j.app
	j.out = j.render(mut a, j.win, j.id)
}
