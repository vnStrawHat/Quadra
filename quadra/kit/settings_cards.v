// The cards layout of Settings (specs/15, Settings, cards): a 250 px navigation with
// icons, a page header, the groups as outlined cards.
module kit

import quadra.core
import quadra.layout
import quadra.ui

// subgroups are the groups the column lists under a page: the titled groups that keep a
// row, when more than one group keeps a row.
fn (s &Settings) subgroups(pg SettingsPage) ([]int, int) {
	mut kept, mut titled := 0, []int{}
	for g, grp in pg.groups {
		if grp.rows.any(s.matches(it)) {
			kept++
			if grp.title != '' {
				titled << g
			}
		}
	}
	if kept < 2 {
		titled = []int{}
	}
	return titled, kept
}

// card_row is a 28 px row of the column: hover `press` at 80 %, chosen `press` with a
// medium label.
fn card_row(id string, title string, on bool, n SettingsNav, t &Theme) &ui.Div {
	mut row := ui.div().test_id(id).flex_row().items_center().gap(8).h(28).px(8).shrink_0().rounded(t.radius_md).cursor(.pointer).role(.treeitem).label(title).selected(on).on_click(core.handler_with(n,
		settings_pick))
	if on {
		row.bg(t.p.press).font_weight(500)
	} else {
		row.hover(ui.Patch{
			background: t.p.press.alpha(0.8)
		})
	}
	return row
}

fn (s &Settings) cards_nav(t &Theme) &ui.Div {
	p := t.p
	mut col :=
		ui.div().test_id('${s.id}-nav').flex_col().w(251).h_full().shrink_0().bg(p.panel).border_r(1).border_color(p.line).role(.tree)
	if el := s.search {
		col.child(ui.div().flex_col().pt(12).px(12).child(el))
	}
	mut list := ui.div().flex_col().gap(8).p(12).grow().min_h(0).overflow_y_scroll()
	shown := s.shown()
	for i, pg in s.pages {
		if s.query != '' && s.count(pg) == 0 {
			continue
		}
		subs, kept := s.subgroups(pg)
		open := s.st.open[i] && subs.len > 0
		on := i == shown && (s.st.group < 0 || s.st.gpage != i || kept == 1)
		mut row :=
			card_row('${s.id}-page-${i}', pg.title, on, SettingsNav{s.st, s.picked, i, -1, true}, t).expanded(open)
		if pg.icon != '' {
			row.child(icon(pg.icon).size(16))
		}
		row.child(ui.div().grow().min_w(0).overflow_hidden().child(ui.text(pg.title)))
		if subs.len > 0 {
			row.child(ui.div().test_id('${s.id}-expand-${i}').size(20).center().shrink_0().rounded(t.radius_md).hover(ui.Patch{
				background: p.press
			}).on_click(core.handler_with(SettingsNav{s.st, s.picked, i, -1, false},
				settings_expand)).child(icon(if open {
				icon_chevron_down
			} else {
				icon_chevron_right
			}).size(16)))
		}
		mut item := ui.div().flex_col().child(row)
		if open {
			mut sub :=
				ui.div().flex_col().gap(4).pl(10).py(2).border_l(1).border_color(p.line_strong)
			for g in subs {
				title := pg.groups[g].title
				on_g := i == shown && s.st.gpage == i && s.st.group == g
				mut r :=
					card_row('${s.id}-group-${i}-${g}', title, on_g, SettingsNav{s.st, s.picked, i, g, false}, t)
				sub.child(r.child(ui.text(title)))
			}
			item.child(ui.div().flex_col().pl(14).child(sub)) // the rows span the column
		}
		list.child(item)
	}
	return col.child(list)
}

// cards_row: the title over the description, at most 60 % of the row, the control on
// the right.
fn cards_row(r SettingsRow, t &Theme) &ui.Div {
	if r.title == '' && r.description == '' { // the control is the whole row
		return ui.div().flex_col().w_full().child(r.control)
	}
	mut left :=
		ui.div().flex_col().grow().min_w(0).child(ui.text(r.title).line_height(20.0 / 14)) // 20 px
	left.style.max_width = layout.pct(60)
	if r.description != '' {
		left.child(ui.text(r.description).color(t.p.text_2))
	}
	return ui.div().flex_row().items_center().justify_between().gap(12).role(.group).label(r.title).child(left).child(ui.div().shrink_0().child(r.control))
}

fn (mut s Settings) cards_page(t &Theme) &ui.Div {
	p := t.p
	mut col := ui.div().test_id('${s.id}-page').flex_col().grow().min_w(0).h_full()
	cur := s.shown()
	if cur < 0 || cur >= s.pages.len {
		return col
	}
	pg := s.pages[cur]
	mut head := ui.div().flex_col().gap(12).p(16).shrink_0().border_b(1).border_color(p.line)
	head.child(ui.div().flex_row().items_center().justify_between().gap(8).child(ui.div().role(.heading).label(pg.title).child(ui.text(pg.title).size(t.ui_text(.lg)).line_height(1.625))).child(ui.div().flex_row().items_center().gap(4).children(s.header)))
	if pg.description != '' {
		head.child(ui.text(pg.description).color(p.text_2))
	}
	mut body := ui.div().flex_col().grow().min_h(0).overflow_y_scroll().scroll_with(&s.st.scroll)
	for gi, g in pg.groups {
		rows := g.rows.filter(s.matches(it))
		if rows.len == 0 {
			continue
		}
		mut group := ui.div().flex_col().gap(16).p(16)
		if g.title != '' {
			mut title :=
				ui.div().flex_col().gap(4).child(ui.text(g.title).size(t.ui_text(.lg)).color(p.text_2).line_height(1.25))
			if g.description != '' {
				title.child(ui.text(g.description).color(p.text_2))
			}
			group.child(title)
		}
		mut box :=
			ui.div().flex_col().gap(16).p(16).border(1).border_color(p.line).rounded(t.radius_lg)
		for r in rows {
			box.child(cards_row(r, t))
		}
		s.heads[gi] = group
		body.child(group.child(box))
	}
	return col.child(head).child(body)
}
