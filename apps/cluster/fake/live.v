// The live part of the fake cluster: tick advances one second, deterministically. Ages
// grow, pods use CPU and memory and write logs, restarts, status changes and pod
// replacements happen on fixed schedules, and each of those emits events.
module fake

import math

// warm_up fills the metric history and a first page of logs, as if the pods had run.
fn (mut c Cluster) warm_up() {
	for t in -history_len .. 0 {
		c.sample(t)
	}
	for t in -40 .. 0 {
		c.write_logs(t)
	}
	c.event('Normal', 'Started', 'Pod/${c.resources[0].name}', 'Started container main')
}

fn (c &Cluster) pod_indices() []int {
	mut out := []int{}
	for i, r in c.resources {
		if r.kind == 'Pod' {
			out << i
		}
	}
	return out
}

// sample adds one CPU and memory sample for every running pod.
fn (mut c Cluster) sample(t int) {
	for i, r in c.resources {
		if r.kind != 'Pod' || r.status != 'Running' {
			continue
		}
		k := r.key()
		base := f64(20 + (i * 37) % 180)
		cpu := base * (1 + 0.35 * math.sin(f64(t) / 9 + f64(i))) + f64((t * 7 + i * 13) % 11)
		mem := 64 + f64((i * 53) % 400) + 12 * math.sin(f64(t) / 40 + f64(i) / 3)
		push(mut c.cpu, k, cpu)
		push(mut c.mem, k, mem)
	}
}

fn push(mut m map[string][]f64, k string, v f64) {
	mut h := m[k] or { []f64{} }
	if h.len >= history_len {
		h.delete(0)
	}
	h << v
	m[k] = h
}

const levels = ['INFO', 'INFO', 'INFO', 'DEBUG', 'INFO', 'WARN', 'INFO', 'INFO', 'INFO', 'ERROR',
	'INFO']

// write_logs writes a line for every third running pod (by tick), colored for the apps
// that log with ANSI escapes (web, api).
fn (mut c Cluster) write_logs(t int) {
	for i, r in c.resources {
		if r.kind != 'Pod' || r.status != 'Running' || (t + i) % 3 != 0 {
			continue
		}
		k := r.key()
		n := c.written[k] or { 0 }
		lvl := levels[(n + i) % levels.len]
		app := r.app
		tag := if app in ['web', 'api'] {
			color := match lvl {
				'WARN' { '33' }
				'ERROR' { '31' }
				'DEBUG' { '2' }
				else { '32' }
			}

			'\x1b[${color}m${lvl:-5}\x1b[0m'
		} else {
			'${lvl:-5}'
		}
		ms := (n * 37 + i) % 90 + 1
		text := '${tag} ${app}: GET /api/v1/items/${(n * 13 + i) % 500} 200 ${ms}ms'
		mut ls := c.logs[k] or { []LogLine{} }
		if ls.len >= max_logs {
			ls.delete(0)
		}
		ls << LogLine{start_ms + i64(t) * 1000 + i64(i % 1000), text}
		c.logs[k] = ls
		c.written[k] = n + 1
	}
}

fn (mut c Cluster) event(typ string, reason string, object string, message string) {
	c.next++
	if c.events.len >= max_events {
		c.events.delete(0)
	}
	c.events << Event{c.next, typ, reason, object, message, c.t}
}

// tick advances the cluster by one second.
pub fn (mut c Cluster) tick() {
	c.t++
	for mut r in c.resources {
		r.age_s++
	}
	c.sample(c.t)
	c.write_logs(c.t)
	pods := c.pod_indices()
	if pods.len == 0 {
		return
	}
	if c.t % 7 == 0 { // a running pod restarts
		i := pods[(c.t / 7 * 5) % pods.len]
		if c.resources[i].status == 'Running' {
			c.resources[i].restarts++
			c.event('Warning', 'BackOff', 'Pod/${c.resources[i].name}',
				'Back-off restarting failed container main')
		}
	}
	if c.t % 11 == 0 { // pending pods start, running ones get rescheduled
		i := pods[(c.t / 11 * 3) % pods.len]
		mut r := &c.resources[i]
		if r.status == 'Pending' {
			r.status, r.ready = 'Running', '1/1'
			c.event('Normal', 'Started', 'Pod/${r.name}', 'Started container main')
		} else if r.status == 'Running' {
			r.status, r.ready = 'Pending', '0/1'
			c.event('Normal', 'Pulling', 'Pod/${r.name}', 'Pulling image "${r.image}"')
		}
	}
	if c.t % 30 == 0 { // a pod is replaced
		i := pods[(c.t / 30 * 7) % pods.len]
		old := c.resources[i]
		app := old.app
		c.event('Normal', 'Killing', 'Pod/${old.name}', 'Stopping container main')
		c.cpu.delete(old.key())
		c.mem.delete(old.key())
		c.logs.delete(old.key())
		c.written.delete(old.key())
		mut p := new_pod(c.next, app, 'Pending')
		p.namespace, p.node, p.image = old.namespace, old.node, old.image
		c.resources[i] = p
		c.event('Normal', 'Scheduled', 'Pod/${p.name}',
			'Successfully assigned ${p.namespace}/${p.name} to ${p.node}')
	}
}

// new_logs returns the lines of pod key written after the first `seen` (and the new total).
pub fn (c &Cluster) new_logs(key string, seen int) ([]LogLine, int) {
	total := c.written[key] or { 0 }
	ls := c.logs[key] or { return []LogLine{}, total }
	n := total - seen
	if n <= 0 {
		return []LogLine{}, total
	}
	return ls[if n >= ls.len {
		0
	} else {
		ls.len - n
	}..].clone(), total
}

// apply_manifest updates a resource from an edited manifest: image and replicas.
pub fn (mut c Cluster) apply_manifest(kind string, key string, image string, replicas int) ! {
	for mut r in c.resources {
		if r.kind == kind && r.key() == key {
			if image != '' && image != r.image {
				r.image = image
				c.event('Normal', 'Updated', '${kind}/${r.name}', 'Image set to ${image}')
			}
			if kind == 'Deployment' && replicas > 0 && replicas != r.replicas {
				c.event('Normal', 'ScalingReplicaSet', '${kind}/${r.name}',
					'Scaled from ${r.replicas} to ${replicas}')
				r.replicas = replicas
				r.ready = '${replicas}/${replicas}'
			}
			return
		}
	}
	return error('${kind} ${key} not found')
}
