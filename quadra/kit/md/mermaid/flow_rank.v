// Flowchart layering: cycles broken by reversing DFS back edges, ranks by longest path,
// long edges split by dummy nodes, and the order inside each rank by barycenter sweeps with
// adjacent swaps, keeping each subgraph's nodes together.
module mermaid

struct LNode {
mut:
	real  int = -1 // index into Flow.nodes; -1 = a dummy on edge `edge`
	edge  int = -1
	rank  int
	order int
	w     f32 // size across the ranks
	h     f32 // size along the rank axis
	pos   f32 // center across the ranks
	up    []int
	down  []int
	path  []int // subgraphs containing it, outermost first
}

struct Layers {
mut:
	n      []LNode
	rows   [][]int // node indexes by rank, in order
	chains [][]int // per edge: its nodes from the layered source to the layered target
	rev    []bool  // per edge: laid out reversed (a back edge of a cycle)
}

// back_edges marks the edges a DFS finds going back to a node on its stack.
fn back_edges(f &Flow) []bool {
	mut out := [][]int{len: f.nodes.len}
	for k, e in f.edges {
		out[e.from] << k
	}
	mut state := []u8{len: f.nodes.len} // 0 new, 1 on stack, 2 done
	mut rev := []bool{len: f.edges.len}
	for s in 0 .. f.nodes.len {
		if state[s] == 0 {
			dfs_back(f, out, s, mut state, mut rev)
		}
	}
	return rev
}

fn dfs_back(f &Flow, out [][]int, v int, mut state []u8, mut rev []bool) {
	state[v] = 1
	for k in out[v] {
		w := f.edges[k].to
		if state[w] == 1 {
			rev[k] = true
		} else if state[w] == 0 {
			dfs_back(f, out, w, mut state, mut rev)
		}
	}
	state[v] = 2
}

// rank_nodes gives each node a rank (every edge spans 2 * minlen, so the middle rank can
// hold the edge's label), longest path from the sources, then sources moved down next to
// their first successor.
fn rank_nodes(f &Flow, rev []bool) []int {
	n := f.nodes.len
	mut indeg := []int{len: n}
	mut succ := [][]int{len: n}
	mut pred := [][]int{len: n}
	for k, e in f.edges {
		if e.from == e.to {
			continue
		}
		a, b := if rev[k] { e.to, e.from } else { e.from, e.to }
		succ[a] << k
		pred[b] << k
		indeg[b]++
	}
	mut rank := []int{len: n}
	mut queue := []int{}
	for v in 0 .. n {
		if indeg[v] == 0 {
			queue << v
		}
	}
	mut topo := []int{cap: n}
	for queue.len > 0 {
		v := queue[0]
		queue.delete(0)
		topo << v
		for k in succ[v] {
			e := f.edges[k]
			w := if rev[k] { e.from } else { e.to }
			rank[w] = if rank[v] + 2 * e.minlen > rank[w] { rank[v] + 2 * e.minlen } else { rank[w] }
			indeg[w]--
			if indeg[w] == 0 {
				queue << w
			}
		}
	}
	for i := topo.len - 1; i >= 0; i-- {
		v := topo[i]
		if pred[v].len > 0 || succ[v].len == 0 {
			continue
		}
		mut best := 1 << 30
		for k in succ[v] {
			e := f.edges[k]
			w := if rev[k] { e.from } else { e.to }
			best = if rank[w] - 2 * e.minlen < best { rank[w] - 2 * e.minlen } else { best }
		}
		rank[v] = best
	}
	mut lo := 1 << 30
	for r in rank {
		lo = if r < lo { r } else { lo }
	}
	return rank.map(it - lo)
}

fn sub_path(f &Flow, sub int) []int {
	mut p := []int{}
	mut s := sub
	for s >= 0 {
		p.insert(0, s)
		s = f.subs[s].parent
	}
	return p
}

fn common(a []int, b []int) int {
	mut i := 0
	for i < a.len && i < b.len && a[i] == b[i] {
		i++
	}
	return i
}

// layers builds the layered graph; sizes are filled in by the caller.
fn layers(f &Flow) Layers {
	rev := back_edges(f)
	rank := rank_nodes(f, rev)
	mut g := Layers{
		rev:    rev
		chains: [][]int{len: f.edges.len}
	}
	mut top := 0
	for v, nd in f.nodes {
		g.n << LNode{
			real: v
			rank: rank[v]
			path: sub_path(f, nd.sub)
		}
		top = if rank[v] > top { rank[v] } else { top }
	}
	for k, e in f.edges {
		if e.from == e.to {
			continue
		}
		a, b := if rev[k] { e.to, e.from } else { e.from, e.to }
		pa, pb := g.n[a].path, g.n[b].path
		mut chain := [a]
		for r in rank[a] + 1 .. rank[b] {
			g.n << LNode{
				edge: k
				rank: r
				path: pa[..common(pa, pb)].clone()
			}
			chain << g.n.len - 1
		}
		chain << b
		for i in 0 .. chain.len - 1 {
			g.n[chain[i]].down << chain[i + 1]
			g.n[chain[i + 1]].up << chain[i]
		}
		g.chains[k] = chain
	}
	g.rows = [][]int{len: top + 1}
	g.init_order(f)
	return g
}

// init_order seeds each rank in DFS order from the sources, in source order.
fn (mut g Layers) init_order(f &Flow) {
	mut seen := []bool{len: g.n.len}
	for v in 0 .. f.nodes.len {
		if g.n[v].up.len == 0 {
			g.visit(v, mut seen)
		}
	}
	for v in 0 .. g.n.len {
		g.visit(v, mut seen)
	}
}

fn (mut g Layers) visit(v int, mut seen []bool) {
	if seen[v] {
		return
	}
	seen[v] = true
	g.n[v].order = g.rows[g.n[v].rank].len
	g.rows[g.n[v].rank] << v
	for w in g.n[v].down.clone() {
		g.visit(w, mut seen)
	}
}
