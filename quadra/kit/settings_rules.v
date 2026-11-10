// The rules layout of Settings (specs/15, Settings): groups under small mono headers, a
// hairline under every row.
module kit

import quadra.core
import quadra.ui

// nav is the 226 px page column: the search field, 28 px rows (hover one step, the
// chosen one two) whose chevron shows the page's groups under it, the keyboard hint at
// the bottom over a hairline.
fn (s &Settings) nav(t &Theme) &ui.Div {
	p := t.p
	mut col :=
		ui.div().test_id('${s.id}-nav').flex_col().w(226).h_full().shrink_0().p(10).gap(2).border_r(1).border_color(p.line_strong).role(.tree)
	if el := s.search {
		col.child(ui.div().flex_col().pb(10).child(el))
	}
	mut list := ui.div().flex_col().gap(2).grow().min_h(0).overflow_y_scroll()
	shown := s.shown()
	for i, pg in s.pages {
		if s.query != '' && s.count(pg) == 0 {
			continue
		}
		on := i == shown
		open := s.st.open[i] && pg.groups.len > 0
		chevron := ui.div().test_id('${s.id}-expand-${i}').size(16).center().shrink_0().rounded(t.radius_sm).hover(ui.Patch{
			background: p.press
		}).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, -1, false}, settings_expand)).child(icon(if open {
			icon_chevron_down
		} else {
			icon_chevron_right
		}).size(12).color(p.text_2))
		mut row := ui.div().test_id('${s.id}-page-${i}').flex_row().items_center().gap(6).h(28).px(6).shrink_0().rounded(t.radius_md).cursor(.pointer).role(.treeitem).label(pg.title).selected(on).expanded(open).text_size(t.ui_text(.md)).text_color(p.text).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, -1, false},
			settings_pick)).child(chevron).child(ui.text(pg.title))
		s.nav_style(mut row, on, t)
		list.child(row)
		if !open {
			continue
		}
		for g, grp in pg.groups {
			if s.query != '' && grp.rows.filter(s.matches(it)).len == 0 {
				continue
			}
			mut sub := ui.div().test_id('${s.id}-group-${i}-${g}').flex_row().items_center().h(26).pl(28).pr(6).shrink_0().rounded(t.radius_md).cursor(.pointer).role(.treeitem).label(grp.title).text_color(p.text_2).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, g, false},
				settings_pick)).child(ui.text(grp.title))
			s.nav_style(mut sub, false, t)
			list.child(sub)
		}
	}
	col.child(list)
	if s.hint.len == 2 {
		col.child(ui.div().pt(10).border_t(1).border_color(p.line).child(key_hint(s.hint[0],
			s.hint[1])))
	}
	return col
}

fn (s &Settings) nav_style(mut row ui.Div, on bool, t &Theme) {
	if on {
		row.bg(t.p.raised_hover).border(1).border_color(t.p.line)
	} else {
		row.hover(ui.Patch{
			background: t.p.panel_hover
		})
	}
}

// row is one setting: 14 px title over a 12 px muted description that wraps, the control
// right, 16 px above and below, a hairline under.
fn settings_row(r SettingsRow, t &Theme) &ui.Div {
	if r.title == '' && r.description == '' { // the control is the whole row
		return ui.div().flex_col().w_full().py(16).border_b(1).border_color(t.p.line).child(r.control)
	}
	mut left := ui.div().flex_col().gap(2).grow().min_w(0).child(ui.text(r.title))
	if r.description != '' {
		left.child(ui.text(r.description).size(t.ui_text(.sm)).color(t.p.text_2))
	}
	return ui.div().flex_row().items_center().gap(16).py(16).border_b(1).border_color(t.p.line).role(.group).label(r.title).child(left).child(ui.div().shrink_0().child(r.control))
}

fn (mut s Settings) page(t &Theme) &ui.Div {
	p := t.p
	mut col :=
		ui.div().test_id('${s.id}-page').flex_col().grow().min_w(0).h_full().overflow_y_scroll().scroll_with(&s.st.scroll).pl(40).pr(32).py(24)
	if s.header.len > 0 {
		col.child(ui.div().flex_row().items_center().gap(4).children(s.header))
	}
	cur := s.shown()
	if cur < 0 || cur >= s.pages.len {
		return col
	}
	pg := s.pages[cur]
	col.child(ui.div().pt(if s.header.len > 0 { 24 } else { 0 }).role(.heading).label(pg.title).child(ui.text(pg.title).size(t.scaled(t.text_xl))))
	mut first := true
	for gi, g in pg.groups {
		rows := g.rows.filter(s.matches(it))
		if rows.len == 0 {
			continue
		}
		head :=
			ui.div().flex_col().pt(if first { 8 } else { 32 }).pb(6).border_b(1).border_color(p.line).child(ui.text(g.title).family(t.mono_family).size(t.ui_text(.sm)).color(p.text_2))
		if g.description != '' {
			head.child(ui.div().pt(2).child(ui.text(g.description).size(t.ui_text(.sm)).color(p.text_3)))
		}
		s.heads[gi] = head
		if first {
			s.first = gi
		}
		col.child(head)
		for r in rows {
			col.child(settings_row(r, t))
		}
		first = false
	}
	return col
}
