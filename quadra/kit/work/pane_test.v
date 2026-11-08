module work

import quadra.prim

fn sample() PaneNode {
	// editor | (logs / shell)
	return hsplit(leaf('editor'), vsplit(leaf('logs'), leaf('shell'), 0.6), 0.7)
}

fn test_groups_in_reading_order() {
	assert sample().groups() == ['editor', 'logs', 'shell']
}

fn test_split_places_the_new_group() {
	t := sample().split('logs', .row, 'events', false)
	assert t.groups() == ['editor', 'logs', 'events', 'shell']
	b := sample().split('editor', .col, 'outline', true)
	assert b.groups() == ['outline', 'editor', 'logs', 'shell']
	assert b.children[0].axis == .col && b.children[0].ratio == 0.5
	assert sample().split('nope', .row, 'x', false).groups() == sample().groups()
}

fn test_remove_promotes_the_sibling() {
	t := sample().remove('logs')
	assert t.groups() == ['editor', 'shell']
	assert t.children[1].is_leaf() // the column split collapsed into its other child
	assert t.ratio == 0.7
	one := leaf('only').remove('only')
	assert one.groups() == ['only']
}

fn test_with_ratio_follows_the_path_and_clamps() {
	t := sample().with_ratio([1], 0.25)
	assert t.children[1].ratio == 0.25 && t.ratio == 0.7
	assert sample().with_ratio([], 2).ratio == 0.95
	assert sample().with_ratio([], -1).ratio == 0.05
}

fn test_toggle_folds_one_side_of_a_stack() {
	t := vsplit(leaf('a'), leaf('b'), 0.5).toggle('b')
	assert t.children[1].collapsed
	fa, fb := t.folded()
	assert !fa && fb
	both := t.toggle('a')
	ba, bb := both.folded()
	assert ba && !bb // never both: the second stays open
	row := hsplit(leaf('a'), leaf('b'), 0.5).toggle('a')
	ra, rb := row.folded()
	assert !row.is_collapsed('a')
	assert !ra && !rb // side-by-side splits never fold
	assert t.is_collapsed('b') && !t.is_collapsed('a')
	assert t.toggle('b').children[1].collapsed == false
}

fn dock3() Dock {
	return Dock{
		root:   hsplit(leaf('a'), leaf('b'), 0.5)
		tabs:   {
			'a': [Tab{
				id: 'x'
			}, Tab{
				id: 'y'
			}]
			'b': [Tab{
				id: 'z'
			}]
		}
		active: {
			'a': 'x'
			'b': 'z'
		}
	}
}

fn test_dock_moves_splits_and_closes_empty_groups() {
	mut d := dock3()
	d.apply(DockEvent{'x', 'a', 'b', .center, 0}) // into b, first
	assert d.tabs['a'].map(it.id) == ['y']
	assert d.tabs['b'].map(it.id) == ['x', 'z']
	assert d.active['a'] == 'y' && d.active['b'] == 'x'
	d.apply(DockEvent{'z', 'b', 'b', .bottom, -1}) // split b: z goes below
	assert d.root.groups() == ['a', 'b', 'group-1']
	assert d.tabs['group-1'].map(it.id) == ['z']
	d.apply(DockEvent{'y', 'a', 'group-1', .center, -1}) // a empties: its pane closes
	assert d.root.groups() == ['b', 'group-1']
	assert 'a' !in d.tabs
	assert d.tabs['group-1'].map(it.id) == ['z', 'y']
	d.move('group-1', TabMove{'y', 0})
	assert d.tabs['group-1'].map(it.id) == ['y', 'z']
}

fn test_dock_zones() {
	r := prim.rect(0, 0, 400, 200)
	assert zone_at(r, 200, 100) == .center
	assert zone_at(r, 20, 100) == .left
	assert zone_at(r, 390, 100) == .right
	assert zone_at(r, 200, 10) == .top
	assert zone_at(r, 200, 195) == .bottom
}
