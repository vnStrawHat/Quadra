// The Editor page: the code editor on a V sample and a YAML manifest.
module pages

import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.ui

pub struct EditorPage {
mut:
	v    core.View[code.Editor]
	yaml core.View[code.Editor]
}

const editor_v_sample = 'module main

import os

// Point is a position on the grid.
@[heap]
struct Point {
mut:
	x int
	y int = 1
}

fn (p Point) str() string {
	return "(\${p.x}, \${p.y})"
}

fn main() {
	mut pts := []Point{len: 3, init: Point{index, index * 2}}
	for i, p in pts {
		if i % 2 == 0 && p.x > 0x10 {
			continue
		}
		println(\'point \${i}: \${p}\n\')
	}
	/* nested /* comments */ work */
	os.write_file(\'out.txt\', pts.str()) or { panic(err) }
}
'

const editor_yaml_sample = '# A deployment, as kubectl would print it.
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  labels: {app: web, tier: "frontend"}
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: web
          image: registry.local/web:1.4.2
          args: [--port, 8080, --verbose]
          env:
            - {name: DEBUG, value: "false"}
          command: &cmd
            - /bin/sh
          script: |
            echo starting # still part of the script
            exec web
      restartPolicy: Always
'

@[markused]
pub fn (mut p EditorPage) render(mut cx core.ViewCx[EditorPage]) core.Element {
	t := kit.theme(cx)
	if p.v.id == 0 {
		p.v = code.new_editor(mut cx.Cx, id: 'demo-editor', text: editor_v_sample, path: 'main.v')
		p.yaml = code.new_editor(mut cx.Cx,
			id:   'demo-yaml'
			text: editor_yaml_sample
			path: 'deploy.yaml'
		)
	}
	return page('Editor',
		'A code editor: piece table buffer, multiple cursors, grouped undo, lazy highlighting for V and YAML, find and replace. Try ctrl-d, alt-up/down, ctrl-alt-down, ctrl-/, alt-drag, ctrl-f and ctrl-h.', [
		demo('V', framed(t, ui.div().h(380).child(cx.child_view(p.v))),
			"code.new_editor(mut cx, text: src, path: 'main.v')"),
		demo('YAML', framed(t, ui.div().h(300).child(cx.child_view(p.yaml))),
			"code.new_editor(mut cx, text: manifest, path: 'deploy.yaml')"),
	])
}
