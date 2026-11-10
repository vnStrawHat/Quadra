module data

struct Model {
mut:
	names  []string
	values []int
}

fn table_of(m &Model) (Table, CellFn) {
	cell := fn [m] (row int, col int) string {
		return if col == 0 { m.names[row] } else { m.values[row].str() }
	}
	t := Table{
		columns: [Column{
			title: 'Name'
		}, Column{
			title:   'Value'
			numeric: true
		}]
	}
	return t, cell
}

fn test_keyed_updates_keep_selection_cursor_and_sort() {
	mut m := &Model{
		names:  ['a', 'b', 'c']
		values: [3, 1, 2]
	}
	mut t, cell := table_of(m)
	t.update_keyed(m.names.clone(), FnCells{cell}, 0)
	t.sort_by(1) // by value: b, c, a
	assert [t.data_row(0), t.data_row(1), t.data_row(2)] == [1, 2, 0]
	t.selected[2] = true // c
	t.cursor = 1 // display position of c
	// c moves to the front of the data, b goes away, d arrives.
	m.names = ['c', 'a', 'd']
	m.values = [2, 3, 0]
	t.update_keyed(m.names.clone(), FnCells{cell}, 1000)
	assert t.selection() == [0] // still c
	assert t.keys[t.data_row(t.cursor)] == 'c'
	assert [t.data_row(0), t.data_row(1), t.data_row(2)] == [2, 0, 1] // still sorted: d, c, a
	assert t.flash.len == 0 // new and removed rows do not flash
	// A changed value flashes once, then fades.
	m.values = [5, 3, 0]
	t.update_keyed(m.names.clone(), FnCells{cell}, 2000)
	assert flash_level(t.flash, 'c', 1, 2000) == 1
	assert flash_level(t.flash, 'c', 0, 2000) == 0
	assert flash_level(t.flash, 'a', 1, 2000) == 0
	assert flash_level(t.flash, 'c', 1, 2400) > 0.4
	assert flash_level(t.flash, 'c', 1, 2900) == 0
	t.update_keyed(m.names.clone(), FnCells{cell}, 4000)
	assert t.flash.len == 0 // finished flashes are dropped
}

fn test_age_text() {
	assert age_text(45) == '45s'
	assert age_text(720) == '12m'
	assert age_text(3 * 3600 + 5) == '3h'
	assert age_text(6 * 86400) == '6d'
}
