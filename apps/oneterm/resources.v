// The status bar's CPU/MEM indicator and its hover table (specs/22).
module main

import runtime
import quadra.core
import quadra.kit
import quadra.native.sys
import quadra.ui

// sample_process reads this process's memory, CPU time and threads. Run off the UI thread.
pub fn sample_process() ResourceSample {
	p := sys.process_counters()
	return ResourceSample{
		cpu_time_100ns: p.cpu_time_100ns
		cores:          runtime.nr_cpus()
		threads:        p.threads
		private_ws:     p.private_ws
		working_set:    p.working_set
		commit:         p.commit
		peak:           p.peak
	}
}

fn (mut o OneTerm) tick_resources(mut cx core.ViewCx[OneTerm]) {
	o.res_ticks++
	if o.res_ticks % 2 != 0 || o.res_busy {
		return
	}
	o.res_busy = true
	now := cx.now_ms()
	core.spawn_task[OneTerm, ResourceSample](mut cx, fn [now] (_ &core.TaskCx) !ResourceSample {
		mut s := sample_process()
		s.at_ms = now
		return s
	}, fn (mut o OneTerm, r &core.TaskResult[ResourceSample], mut cx core.ViewCx[OneTerm]) {
		o.res_busy = false
		if r.ok {
			o.apply_sample(r.value)
			cx.notify()
		}
	})
}

fn (mut o OneTerm) apply_sample(s ResourceSample) {
	mut cpu := f64(0)
	if o.res_prev.at_ms > 0 && s.at_ms > o.res_prev.at_ms && s.cores > 0 {
		dcpu := f64(s.cpu_time_100ns - o.res_prev.cpu_time_100ns) / 10000 // to ms
		cpu = dcpu / (s.at_ms - o.res_prev.at_ms) / f64(s.cores) * 100
	}
	o.res_prev = s
	mem := if s.private_ws > 0 { s.private_ws } else { s.working_set }
	o.res.text = 'CPU ${cpu:.1f}%  MEM ${format_memory(mem)}'
	o.res.sample = s
	o.res.cpu = cpu
}

// resource_table is the indicator's hover table; it reads the box, so it follows each sample.
fn resource_table(box &ResourceBox, t &kit.Theme) fn () core.Element {
	theme := unsafe { t }
	return fn [box, theme] () core.Element {
		s := box.sample
		row := fn [theme] (k string, v string) &ui.Div {
			return ui.div().flex_row().items_center().gap(16).child(ui.text(k).color(theme.p.text_2)).child(ui.div().grow()).child(ui.text(v))
		}
		threads := if s.threads >= 0 { s.threads.str() } else { 'n/a' }
		mut col := ui.div().flex_col().gap(2).min_w(280).child(ui.text('Memory'))
		if s.private_ws > 0 {
			col.child(row('Private working set', format_memory(s.private_ws)))
		}
		col.child(row('Working set', format_memory(s.working_set)))
		col.child(row('Commit (private bytes)', format_memory(s.commit)))
		col.child(row('Peak working set', format_memory(s.peak)))
		col.child(ui.div().h(6)).child(ui.text('CPU'))
		col.child(row('Usage', '${box.cpu:.1f}% of ${s.cores} logical cores'))
		col.child(row('CPU time (user + kernel)', format_duration(s.cpu_time_100ns / 10000)))
		col.child(row('Threads', threads))
		col.child(row('Uptime', format_duration(u64(s.at_ms))))
		return col
	}
}
