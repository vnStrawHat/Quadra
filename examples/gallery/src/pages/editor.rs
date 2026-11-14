//! The code editor: a V sample and a YAML manifest.

use quadra::{Div, column};
use quadra_code::editor;

use crate::common::{demo, framed, page};

const V_SAMPLE: &str = r#"module main

import os

// Point is a position on the grid.
@[heap]
struct Point {
mut:
	x int
	y int = 1
}

fn (p Point) str() string {
	return "(${p.x}, ${p.y})"
}

fn main() {
	mut pts := []Point{len: 3, init: Point{index, index * 2}}
	for i, p in pts {
		if i % 2 == 0 && p.x > 0x10 {
			continue
		}
		println('point ${i}: ${p}\n')
	}
	/* nested /* comments */ work */
	os.write_file('out.txt', pts.str()) or { panic(err) }
}
"#;

const YAML_SAMPLE: &str = r#"# A deployment, as kubectl would print it.
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
"#;

pub fn editor_page() -> Div {
    let about = "A code editor: a buffer of pieces, several cursors, undo by groups, colours read a \
                 line at a time as far as the view shows. Try ctrl-d, alt-up/down, ctrl-alt-down, \
                 ctrl-/ and alt-drag.";
    page(
        "Editor",
        about,
        [
            demo(
                "V",
                framed(column().height(380.0).child(editor(V_SAMPLE).path("main.v"))),
                "editor(source).path(\"main.v\")",
            ),
            demo(
                "YAML",
                framed(column().height(300.0).child(editor(YAML_SAMPLE).path("deploy.yaml"))),
                "editor(manifest).path(\"deploy.yaml\")",
            ),
        ],
    )
}
