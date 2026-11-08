// Layout pages: tabs, accordion, split pane, scroll area, grid.
module pages

import quadra.core
import quadra.prim
import quadra.kit
import quadra.ui

pub struct TabsPage {
mut:
	tab  int
	pill int
}

@[markused]
pub fn (mut p TabsPage) render(mut cx core.ViewCx[TabsPage]) core.Element {
	t := kit.theme(cx)
	panels := ['Pod api-7d9f8c6b5-x2k4p is running on node pool-a-3.',
		'12:04:51  GET /healthz 200 0.4ms', 'apiVersion: v1\nkind: Pod']
	return page('Tabs',
		'Switches between panels in the same place. The active tab has an accent label.', [
		demo('Underline', ui.div().flex_col().child(kit.tabs('tb-main',
			['Overview', 'Logs', 'YAML'], p.tab).on_change(cx.listener(fn (mut p TabsPage, v &int, mut _ core.ViewCx[TabsPage]) {
			p.tab = *v
		}))).child(ui.div().p(12).child(ui.text(panels[p.tab]).family(t.mono_family).color(t.p.text_2))),
			"kit.tabs('detail', ['Overview', 'Logs', 'YAML'], v.tab).on_change(...)"),
		demo('Pill', kit.tabs('tb-pill', ['Day', 'Week', 'Month'], p.pill).pill().on_change(cx.listener(fn (mut p TabsPage, v &int, mut _ core.ViewCx[TabsPage]) {
			p.pill = *v
		})), '.pill()'),
	])
}

pub struct AccordionPage {
mut:
	open []bool = [true, false, false]
}

@[markused]
pub fn (mut p AccordionPage) render(mut cx core.ViewCx[AccordionPage]) core.Element {
	return page('Accordion', 'Sections that expand in place, for long settings and details.', [
		demo('Basic', kit.accordion('acc', [
			kit.AccordionItem{'What is Quadra?', ui.text('A GPU UI framework for V.')},
			kit.AccordionItem{'Does it support Vietnamese?', ui.text('Có, cả hiển thị lẫn bộ gõ.')},
			kit.AccordionItem{'Which platforms?', ui.text('Windows first; Linux and macOS next.')},
		], p.open).on_toggle(cx.listener(fn (mut p AccordionPage, i &int, mut _ core.ViewCx[AccordionPage]) {
			p.open[*i] = !p.open[*i]
		})), "kit.accordion('faq', items, v.open).on_toggle(...)"),
	])
}

pub struct SplitPage {
mut:
	ratio f32 = 0.3
	vert  f32 = 0.6
}

@[markused]
pub fn (mut p SplitPage) render(mut cx core.ViewCx[SplitPage]) core.Element {
	t := kit.theme(cx)
	left := ui.div().size_full().p(12).bg(t.p.panel).child(hint(t, 'Explorer'))
	top := ui.div().size_full().p(12).child(ui.text('Editor: drag the dividers.'))
	bottom := ui.div().size_full().p(12).bg(t.p.panel).child(mono(t, '$ v run .'))
	inner := kit.split('sp-v', top, bottom, p.vert).vertical().on_resize(cx.listener(fn (mut p SplitPage, r &f32, mut _ core.ViewCx[SplitPage]) {
		p.vert = *r
	}))
	return page('Split pane', 'Resizable panes with a 1 px divider and a 6 px grab area.', [
		demo('Workbench', framed(t, kit.split('sp-h', left, inner, p.ratio).on_resize(cx.listener(fn (mut p SplitPage, r &f32, mut _ core.ViewCx[SplitPage]) {
			p.ratio = *r
		}))).h(260), "kit.split('id', left, right, v.ratio).on_resize(...)"),
	])
}

fn scroll_page(t &kit.Theme) core.Element {
	mut list := ui.div().flex_col()
	for i in 0 .. 60 {
		list.child(ui.div().flex_row().gap(12).px(12).h(t.row_h).items_center().border_b(1).border_color(t.p.line).child(mono(t, '${
			i + 1:3}')).child(ui.text('12:04:${i % 60:02}  GET /api/v1/pods 200')))
	}
	return page('Scroll area', 'A fixed-height region with a thin overlay scrollbar.', [
		demo('Log', framed(t, kit.scroll_area(list, 220)).w(460), 'kit.scroll_area(content, 220)'),
	])
}

fn grid_page(t &kit.Theme) core.Element {
	mut cards := ui.div().grid_fill(140).gap(8)
	for i in 0 .. 12 {
		cards.child(ui.div().h(64).bg(prim.hsla(190 + f32(i) * 6, 0.5, if t.mode == .dark {
			f32(0.3)
		} else {
			0.8
		}, 1)).center().child(mono(t, 'node-${i + 1}')))
	}
	return page('Grid', 'Responsive columns: as many as fit at the minimum width.', [
		demo('Fill', cards, 'ui.div().grid_fill(140).gap(8)'),
	])
}
