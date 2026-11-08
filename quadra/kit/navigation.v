// Navigation: sidebar, toolbar, breadcrumb.
module kit

import quadra.core
import quadra.layout
import quadra.ui

pub struct NavItem {
pub:
	id    string
	label string
	glyph string
}

pub struct NavSection {
pub:
	title string
	items []NavItem
}

@[heap]
pub struct Sidebar {
	Lazy
pub mut:
	sections  []NavSection
	selected  string
	collapsed bool
	width     f32 = 240
	picked    ?core.AnyListener
	header    ?core.Element
}

pub fn sidebar(sections []NavSection, selected string) &Sidebar {
	return &Sidebar{
		sections: sections
		selected: selected
	}
}

pub fn (self_ &Sidebar) collapsed(on bool) &Sidebar {
	mut s := unsafe { self_ }
	s.collapsed = on
	return s
}

pub fn (self_ &Sidebar) header(el core.Element) &Sidebar {
	mut s := unsafe { self_ }
	s.header = el
	return s
}

pub fn (self_ &Sidebar) on_select(l core.Listener[string]) &Sidebar {
	mut s := unsafe { self_ }
	s.picked = l.any
	return s
}

fn (sb &Sidebar) item(it NavItem, t &Theme) &ui.Div {
	on := it.id == sb.selected
	picked := sb.picked
	id := it.id
	p := t.p
	// A panel row (specs/15): 26 tall, hover one step, the selected row two.
	mut row :=
		ui.div().test_id('nav-${it.id}').flex_row().items_center().gap(6).h(t.row_h).pl(10).pr(8).cursor(.pointer).role(.treeitem).label(it.label).selected(on).text_size(t.font_size(.md)).text_color(p.text)
	row.on_click(fire_click(picked, id))
	if on {
		row.bg(p.press)
	} else {
		row.hover(ui.Patch{
			background: p.raised_hover
		})
	}
	if it.glyph != '' {
		row.child(icon(it.glyph).size(16).color(p.text_2))
	}
	if !sb.collapsed {
		row.child(ui.text(it.label))
	}
	if sb.collapsed {
		return row.justify_center()
	}
	return row
}

pub fn (mut sb Sidebar) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut sb, mut cx)
	return sb.layout_inner(el, mut cx)
}

pub fn (mut sb Sidebar) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	mut col :=
		ui.div().flex_col().py(4).w(if sb.collapsed { 40 } else { sb.width }).h_full().shrink_0().bg(t.p.panel).border_r(1).border_color(t.p.line_strong).overflow_y_scroll().role(.tree)
	if h := sb.header {
		col.child(h)
	}
	for s in sb.sections {
		if s.title != '' && !sb.collapsed {
			col.child(section_label(s.title, t).pl(10).pt(12).pb(4))
		}
		for it in s.items {
			col.child(sb.item(it, t))
		}
	}
	return col
}

// toolbar lays out controls in a row with consistent spacing; pass `kit.vdivider()` to
// separate groups. 45 tall on the canvas tone with a hairline below (specs/15).
pub fn toolbar(children []core.Element) &Styled {
	return themed_with(children, fn (children []core.Element, t &Theme) core.Element {
		return ui.div().flex_row().items_center().gap(4).px(8).py(6).h(45).shrink_0().border_b(1).border_color(t.p.line).bg(t.p.canvas).children(children)
	})
}

@[heap]
pub struct Breadcrumb {
	Lazy
pub mut:
	items  []string
	picked ?core.AnyListener
}

pub fn breadcrumb(items []string) &Breadcrumb {
	return &Breadcrumb{
		items: items
	}
}

pub fn (self_ &Breadcrumb) on_select(l core.Listener[int]) &Breadcrumb {
	mut b := unsafe { self_ }
	b.picked = l.any
	return b
}

pub fn (mut b Breadcrumb) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut b, mut cx)
	return b.layout_inner(el, mut cx)
}

pub fn (mut b Breadcrumb) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	// Mono 14 px muted segments split by a placeholder-toned "›" (specs/15).
	mut row :=
		ui.div().flex_row().items_center().gap(4).text_size(t.font_size(.md)).font(t.mono_family).text_color(t.p.text_2)
	for i, item in b.items {
		last := i == b.items.len - 1
		if i > 0 {
			row.child(ui.text('›').color(t.p.text_3))
		}
		mut crumb := ui.div().test_id('crumb-${i}').child(ui.text(item))
		if !last {
			picked := b.picked

			crumb.cursor(.pointer).hover(ui.Patch{
				text_color: t.p.text
			}).on_click(fire_click(picked, i))
		}
		row.child(crumb)
	}
	return row
}
