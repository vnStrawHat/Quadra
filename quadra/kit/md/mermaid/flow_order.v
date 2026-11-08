// Ordering inside ranks: barycenter sweeps down and up, then swaps of neighbours that
// remove crossings; the order with the fewest crossings wins. Nodes of one subgraph stay
// next to each other (they are sorted as a group, by the group's mean barycenter).
module mermaid

fn (mut g Layers) order() {
	mut best := g.rows.map(it.clone())
	mut best_c := g.crossings()
	for it in 0 .. 8 {
		down := it % 2 == 0
		if down {
			for r in 1 .. g.rows.len {
				g.sort_row(r, true)
			}
		} else {
			for r := g.rows.len - 2; r >= 0; r-- {
				g.sort_row(r, false)
			}
		}
		g.transpose()
		c := g.crossings()
		if c < best_c {
			best_c = c
			best = g.rows.map(it.clone())
		}
		if best_c == 0 {
			break
		}
	}
	g.rows = best
	for row in g.rows {
		for i, v in row {
			g.n[v].order = i
		}
	}
}

// sort_row orders rank r by the mean order of each node's neighbours in the rank above
// (down) or below; nodes without such neighbours keep their place.
fn (mut g Layers) sort_row(r int, down bool) {
	row := g.rows[r]
	mut bc := map[int]f32{}
	for v in row {
		nb := if down { g.n[v].up } else { g.n[v].down }
		if nb.len == 0 {
			bc[v] = f32(g.n[v].order)
			continue
		}
		mut s := f32(0)
		for w in nb {
			s += f32(g.n[w].order)
		}
		bc[v] = s / f32(nb.len)
	}
	g.rows[r] = g.group_sort(row, bc, 0)
	for i, v in g.rows[r] {
		g.n[v].order = i
	}
}

struct Unit {
	key   f32
	items []int
}

// group_sort sorts items by bc, keeping items of one subgraph (at this depth) together.
fn (g &Layers) group_sort(items []int, bc map[int]f32, depth int) []int {
	mut units := []Unit{}
	mut at := map[int]int{} // subgraph -> unit index
	for v in items {
		p := g.n[v].path
		if p.len <= depth {
			units << Unit{bc[v], [v]}
			continue
		}
		if u := at[p[depth]] {
			mut its := units[u].items.clone()
			its << v
			units[u] = Unit{0, its}
		} else {
			at[p[depth]] = units.len
			units << Unit{0, [v]}
		}
	}
	mut keyed := []Unit{cap: units.len}
	for u in units {
		if u.items.len == 1 && g.n[u.items[0]].path.len <= depth {
			keyed << u
			continue
		}
		mut s := f32(0)
		for v in u.items {
			s += bc[v]
		}
		keyed << Unit{s / f32(u.items.len), g.group_sort(u.items, bc, depth + 1)}
	}
	keyed.sort_with_compare(fn (a &Unit, b &Unit) int {
		return if a.key < b.key {
			-1
		} else if a.key > b.key {
			1
		} else {
			a.items[0] - b.items[0]
		}
	})
	mut out := []int{cap: items.len}
	for u in keyed {
		out << u.items
	}
	return out
}

// crossings counts edge crossings between every pair of neighbouring ranks.
fn (g &Layers) crossings() int {
	mut c := 0
	for r in 0 .. g.rows.len - 1 {
		c += g.cross_between(r)
	}
	return c
}

fn (g &Layers) cross_between(r int) int {
	mut es := [][]int{}
	for v in g.rows[r] {
		for w in g.n[v].down {
			es << [g.n[v].order, g.n[w].order]
		}
	}
	mut c := 0
	for i in 0 .. es.len {
		for j in i + 1 .. es.len {
			if (es[i][0] - es[j][0]) * (es[i][1] - es[j][1]) < 0 {
				c++
			}
		}
	}
	return c
}

// pair_cross counts crossings between the edges of a and of b when a is left of b.
fn (g &Layers) pair_cross(a int, b int) int {
	mut c := 0
	for side in 0 .. 2 {
		na := if side == 0 { g.n[a].up } else { g.n[a].down }
		nb := if side == 0 { g.n[b].up } else { g.n[b].down }
		for x in na {
			for y in nb {
				if g.n[x].order > g.n[y].order {
					c++
				}
			}
		}
	}
	return c
}

// transpose swaps neighbours (of the same subgraph) while that removes crossings.
fn (mut g Layers) transpose() {
	for _ in 0 .. 4 {
		mut better := false
		for r in 0 .. g.rows.len {
			for i in 0 .. g.rows[r].len - 1 {
				a, b := g.rows[r][i], g.rows[r][i + 1]
				if g.n[a].path != g.n[b].path {
					continue
				}
				if g.pair_cross(b, a) < g.pair_cross(a, b) {
					g.rows[r][i], g.rows[r][i + 1] = b, a
					g.n[a].order, g.n[b].order = i + 1, i
					better = true
				}
			}
		}
		if !better {
			return
		}
	}
}
