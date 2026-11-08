// Scene snapshots: a stable text form of the composed primitives for diffable tests.
module testing

import os
import quadra.gpu

fn r1(v f32) string {
	return '${f32(int(v * 10 + if v >= 0 {
		f32(0.5)
	} else {
		f32(-0.5)
	})) / 10}'
}

fn color(c [4]f32) string {
	b := fn (v f32) string {
		return u8(if v < 0 {
			f32(0)
		} else if v > 1 {
			f32(255)
		} else {
			v * 255 + 0.5
		}).hex()
	}
	return '#${b(c[0])}${b(c[1])}${b(c[2])}${b(c[3])}'
}

// snapshot serializes the last drawn scene, one primitive per line.
pub fn (h &Harness) snapshot() string {
	scene := h.scene()
	mut lines := []string{cap: scene.instances.len}
	for i in scene.instances {
		kind := unsafe { gpu.Kind(int(i.kind)) }
		b := '${r1(i.bounds[0])},${r1(i.bounds[1])} ${r1(i.bounds[2])}x${r1(i.bounds[3])}'
		lines << match kind {
			.quad { 'quad ${b} fill ${color(i.color)} r ${r1(i.radii[0])}' }
			.shadow { 'shadow ${b} ${color(i.color2)} blur ${r1(i.params[0])}' }
			.mono { 'glyph ${b} ${color(i.color)}' }
			.poly { 'image ${b}' }
			.line { 'line ${b} ${color(i.color)}' }
		}
	}
	return lines.join('\n')
}

// match_snapshot compares with tests/snapshots/<name>.scene, writing it when missing or
// when QUADRA_UPDATE_SNAPSHOTS=1.
pub fn (h &Harness) match_snapshot(dir string, name string) ! {
	got := h.snapshot() + '\n'
	path := os.join_path(dir, '${name}.scene')
	if os.getenv('QUADRA_UPDATE_SNAPSHOTS') == '1' || !os.exists(path) {
		os.mkdir_all(dir)!
		os.write_file(path, got)!
		return
	}
	want := os.read_file(path)!
	if want != got {
		os.write_file(path + '.new', got)!
		return error('snapshot ${name} differs; see ${path}.new')
	}
}
