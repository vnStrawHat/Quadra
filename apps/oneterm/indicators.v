// The status bar's sampled indicators: git, network and CPU/MEM, off the UI thread (specs/22).
module main

import os
import quadra.core
import quadra.kit
import quadra.ui

// GitLabel is the git indicator: `main*  (+1 -2) ↑3`.
pub struct GitLabel {
pub:
	head    string // '' = hidden
	added   u64
	removed u64
	tail    string // ` ↑2 ↓1`
}

pub fn (l GitLabel) whole() string {
	if l.head == '' {
		return ''
	}
	diff := if l.added > 0 || l.removed > 0 { ' (+${l.added} -${l.removed})' } else { '' }
	return l.head + diff + l.tail
}

fn (l GitLabel) element(budget f32, measure Measure, t &kit.Theme) core.Element {
	if measure.width(l.whole()) > budget {
		return ui.text(elide_right(l.whole(), budget, measure))
	}
	row := ui.div().flex_row().items_center().child(ui.text(l.head))
	if l.added > 0 || l.removed > 0 {
		row.child(ui.text(' (')).child(ui.text('+${l.added}').color(t.p.ok)).child(ui.text(' ')).child(ui.text('-${l.removed}').color(t.p.error)).child(ui.text(')'))
	}
	if l.tail != '' {
		row.child(ui.text(l.tail))
	}
	return row
}

// build_git_label reads `git status --porcelain=v2 --branch` output.
pub fn build_git_label(status string, added u64, removed u64) GitLabel {
	mut head, mut oid, mut ab := '', '', ''
	mut dirty := false
	for line in status.split_into_lines() {
		if line.starts_with('# ') {
			h := line[2..]
			if h.starts_with('branch.head ') {
				head = h.all_after('branch.head ')
			} else if h.starts_with('branch.oid ') {
				oid = h.all_after('branch.oid ')
			} else if h.starts_with('branch.ab ') {
				ab = h.all_after('branch.ab ')
			}
		} else if line != '' {
			dirty = true
		}
	}
	if head == '' {
		return GitLabel{}
	}
	mut name := head
	if head == '(detached)' {
		name = if oid != '' && oid != '(initial)' { oid.substr_ni(0, 7) } else { 'HEAD' }
	}
	if dirty {
		name += '*'
	}
	mut tail := ''
	if ab != '' {
		parts := ab.split(' ')
		ahead := if parts.len > 0 { parts[0].trim_left('+-').u64() } else { u64(0) }
		behind := if parts.len > 1 { parts[1].trim_left('+-').u64() } else { u64(0) }
		if ahead > 0 {
			tail += ' ↑${ahead}'
		}
		if behind > 0 {
			tail += ' ↓${behind}'
		}
	}
	return GitLabel{name, added, removed, tail}
}

// parse_numstat sums `git diff --numstat` (binary entries show `-` and count as zero).
pub fn parse_numstat(output string) (u64, u64) {
	mut a, mut r := u64(0), u64(0)
	for line in output.split_into_lines() {
		c := line.split('\t')
		if c.len >= 2 {
			a += c[0].u64()
			r += c[1].u64()
		}
	}
	return a, r
}

// run_git runs git in `cwd` as its working folder (the path never reaches a command line).
fn run_git(exe string, cwd string, args []string) ?string {
	mut p := os.new_process(exe)
	mut full := ['--no-optional-locks']
	full << args
	p.set_args(full)
	p.set_work_folder(cwd)
	p.set_redirect_stdio()
	p.run()
	out := p.stdout_slurp()
	p.wait()
	code := p.code
	p.close()
	return if code == 0 { out } else { none }
}

struct GitResult {
	cwd   string
	label GitLabel
}

fn git_query(cwd string) GitResult {
	exe := os.find_abs_path_of_executable('git') or { return GitResult{
		cwd: cwd
	} }
	if !os.is_dir(cwd) {
		return GitResult{
			cwd: cwd
		}
	}
	status := run_git(exe, cwd, ['status', '--porcelain=v2', '--branch']) or {
		return GitResult{
			cwd: cwd
		}
	}
	diff := run_git(exe, cwd, ['diff', '--numstat', 'HEAD']) or { '' }
	a, r := parse_numstat(diff)
	return GitResult{cwd, build_git_label(status, a, r)}
}

pub struct GitState {
pub mut:
	label GitLabel
mut:
	cwd   string // what `label` is for; a new cwd clears it
	busy  bool
	since int // seconds since the last run
}

@[heap]
pub struct ResourceBox {
pub mut:
	text   string // 'CPU 1.2%  MEM 45.2 MB'
	sample ResourceSample
	cpu    f64
}

pub struct ResourceSample {
pub mut:
	cpu_time_100ns u64
	cores          int
	threads        int // -1: unknown
	private_ws     u64
	working_set    u64
	commit         u64
	peak           u64
	uptime_ms      u64
	at_ms          f64
}

pub fn format_memory(b u64) string {
	if b >= 1073741824 {
		return '${f64(b) / 1073741824:.2f} GB'
	}
	if b >= 1048576 {
		return '${f64(b) / 1048576:.1f} MB'
	}
	if b >= 1024 {
		return '${f64(b) / 1024:.1f} KB'
	}
	return '${b} B'
}

pub fn format_duration(ms u64) string {
	s := ms / 1000
	if s < 60 {
		return '${f64(ms) / 1000:.1f} s'
	}
	if s < 3600 {
		return '${s / 60}m ${s % 60:02}s'
	}
	return '${s / 3600}h ${s % 3600 / 60:02}m'
}

pub fn format_speed(bps f64) string {
	if bps < 1000 {
		return '${u64(bps + 0.5)} bps'
	}
	if bps < 1000000 {
		return '${bps / 1000:.1f} Kbps'
	}
	if bps < 1000000000 {
		return '${bps / 1000000:.1f} Mbps'
	}
	return '${bps / 1000000000:.2f} Gbps'
}

fn (mut o OneTerm) tick_indicators(mut cx core.ViewCx[OneTerm]) {
	o.tick_git(mut cx)
	o.tick_net()
	o.tick_resources(mut cx)
}

fn (mut o OneTerm) tick_git(mut cx core.ViewCx[OneTerm]) {
	cwd := if s := o.current().active_space() {
		if s.remote { '' } else { o.active_cwd(mut cx) }
	} else {
		''
	}
	if cwd != o.git.cwd {
		o.git.cwd, o.git.label, o.git.since = cwd, GitLabel{}, 2
	}
	o.git.since++
	if cwd == '' || o.git.busy || o.git.since < 2 {
		return
	}
	o.git.busy, o.git.since = true, 0
	core.spawn_task_with[OneTerm, string, GitResult](mut cx, cwd, fn (cwd string, _ &core.TaskCx) !GitResult {
		return git_query(cwd)
	}, fn (mut o OneTerm, r &core.TaskResult[GitResult], mut cx core.ViewCx[OneTerm]) {
		o.git.busy = false
		if r.ok && r.value.cwd == o.git.cwd { // a slow answer for a cwd already left is dropped
			o.git.label = r.value.label
			cx.notify()
		}
	})
}

fn (mut o OneTerm) tick_net() {
	s := o.current().active_space() or {
		o.net_text, o.net_last = '', none
		return
	}
	stats := net_counters(s) or {
		o.net_text, o.net_last = '', none
		return
	}
	mut drx, mut dtx := u64(0), u64(0)
	if prev := o.net_last {
		drx = if stats.rx_bytes > prev.rx_bytes { stats.rx_bytes - prev.rx_bytes } else { u64(0) }
		dtx = if stats.tx_bytes > prev.tx_bytes { stats.tx_bytes - prev.tx_bytes } else { u64(0) }
	}
	o.net_last = stats
	o.net_text = '↓ ${format_speed(f64(drx) * 8)}  ↑ ${format_speed(f64(dtx) * 8)}'
}
