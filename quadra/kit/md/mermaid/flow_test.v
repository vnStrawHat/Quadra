module mermaid

import quadra.prim
import quadra.text
import quadra.text.fake

fn flow(src string) Flow {
	d := diagram(src)
	assert d.error() == none, d.error() or { '' }
	assert d.kind() == 'flowchart'
	return d.flow
}

fn test_node_shapes() {
	f :=
		flow('graph TD\na[Rect]\nb(Round)\nc([Stadium])\nd((Circle))\ne{Diamond}\nf{{Hex}}\ng[[Sub]]\nh[(DB)]\ni>Flag]\nj["Quoted [x]"]\nk')
	shapes := f.nodes.map(it.shape)
	assert shapes == [Shape.rect, .round, .stadium, .circle, .diamond, .hexagon, .subroutine,
		.cylinder, .flag, .rect, .rect]
	assert f.nodes[0].label == 'Rect'
	assert f.nodes[9].label == 'Quoted [x]'
	assert f.nodes[10].label == 'k'
}

fn test_links() {
	f :=
		flow('flowchart LR\nA --> B\nB --- C\nC -.-> D\nD ==> E\nE --o F\nF --x G\nG <--> H\nH --->|long| I\nI -- text --> J\nJ -. dots .-> K')
	assert f.dir == 'LR'
	e := f.edges
	assert e.len == 10
	assert e[0].head == .arrow && e[0].stroke == .solid
	assert e[1].head == .none
	assert e[2].stroke == .dotted && e[2].head == .arrow
	assert e[3].stroke == .thick
	assert e[4].head == .circle && e[5].head == .cross
	assert e[6].head == .arrow && e[6].tail == .arrow
	assert e[7].label == 'long' && e[7].minlen == 2
	assert e[8].label == 'text' && e[8].head == .arrow && e[8].minlen == 1
	assert e[9].label == 'dots' && e[9].stroke == .dotted
}

fn test_chains_groups_and_comments() {
	f :=
		flow('graph TD\n%% a comment\nA --> B --> C; C --> D\na1 & a2 --> b1 & b2\nclassDef x fill:#f00\nstyle A fill:#fff\nA:::x --> E')
	assert f.edges.len == 3 + 4 + 1
	assert f.edges[1].from == f.ids['B'] && f.edges[1].to == f.ids['C']
	assert f.edges.filter(it.from == f.ids['a2'] && it.to == f.ids['b2']).len == 1
}

fn test_subgraphs() {
	f :=
		flow('graph TD\nsubgraph one [First group]\nA --> B\nsubgraph inner\nC\nend\nend\nsubgraph Two words\nD\nend\nB --> D')
	assert f.subs.len == 3
	assert f.subs[0].title == 'First group'
	assert f.subs[1].parent == 0
	assert f.subs[2].title == 'Two words'
	assert f.nodes[f.ids['A']].sub == 0
	assert f.nodes[f.ids['C']].sub == 1
	assert f.nodes[f.ids['D']].sub == 2
}

fn test_errors_and_kinds() {
	assert diagram('graph TD\nA -->').error()?.contains('line 2')
	d := diagram('pie title Pets\n"Dogs" : 3')
	assert d.kind() == 'pie'
	assert d.error()?.contains('not supported')
	assert diagram('').error()? == 'empty diagram'
	assert diagram('%% hi\n\nsequenceDiagram\nA->>B: hi').kind() == 'sequence'
}

fn measure() Measure {
	return Measure{
		ts:   text.new_system(fake.new())
		base: text.TextStyle{
			size: 14
		}
	}
}

// boxes are the node rects of a laid-out flowchart, in node order.
fn node_boxes(d Drawing, n int) []prim.Rect {
	mut out := []prim.Rect{}
	for op in d.ops {
		if out.len == n {
			break
		}
		if (op.kind == .box && op.bg == .panel) || op.kind == .fill {
			out << if op.kind == .box { op.rect } else { bbox(op.pts) }
		}
	}
	return out
}

fn bbox(pts []prim.Point) prim.Rect {
	mut x0, mut y0, mut x1, mut y1 := pts[0].x, pts[0].y, pts[0].x, pts[0].y
	for q in pts {
		x0, y0 = prim.min(x0, q.x), prim.min(y0, q.y)
		x1, y1 = prim.max(x1, q.x), prim.max(y1, q.y)
	}
	return prim.rect(x0, y0, x1 - x0, y1 - y0)
}

const sample = 'graph TD
A[Start] --> B{Is it?}
B -->|Yes| C[OK]
C --> D[Rethink]
D --> B
B ---->|No| E[End]
subgraph work [Work]
C
D
end
F((Hub)) --> A & C & E'

fn test_layout_has_no_overlaps_and_ranks_follow_edges() {
	f := flow(sample)
	mut m := measure()
	// Nodes are drawn last, one shape each, in node order.
	d := layout_flow(&f, mut m)
	shapes := d.ops.filter((it.kind == .box && it.bg == .panel) || it.kind == .fill)
	assert shapes.len == f.nodes.len
	boxes := shapes.map(if it.kind == .box { it.rect } else { bbox(it.pts) })
	for i in 0 .. boxes.len {
		for j in i + 1 .. boxes.len {
			assert !boxes[i].intersects(boxes[j]), 'nodes ${i} and ${j} overlap'
		}
	}
	// Top-down: every edge of the DAG part goes down.
	for e in f.edges {
		if e.from != f.ids['D'] { // D --> B is the back edge of the cycle
			assert boxes[e.to].y > boxes[e.from].y, '${f.nodes[e.from].id} -> ${f.nodes[e.to].id}'
		}
	}
	assert d.width > 0 && d.height > 0
}

fn test_edges_end_on_node_borders() {
	f := flow('graph TD\nA[One] --> B[Two]\nA --> C((Three))\nC --> D{Four}')
	mut m := measure()
	d := layout_flow(&f, mut m)
	shapes := d.ops.filter((it.kind == .box && it.bg == .panel) || it.kind == .fill)
	boxes := shapes.map(if it.kind == .box { it.rect } else { bbox(it.pts) })
	heads := d.ops.filter(it.kind == .head)
	assert heads.len == 3
	for k, h in heads {
		tip := h.pts[0]
		b := boxes[f.edges[k].to]
		// On the outline: within 1.5 px of the box, not deep inside it.
		assert tip.x >= b.x - 1.5 && tip.x <= b.right() + 1.5, 'tip ${tip} box ${b} boxes ${boxes}'
		assert tip.y >= b.y - 1.5 && tip.y <= b.y + 6, 'tip ${tip} box ${b}'
	}
}

fn test_lr_orientation() {
	f := flow('graph LR\nA --> B --> C')
	mut m := measure()
	d := layout_flow(&f, mut m)
	b := node_boxes(d, 3)
	assert b[1].x > b[0].right() && b[2].x > b[1].right()
	assert prim.max(b[0].y, b[1].y) - prim.min(b[0].y, b[1].y) < 1 // a straight row
	f2 := flow('graph RL\nA --> B')
	d2 := layout_flow(&f2, mut m)
	b2 := node_boxes(d2, 2)
	assert b2[1].right() < b2[0].x
}
