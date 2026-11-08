module prim

fn test_rect_ops() {
	a := rect(0, 0, 10, 10)
	b := rect(5, 5, 10, 10)
	assert a.intersects(b)
	assert a.intersect(b) == rect(5, 5, 5, 5)
	assert a.union(b) == rect(0, 0, 15, 15)
	assert !a.intersects(rect(10, 0, 5, 5)) // edges touch only
	assert a.contains(pt(0, 0)) && !a.contains(pt(10, 10))
	assert a.inset(Edges{1, 2, 3, 4}) == rect(4, 1, 4, 6)
	assert rect(0, 0, 0, 5).union(b) == b
}

fn test_colors() {
	c := rgb(0x2563EB)
	assert c.hex() == '#2563ebff'
	assert rgba(0x11223380).hex() == '#11223380'
	red := hsla(0, 1, 0.5, 1)
	assert red.hex() == '#ff0000ff'
	assert hsla(120, 1, 0.5, 1).hex() == '#00ff00ff'
	assert black.lerp(white, 0.5).hex() == '#808080ff'
}
