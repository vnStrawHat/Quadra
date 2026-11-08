// Workbench chrome: the activity bar and panel headers (specs/16).
module work

import quadra.core
import quadra.layout
import quadra.kit
import quadra.ui

pub struct Activity {
pub:
	id    string
	glyph string
	title string // tooltip; also the accessible name
	badge int    // a count shown on the icon, 0 = none
}

@[heap]
pub struct ActivityBar {
	kit.Lazy
pub mut:
	id     string
	items  []Activity
	footer []Activity // pinned to the bottom (settings, account)
	active string
	change ?core.AnyListener
}

pub fn activity_bar(id string, items []Activity, active string) &ActivityBar {
	return &ActivityBar{
		id:     id
		items:  items
		active: active
	}
}

pub fn (self_ &ActivityBar) footer(items []Activity) &ActivityBar {
	mut a := unsafe { self_ }
	a.footer = items
	return a
}

pub fn (self_ &ActivityBar) on_select(l core.Listener[string]) &ActivityBar {
	mut a := unsafe { self_ }
	a.change = l.any
	return a
}

pub fn (mut a ActivityBar) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut a, mut cx)
	return a.layout_inner(el, mut cx)
}

pub fn (mut a ActivityBar) build_tree(mut cx core.LayoutCx) core.Element {
	t := kit.theme_of(cx.app)
	mut col :=
		ui.div().id(a.id).test_id(a.id).flex_col().w(t.activity_w).h_full().shrink_0().bg(t.p.panel).border_r(1).border_color(t.p.line).role(.tablist)
	for it in a.items {
		col.child(a.item(it, t))
	}
	col.child(ui.div().grow())
	for it in a.footer {
		col.child(a.item(it, t))
	}
	return col
}

// item is one panel toggle (specs/15): a 28 px ghost icon button whose icon turns the
// accent when its panel is shown; a count badge in the corner.
fn (a &ActivityBar) item(it Activity, t &kit.Theme) core.Element {
	p := t.p
	on := it.id == a.active
	change := a.change
	id := it.id
	mut d :=
		ui.div().test_id('${a.id}-${it.id}').w_full().h(t.activity_w).center().cursor(.pointer).role(.tab).label(it.title).selected(on).on_click(kit.fire_click(change, id))
	icon := ui.div().size(28).center().rounded(t.radius_md).hover(ui.Patch{
		background: p.panel_hover
	}).child(kit.icon(it.glyph).size(16).color(if on {
		p.accent_text
	} else {
		p.text_2
	}))
	if it.badge > 0 {
		label := if it.badge > 99 { '99+' } else { it.badge.str() }
		icon.child(ui.div().absolute().top(-2).right(-4).h(14).min_w(14).px(1).rounded(7).bg(p.canvas.lerp(p.error,
			0.4)).border(1).border_color(p.line_strong).center().text_size(9).text_color(p.text).font_weight(500).child(ui.text(label)))
	}
	return kit.tooltip(d.child(icon), it.title).placement(.right)
}

// panel_header is a 28 px strip: the title muted, then actions (icon buttons) on the right.
pub fn panel_header(title string, actions []core.Element) &kit.Styled {
	return kit.themed_with(PanelHeader{title, actions}, fn (h PanelHeader, t &kit.Theme) core.Element {
		title, actions := h.title, h.actions
		return ui.div().flex_row().items_center().gap(4).h(t.panel_header_h).shrink_0().pl(8).pr(4).bg(t.p.panel).border_b(1).border_color(t.p.line).child(ui.text(title).color(t.p.text_2)).child(ui.div().grow()).children(actions)
	})
}

struct PanelHeader {
	title   string
	actions []core.Element
}
