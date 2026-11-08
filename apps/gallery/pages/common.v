// Shared building blocks for gallery pages.
module pages

import quadra.core
import quadra.kit
import quadra.ui

// page lays out the section, title, description and demos of one component page.
pub fn page(title string, description string, sections []core.Element) core.Element {
	mut section := 'Overview'
	for p in all() {
		if p.title == title {
			section = p.section
		}
	}
	return kit.themed_with(PageArgs{title, description, sections, section}, fn (a PageArgs, t &kit.Theme) core.Element {
		title, description, sections, section := a.title, a.description, a.sections, a.section
		mut body := ui.div().flex_col().gap(20).px(32).py(28).w_full().max_w(1000)
		body.child(ui.div().flex_col().gap(6).child(kit.section_label(section, t)).child(kit.heading(title,
			1)).child(ui.div().max_w(640).child(ui.text(description).color(t.p.text_2).selectable())))
		body.children(sections)
		return body
	})
}

// demo is one framed instrument: a header strip, the live demo on the canvas, the code strip.
pub fn demo(title string, content core.Element, code string) core.Element {
	return kit.themed_with(DemoArgs{title, content, code}, fn (a DemoArgs, t &kit.Theme) core.Element {
		title, content, code := a.title, a.content, a.code
		p := t.p
		mut d := ui.div().flex_col().border(1).border_color(p.line)
		d.child(ui.div().flex_row().items_center().h(t.panel_header_h).px(12).bg(p.panel).border_b(1).border_color(p.line).child(kit.section_label(title,
			t)))
		d.child(ui.div().flex_col().p(20).bg(p.canvas).child(content))
		if code != '' {
			d.child(ui.div().px(12).py(7).bg(p.panel).border_t(1).border_color(p.line).child(ui.text(code).family(t.mono_family).size(t.scaled(t.text_sm)).color(p.text_2).selectable()))
		}
		return d
	})
}

pub fn row(items []core.Element) core.Element {
	return ui.div().flex_row().wrap().items_center().gap(12).children(items)
}

pub fn col(items []core.Element) core.Element {
	return ui.div().flex_col().gap(12).children(items)
}

// framed is a hairline box for demos that need an edge (panes, lists, menus in place).
pub fn framed(t &kit.Theme, content core.Element) &ui.Div {
	return ui.div().flex_col().border(1).border_color(t.p.line).overflow_hidden().child(content)
}

// hint is one line of tertiary text.
pub fn hint(t &kit.Theme, s string) core.Element {
	return ui.text(s).size(t.scaled(t.text_sm)).color(t.p.text_3)
}

// mono is a value in the mono face.
pub fn mono(t &kit.Theme, s string) core.Element {
	return ui.text(s).family(t.mono_family).color(t.p.text_2).selectable()
}

// setting is a settings row: a title and a description on the left, a control on the right.
pub fn setting(t &kit.Theme, title string, description string, control core.Element) core.Element {
	return ui.div().flex_row().items_center().gap(16).py(10).border_b(1).border_color(t.p.line).child(ui.div().flex_col().gap(2).grow().child(ui.text(title)).child(hint(t,
		description))).child(control)
}

struct PageArgs {
	title       string
	description string
	sections    []core.Element
	section     string
}

struct DemoArgs {
	title   string
	content core.Element
	code    string
}
