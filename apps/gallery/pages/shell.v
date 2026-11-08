// The gallery shell: titlebar, sidebar of pages, and the current page.
module pages

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub struct Gallery {
pub mut:
	current string = 'start'
	views   map[string]core.AnyView
	dark    bool
	hud     bool // show the performance overlay (main.v; off in tests and screenshots)
	no_nav  bool // the page list is hidden (the status bar toggle)
mut:
	hud_view core.View[Hud]
}

@[markused]
pub fn (mut g Gallery) render(mut cx core.ViewCx[Gallery]) core.Element {
	if g.current !in g.views {
		g.views[g.current] = make(g.current, mut cx.Cx)
	}
	mut sections := []kit.NavSection{}
	mut order := []string{}
	mut by := map[string][]kit.NavItem{}
	for p in all() {
		if p.section !in by {
			order << p.section
		}
		by[p.section] << kit.NavItem{p.id, p.title, p.glyph}
	}
	for s in order {
		sections << kit.NavSection{s, by[s]}
	}
	toggle := kit.icon_button('theme-toggle', if g.dark { kit.icon_sun } else { kit.icon_moon }).size(.sm).tooltip('Toggle theme').on_click(cx.listener(fn (mut g Gallery, _ &core.ClickEvent, mut cx core.ViewCx[Gallery]) {
		g.dark = !g.dark
		kit.set_theme(mut cx.app, if g.dark { kit.dark() } else { kit.light() })
	}))
	// The window draws its own titlebar (custom_titlebar in main.v): drag, snap layouts and
	// the window buttons still behave natively.
	header := kit.titlebar('Quadra Gallery').menu(app_menu()).controls([
		ui.div().grow(),
		toggle,
	])
	nav := kit.sidebar(sections, g.current).on_select(cx.listener(fn (mut g Gallery, id &string, mut _ core.ViewCx[Gallery]) {
		g.current = *id
	}))
	body :=
		ui.div().grow().h_full().overflow_y_scroll().id(g.current).child(cx.child_any(g.views[g.current]))
	t := kit.theme(cx)
	mut title := ''
	for p in all() {
		if p.id == g.current {
			title = p.title
		}
	}
	status := kit.statusbar([
		kit.status_button('status-pages', kit.icon_panel_left, !g.no_nav).tooltip('Pages').on_click(cx.listener(fn (mut g Gallery, _ &core.ClickEvent, mut _ core.ViewCx[Gallery]) {
			g.no_nav = !g.no_nav
		})),
		kit.status_button('status-search', kit.icon_search, false).tooltip_keys('Commands',
			'ctrl-k').on_click(core.handler(fn (_ &core.ClickEvent, mut cx core.Cx) {
			input.open_command_palette(mut cx, commands())
		})),
		kit.status_divider(),
		kit.status_label(.running, 'Ready'),
		kit.status_item(kit.icon_layers, title),
	], [
		kit.status_item('', '${all().len} pages'),
		kit.status_item(kit.icon_palette, '${t.name} · ${t.density}'),
		kit.status_item(kit.icon_command, 'Ctrl-K'),
	])
	mut main := ui.div().flex_row().grow()
	if !g.no_nav {
		main.child(nav)
	}
	mut root := ui.div().flex_col().size_full().child(header).child(main.child(body)).child(status)
	if g.hud {
		if g.hud_view.id == 0 {
			g.hud_view = cx.new_view(Hud{})
		}
		root.child(cx.child_view(g.hud_view))
	}
	return root
}
