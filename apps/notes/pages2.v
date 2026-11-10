// The table of all notes and the settings page.
module main

import os
import quadra.app
import quadra.core
import quadra.kit
import quadra.kit.data
import quadra.ui

// refresh_table loads the current library into the table (call from listeners, not render).
fn (mut s Shell) refresh_table(mut cx core.ViewCx[Shell]) {
	if s.table_v == s.version {
		return
	}
	s.table_v = s.version
	n := cx.peek(s.notes)
	cx.update_view_with(s.table, table_columns(cx.Cx), fn (mut t data.Table, columns []data.Column, mut _ core.ViewCx[data.Table]) {
		for i, c in columns {
			t.columns[i].title = c.title
		}
	})
	mut rows := s.rows
	rows.notes = n.lib.notes.clone()
	rows.untitled = app.t(cx.Cx, 'note.untitled')
	rows.locale = app.locale(cx.Cx)
	data.set_rows_from(mut cx.Cx, s.table, n.lib.notes.len, rows)
}

fn (mut s Shell) table_page(mut cx core.ViewCx[Shell]) core.Element {
	return ui.div().flex_col().size_full().child(cx.child_view(s.table))
}

fn (mut s Shell) settings_page(mut cx core.ViewCx[Shell]) core.Element {
	p := s.prefs.get(cx.Cx)
	t := kit.theme(cx.Cx)
	theme_labels :=
		['settings.theme_system', 'settings.theme_light', 'settings.theme_dark'].map(app.t(cx.Cx, it))
	density_labels := ['settings.compact', 'settings.comfortable'].map(app.t(cx.Cx, it))
	theme := kit.segmented('set-theme', theme_labels, themes.index(p.theme)).on_change(cx.listener(fn (mut s Shell, i &int, mut cx core.ViewCx[Shell]) {
		update_prefs_with(mut cx.Cx, s.prefs, themes[*i], fn (mut p Prefs, v string, mut _ core.ViewCx[Prefs]) {
			p.theme = v
		})
	}))
	density := kit.segmented('set-density', density_labels, densities.index(p.density)).on_change(cx.listener(fn (mut s Shell, i &int, mut cx core.ViewCx[Shell]) {
		update_prefs_with(mut cx.Cx, s.prefs, densities[*i], fn (mut p Prefs, v string, mut _ core.ViewCx[Prefs]) {
			p.density = v
		})
	}))
	language := kit.segmented('set-language', ['English', 'Tiếng Việt'],
		languages.index(p.language)).on_change(cx.listener(fn (mut s Shell, i &int, mut cx core.ViewCx[Shell]) {
		update_prefs_with(mut cx.Cx, s.prefs, languages[*i], fn (mut p Prefs, v string, mut _ core.ViewCx[Prefs]) {
			p.language = v
		})
		s.relabel(mut cx)
	}))
	root := cx.peek(s.notes).root
	return ui.div().flex_col().px(32).py(28).w_full().max_w(720).child(ui.div().pb(12).child(kit.heading(app.t(cx.Cx,
		'settings.title'), 1))).child(settings_row(t, app.t(cx.Cx, 'settings.theme'), theme)).child(settings_row(t, app.t(cx.Cx,
		'settings.density'), density)).child(settings_row(t, app.t(cx.Cx, 'settings.language'),
		language)).child(settings_row(t, app.t(cx.Cx, 'settings.data'),
		ui.text(os.real_path(root)).family(t.mono_family).size(t.font_size(.sm)).color(t.p.text_2)))
}

// settings_row is a label on the left and its control on the right, over a hairline.
fn settings_row(t &kit.Theme, label string, control core.Element) core.Element {
	return ui.div().flex_row().items_center().gap(16).py(12).border_b(1).border_color(t.p.line).child(ui.div().w(180).shrink_0().child(ui.text(label))).child(control)
}
