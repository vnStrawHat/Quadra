// Reads a checked holder back as the wrong type: outside -prod it must panic with both
// type names (tests/holder_test.v runs it).
module main

import quadra.prim

struct Apple {
	seeds int
}

struct Brick {
	weight int
}

fn main() {
	h := prim.hold(&Apple{3})
	b := prim.held[Brick](h)
	println('read ${b.weight}: no panic')
}
