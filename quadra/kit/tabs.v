// Tabs (header row; the caller renders the selected panel) and accordion.
module kit

import quadra.core
import quadra.layout
import quadra.ui

@[heap]
pub struct Tabs {
	Lazy
pub mut:
	id       string
	items    []string
	selected int
	pill     bool
	change   ?core.AnyListener
}

pub fn tabs(id string, items []string, selected int) &Tabs {
	return &Tabs{
		id:       id
		items:    items
		selected: selected
	}
}

pub fn (self_ &Tabs) pill() &Tabs {
	mut t := unsafe { self_ }
	t.pill = true
	return t
}

pub fn (self_ &Tabs) on_change(l core.Listener[int]) &Tabs {
	mut t := unsafe { self_ }
	t.change = l.any
	return t
}

fn (tb &Tabs) tab(i int, item string, t &Theme) &ui.Div {
	p := t.p
	on := i == tb.selected
	// Toggle buttons (specs/15): the selected one has an accent label; pills also take the
	// accent tint.
	mut tab :=
		ui.div().test_id('${tb.id}-${i}').px(8).h(t.height(.md)).flex_row().items_center().cursor(.pointer).role(.tab).label(item).selected(on).text_size(t.ui_text(.md)).rounded(t.radius_md)
	tab.on_click(fire_click(tb.change, i))
	if on {
		tab.text_color(p.accent_text)
		if tb.pill {
			tab.bg(p.accent_tint)
		}
	} else {
		tab.text_color(if tb.pill { p.text } else { p.text_2 }).hover(ui.Patch{
			background: p.panel_hover
			text_color: p.text
		})
	}
	return tab.child(ui.text(item))
}

pub fn (mut tb Tabs) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut tb, mut cx)
	return tb.layout_inner(el, mut cx)
}

pub fn (mut tb Tabs) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	mut row := ui.div().id(tb.id).flex_row().gap(if tb.pill { 4 } else { 0 }).role(.tablist)
	if !tb.pill {
		row.border_b(1).border_color(t.p.line)
	}
	for i, item in tb.items {
		row.child(tb.tab(i, item, t))
	}
	return row
}

pub struct AccordionItem {
pub:
	title string
	body  core.Element
}

@[heap]
pub struct Accordion {
	Lazy
pub mut:
	id     string
	items  []AccordionItem
	open   []bool
	toggle ?core.AnyListener
}

// accordion shows collapsible sections; `open` (owned by the parent) says which are expanded.
pub fn accordion(id string, items []AccordionItem, open []bool) &Accordion {
	return &Accordion{
		id:    id
		items: items
		open:  open
	}
}

pub fn (self_ &Accordion) on_toggle(l core.Listener[int]) &Accordion {
	mut a := unsafe { self_ }
	a.toggle = l.any
	return a
}

pub fn (mut a Accordion) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut a, mut cx)
	return a.layout_inner(el, mut cx)
}

pub fn (mut a Accordion) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	p := t.p
	mut col := ui.div().flex_col().border(1).border_color(p.line).rounded(t.radius_md)
	for i, item in a.items {
		is_open := i < a.open.len && a.open[i]
		// A disclosure row (specs/15): the chevron leads, right when closed, down when open.
		chevron := if is_open { icon_chevron_down } else { icon_chevron_right }
		mut head :=
			ui.div().test_id('${a.id}-${i}').flex_row().items_center().gap(4).px(8).h(t.height(.md)).cursor(.pointer).role(.button).label(item.title).expanded(is_open)

		head.hover(ui.Patch{
			background: p.panel_hover
		}).on_click(fire_click(a.toggle, i))
		head.child(icon(chevron).size(14).color(p.text_2)).child(ui.text(item.title))
		if i > 0 {
			head.border_t(1).border_color(p.line)
		}
		col.child(head)
		if is_open {
			col.child(ui.div().pl(26).pr(12).pb(12).child(item.body))
		}
	}
	return col
}
