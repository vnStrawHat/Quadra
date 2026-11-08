// Runner of the comparison scenes (docs/benchmarks.md), Windows only. It starts each app
// with a scene, reads what it says on standard output and measures the whole process from
// outside: CPU time per frame, peak private working set, idle memory and CPU, startup.
//   run [-runs 5] [-out file.json] name=app.exe [name=app.exe ...]
module main

import json
import os
import time

struct Series {
	median f64
	low    f64
	high   f64
}

fn series_of(v []f64) Series {
	mut s := v.clone()
	s.sort()
	m := if s.len % 2 == 0 { (s[s.len / 2 - 1] + s[s.len / 2]) / 2 } else { s[s.len / 2] }
	return Series{m, s[0], s.last()}
}

struct AppResult {
	size   u64
	scenes map[string]map[string]Series
}

struct Results {
mut:
	date       string
	machine    string
	runs       int
	frames     int
	frameworks map[string]AppResult
}

struct Done {
	frames int @[json: 'frames']
	wall   i64 @[json: 'wallUs']
	p50    i64 @[json: 'p50Us']
	p99    i64 @[json: 'p99Us']
	max    i64 @[json: 'maxUs']
}

const animated = ['list', 'grid', 'document']

fn run_counter(path string, idle_ms int) !map[string]f64 {
	began := time.sys_mono_now()
	mut p := start(path, 'counter')!
	defer {
		p.stop()
	}
	shown := spawn wait_window(p.pid, began)
	p.wait_line('ready')!
	drawn := time.sys_mono_now() - began
	on_screen := shown.wait()
	if on_screen == 0 {
		return error('no window came on screen')
	}
	startup := if drawn > on_screen { drawn } else { on_screen }
	time.sleep(idle_ms * time.millisecond)
	before := p.usage()
	time.sleep(idle_ms * time.millisecond)
	after := p.usage()
	return {
		'startupMs':       f64(startup) / 1e6
		'idleMemory':      f64(after.private_ws)
		'idleCpuMsPerSec': f64(after.cpu_100ns - before.cpu_100ns) / 1e4 / (f64(idle_ms) / 1000)
	}
}

fn run_animated(path string, scene string, frames int) !map[string]f64 {
	mut p := start(path, scene)!
	defer {
		p.stop()
	}
	sampler := spawn sample_peak(p.handle, &p.stopped)
	p.wait_line('start')!
	before := p.usage()
	line := p.wait_line('done')!
	after := p.usage()
	p.stopped = true
	peak := sampler.wait()
	d := json.decode(Done, line.all_after('done '))!
	return {
		'cpuMsPerFrame': f64(after.cpu_100ns - before.cpu_100ns) / 1e4 / f64(frames)
		'fps':           f64(d.frames) / (f64(d.wall) / 1e6)
		'p50Ms':         f64(d.p50) / 1000
		'p99Ms':         f64(d.p99) / 1000
		'maxMs':         f64(d.max) / 1000
		'peakMemory':    f64(peak)
	}
}

fn add(mut samples map[string]map[string][]f64, scene string, m map[string]f64) {
	if scene !in samples {
		samples[scene] = map[string][]f64{}
	}
	for k, v in m {
		samples[scene][k] << v
	}
}

fn main() {
	mut runs := 5
	mut out := ''
	mut apps := map[string]string{}
	mut names := []string{}
	mut i := 1
	for i < os.args.len {
		a := os.args[i]
		if a == '-runs' {
			runs = os.args[i + 1].int()
			i++
		} else if a == '-out' {
			out = os.args[i + 1]
			i++
		} else if a.contains('=') {
			names << a.all_before('=')
			apps[a.all_before('=')] = os.real_path(a.all_after('='))
		}
		i++
	}
	frames := 600
	mut samples := map[string]map[string]map[string][]f64{}
	for n in names {
		samples[n] = map[string]map[string][]f64{}
		// One untimed start so the first measured one is not a cold disk read.
		mut p := start(apps[n], 'counter') or { panic(err) }
		p.wait_line('ready') or { panic('${n}: ${err}') }
		p.stop()
	}
	for round in 0 .. runs {
		for n in names {
			m := run_counter(apps[n], 5000) or { panic('${n} counter: ${err}') }
			eprintln('round ${round + 1} ${n} counter: ${m}')
			add(mut samples[n], 'counter', m)
		}
		for scene in animated {
			for n in names {
				m := run_animated(apps[n], scene, frames) or { panic('${n} ${scene}: ${err}') }
				eprintln('round ${round + 1} ${n} ${scene}: ${m}')
				add(mut samples[n], scene, m)
			}
		}
	}
	mut res := Results{
		date:    time.utc().format_rfc3339()
		machine: machine()
		runs:    runs
		frames:  frames
	}
	for n in names {
		mut scenes := map[string]map[string]Series{}
		for scene, measures in samples[n] {
			scenes[scene] = map[string]Series{}
			for k, v in measures {
				scenes[scene][k] = series_of(v)
			}
		}
		res.frameworks[n] = AppResult{os.file_size(apps[n]), scenes}
	}
	s := json.encode_pretty(res)
	if out == '' {
		println(s)
	} else {
		os.write_file(out, s + '\n') or { panic(err) }
	}
}
