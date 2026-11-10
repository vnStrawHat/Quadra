// Size caps of flowcharts and sequence diagrams (specs/21): a diagram past them is a parse error, shown as its
// source, so a document cannot make the layout allocate without bound.
module mermaid

const max_flow_nodes = 1000
const max_flow_edges = 500 // `a & b --> c & d` makes every pair; Mermaid's own default too
const max_flow_subgraphs = 100
const max_flow_dummies = 5000 // the nodes long edges are split into, one per rank crossed
const max_link_len = 64 // ranks one link may ask for (`a ------> b`)
const max_seq_actors = 200
const max_seq_items = 5000 // messages, notes and frame lines

fn link_len(n int) int {
	return if n < 1 {
		1
	} else if n > max_link_len {
		max_link_len
	} else {
		n
	}
}

// add_edges links every node of cur to every node of next, within max_flow_edges.
fn (mut f Flow) add_edges(cur []int, next []int, l Link) ! {
	if f.edges.len + cur.len * next.len > max_flow_edges {
		return error('more than ${max_flow_edges} links')
	}
	for a in cur {
		for b in next {
			f.edges << FEdge{a, b, l.label, l.stroke, l.head, l.tail, l.minlen}
		}
	}
}

// check_size fails a parsed flowchart past the caps, before any layout runs: the dummy count
// comes from the same ranking the layout uses (layers), which is linear.
fn check_size(f &Flow) ! {
	if f.nodes.len > max_flow_nodes {
		return error('more than ${max_flow_nodes} nodes')
	}
	if f.subs.len > max_flow_subgraphs {
		return error('more than ${max_flow_subgraphs} subgraphs')
	}
	rev := back_edges(f)
	rank := rank_nodes(f, rev)
	mut n := 0
	for k, e in f.edges {
		a, b := if rev[k] { e.to, e.from } else { e.from, e.to }
		if rank[b] - rank[a] > 1 {
			n += rank[b] - rank[a] - 1
		}
		if n > max_flow_dummies {
			return error('the diagram is too large to lay out')
		}
	}
}

fn check_seq_size(s &Seq) ! {
	if s.actors.len > max_seq_actors {
		return error('more than ${max_seq_actors} participants')
	}
	if s.items.len > max_seq_items {
		return error('more than ${max_seq_items} messages, notes and frames')
	}
}
