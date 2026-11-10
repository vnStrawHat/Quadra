// --stats (specs/22): what the active terminal did in the last second, in the status bar.
module main

import quadra.core
import quadra.kit.term

// stats_line describes one second from two readings. Paint is the terminal's CPU time building
// the scene; frame is the last whole frame of the window, GPU submission included.
pub fn stats_line(a term.Stats, b term.Stats, frame_us i64) string {
	chunks := b.chunks - a.chunks
	frames := b.frames - a.frames
	mib := f64(b.bytes - a.bytes) / 1048576
	parse := if chunks > 0 { f64(b.parse_us - a.parse_us) / f64(chunks) / 1000 } else { 0.0 }
	paint := if frames > 0 { f64(b.paint_us - a.paint_us) / f64(frames) / 1000 } else { 0.0 }
	return '${mib:.1f} MiB/s  ${chunks} chunks/s  parse ${parse:.2f} ms  ${frames} fps  paint ${paint:.2f} ms  frame ${f64(frame_us) / 1000:.2f} ms'
}

fn (mut o OneTerm) sample_stats(mut cx core.ViewCx[OneTerm]) {
	t := o.active_terminal(mut cx) or {
		o.stats_text, o.stats_view = '', 0
		return
	}
	id := o.active_view().id
	if id != o.stats_view { // another terminal: its counters start from here
		o.stats_view, o.stats_last, o.stats_text = id, t.stats, ''
		return
	}
	frame := if cx.app.windows.len > 0 { cx.app.windows[0].stats.last_frame_us } else { i64(0) }
	o.stats_text = stats_line(o.stats_last, t.stats, frame)
	eprintln('stats: ${o.stats_text}') // for scripted runs (bench/fire/run.sh)
	o.stats_last = t.stats
}
