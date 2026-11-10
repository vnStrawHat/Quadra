// Data pages: table, tree, and the stress page.
module pages

import quadra.core
import quadra.kit
import quadra.kit.data
import quadra.ui

const first_names = ['An', 'Bình', 'Chi', 'Dũng', 'Giang', 'Hà', 'Khoa', 'Lan', 'Minh', 'Ngọc']
const cities = ['Hà Nội', 'Huế', 'Đà Nẵng', 'Cần Thơ', 'Hải Phòng']

pub struct TablePage {
mut:
	table core.View[data.Table]
	sel   int
}

@[markused]
pub fn (mut p TablePage) render(mut cx core.ViewCx[TablePage]) core.Element {
	if p.table.id == 0 {
		p.table = data.new_table(mut cx.Cx,
			id:      'dt'
			columns: [
				data.Column{
					title:   '#'
					width:   90
					numeric: true
					pinned:  true
				},
				data.Column{
					title: 'Tên'
					width: 280
				},
				data.Column{
					title: 'Thành phố'
					width: 280
				},
				data.Column{
					title:   'Điểm'
					width:   90
					numeric: true
				},
			]
			rows:    1_000_000
			cell:    fn (row int, col int) string {
				return match col {
					0 { '${row + 1}' }
					1 { first_names[row % first_names.len] + ' ${row / 10}' }
					2 { cities[(row / 7) % cities.len] }
					else { '${(i64(row) * 37) % 101}' }
				}
			}
		)
		data.on_select(mut cx.Cx, p.table, cx.listener(fn (mut p TablePage, s &[]int, mut _ core.ViewCx[TablePage]) {
			p.sel = s.len
		}))
	}
	t := kit.theme(cx)
	return page('Table',
		'A million rows with sortable, resizable columns, shift/ctrl selection and arrow keys. Only visible rows are built.', [
		demo('1,000,000 rows', col([framed(t, cx.child_view(p.table)).h(360),
			hint(t, '${p.sel} row(s) selected')]),
			'data.new_table(mut cx.Cx, columns: cols, rows: 1_000_000, cell: fn (r int, c int) string { ... })'),
	])
}

pub struct TreePage {
mut:
	tree core.View[data.Tree]
	node string
}

@[markused]
pub fn (mut p TreePage) render(mut cx core.ViewCx[TreePage]) core.Element {
	if p.tree.id == 0 {
		p.tree = data.new_tree(mut cx.Cx, 'dtree', [
			data.TreeNode{'quadra', 'quadra', kit.icon_folder, [
				data.TreeNode{'core', 'core', kit.icon_folder, [
					data.TreeNode{'app.v', 'app.v', kit.icon_file, []},
					data.TreeNode{'window.v', 'window.v', kit.icon_file, []},
				]},
				data.TreeNode{'ui', 'ui', kit.icon_folder, [
					data.TreeNode{'div.v', 'div.v', kit.icon_file, []},
				]},
				data.TreeNode{'kit', 'kit', kit.icon_folder, []},
			]},
			data.TreeNode{'docs', 'docs', kit.icon_folder, [
				data.TreeNode{'progress.md', 'progress.md', kit.icon_file, []},
			]},
		])
		data.on_tree_select(mut cx.Cx, p.tree, cx.listener(fn (mut p TreePage, id &string, mut _ core.ViewCx[TreePage]) {
			p.node = *id
		}))
	}
	t := kit.theme(cx)
	return page('Tree', 'Nested items such as files. Arrow keys expand, collapse and move.', [
		demo('Explorer', row([framed(t, cx.child_view(p.tree)).w(260).h(220),
			hint(t, 'Selected: ${p.node}')]), "data.new_tree(mut cx.Cx, 'files', nodes)"),
	])
}

fn stress_page(t &kit.Theme) core.Element {
	mut grid := ui.div().grid_fill(96).gap(4)
	for i in 0 .. 1000 {
		grid.child(ui.div().h(28).bg(t.p.panel).cursor(.pointer).hover(ui.Patch{
			background: t.p.accent
			text_color: t.p.text_on_accent
		}).center().text_size(t.font_size(.xs)).child(ui.text('Item ${i}').family(t.mono_family)))
	}
	return page('Stress',
		'1,000 hoverable cells. Hover changes repaint only; open the profiler with Ctrl-Shift-P.', [
		grid,
	])
}

const feed_words = 'deploy rollout pod node ready scaled restarted pulled image config map secret volume mounted probe failed passed'.split(' ')

fn feed_text(i int) string {
	mut words := []string{}
	for k in 0 .. 4 + (i * 7919) % 40 {
		words << feed_words[(i + k * 31) % feed_words.len]
	}
	return words.join(' ')
}

struct ThemeRef {
	t &kit.Theme = unsafe { nil }
}

fn variable_list_page(t &kit.Theme) core.Element {
	list := ui.list_with('feed', 100_000, ThemeRef{unsafe { t }}, fn (r ThemeRef, i int) core.Element {
		t := r.t
		return ui.div().flex_col().gap(2).px(12).py(8).border_b(1).border_color(t.p.line).child(ui.text('#${i}').color(t.p.text_3).size(t.font_size(.xs))).child(ui.text(feed_text(i)))
	}).estimate(52)
	return page('Variable list',
		'100,000 rows whose height is their wrapped text. Rows are measured as they appear; scrolling up never moves what is on screen.', [
		demo('Event feed', framed(t, list).h(360),
			"ui.list('feed', n, fn (i int) core.Element { ... }).estimate(52)"),
	])
}
