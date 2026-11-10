module layout

import quadra.prim

fn test_absolute_negative_insets() {
	mut t := Tree{}
	// -1 is also the `unknown` sentinel: it must still count as a set inset
	a := t.node(Style{
		position:    .absolute
		inset_right: px(-1)
		inset_top:   px(-1)
		width:       px(20)
		height:      px(20)
	}, [])
	b := t.node(Style{
		position:     .absolute
		inset_left:   px(-1)
		inset_bottom: px(-4)
		width:        px(10)
		height:       px(10)
	}, [])
	root := t.node(Style{ width: px(200), height: px(100) }, [a, b])
	t.compute(root, prim.size(200, 100))
	assert t.rect(a) == prim.rect(181, -1, 20, 20)
	assert t.rect(b) == prim.rect(-1, 94, 10, 10)
}
