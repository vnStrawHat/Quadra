// The editor, the preview, the outline panel and the empty state.
module main

import os
import quadra.core
import quadra.kit
import quadra.kit.md
import quadra.ui

const preview_pad = f32(32) // the preview column's top padding

fn (mut f Folio) main_area(lay Layout, p &Prefs, t &kit.Theme, mut cx core.ViewCx[Folio]) core.Element {
	mut tab := f.cur()
	if lay == .raw {
		return f.editor_pane(tab, t, mut cx).child(f.sync_el(tab, lay, mut cx))
	}
	if lay == .preview {
		return f.preview_pane(mut tab, lay, p, t, mut cx)
	}
	return ui.div().grow().min_h(0).child(kit.split('split', f.editor_pane(tab, t, mut cx), f.preview_pane(mut tab,
		lay, p, t, mut cx), p.split).on_resize(cx.listener(fn (mut f Folio, r &f32, mut cx core.ViewCx[Folio]) {
		f.prefs_with(mut cx.Cx, *r, fn (mut p Prefs, ratio f32, mut _ core.ViewCx[Prefs]) {
			p.split = ratio
		})
	})))
}

fn (mut f Folio) editor_pane(tab &Tab, t &kit.Theme, mut cx core.ViewCx[Folio]) &ui.Div {
	return ui.div().test_id('editor-pane').flex_col().grow().h_full().min_h(0).bg(t.p.canvas).child(ui.div().grow().min_h(0).child(cx.child_view(tab.editor)))
}

fn (mut f Folio) preview_pane(mut tab Tab, lay Layout, p &Prefs, t &kit.Theme, mut cx core.ViewCx[Folio]) core.Element {
	view := md.view(tab.text).track(tab.pos).memo(tab.memo).scale(p.scale).base(if tab.path == '' {
		os.getwd()
	} else {
		os.dir(tab.path)
	}).on_link(cx.listener(fn (mut f Folio, url &string, mut cx core.ViewCx[Folio]) {
		f.follow_link(*url, mut cx)
	}))
	column := ui.div().flex_col().w_full().max_w(1120).px(40).pt(preview_pad).pb(96).child(view)
	scroller :=
		ui.div().test_id('preview').grow().w_full().overflow_y_scroll().scroll_with(tab.scroll).flex_col().items_center().child(column)
	return ui.div().test_id('preview-pane').grow().h_full().min_h(0).flex_col().bg(t.p.canvas).child(f.sync_el(tab,
		lay, mut cx)).child(scroller)
}

// sync_el keeps the preview on the editor's line (Split) and finds the current section.
fn (mut f Folio) sync_el(tab &Tab, lay Layout, mut cx core.ViewCx[Folio]) &Sync {
	return &Sync{
		owner:    cx.handle()
		editor:   tab.editor
		pos:      tab.pos
		scroll:   tab.scroll
		st:       tab.sync
		follow:   lay == .split
		preview:  lay != .raw
		headings: tab.heads.map(it.line)
	}
}

// outline_panel lists the headings as a tree with indent guides; the section being read
// is marked, a click goes there, the field at the top filters (specs/15: 32 px, a search
// icon, a hairline below).
fn (mut f Folio) outline_panel(p &Prefs, t &kit.Theme, mut cx core.ViewCx[Folio]) core.Element {
	tab := f.cur()
	heads := tab.heads
	mut list := ui.div().flex_col().grow().min_h(0).overflow_y_scroll().py(4)
	mut base := 6 // the top heading level present
	for h in heads {
		base = if h.level < base { h.level } else { base }
	}
	mut shown := 0
	for k, h in heads {
		if f.query != '' && !h.text.to_lower().contains(f.query) {
			continue
		}
		shown++
		on := k == tab.sync.section
		mut row :=
			ui.div().test_id('outline-${k}').flex_row().items_center().h(t.row_h).pl(8).pr(8).cursor(.pointer).role(.treeitem).label(h.text).selected(on).overflow_hidden()
		for _ in 0 .. h.level - base {
			row.child(ui.div().w(20).h_full().shrink_0().flex_row().justify_center().child(ui.div().w(1).h_full().bg(t.p.text.alpha(0.1))))
		}
		if on {
			row.bg(t.p.press).text_color(t.p.text)
		} else {
			row.text_color(t.p.text).hover(ui.Patch{
				background: t.p.panel_hover
				text_color: t.p.text
			})
		}
		row.child(ui.div().pl(4).child(ui.text(if h.text == '' {
			'(untitled)'
		} else {
			h.text
		}).truncate()))
		row.on_click(core.listener_with[Folio, int, core.ClickEvent](cx.entity, h.line, fn (mut f Folio, line int, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
			f.reveal_line(line, mut cx)
		}))
		list.child(row)
	}
	if shown == 0 {
		hint := if heads.len == 0 { 'No headings yet.' } else { 'No matching headings.' }
		list.child(ui.div().px(12).py(8).child(ui.text(hint).color(t.p.text_3)))
	}
	search :=
		ui.div().flex_row().items_center().gap(6).h(32).px(8).shrink_0().border_b(1).border_color(t.p.line_strong).child(kit.icon(kit.icon_search).size(14).color(t.p.text_2)).child(ui.div().grow().child(cx.child_view(f.filter)))
	return ui.div().test_id('outline').flex_col().w(p.outline_w).shrink_0().h_full().bg(t.p.panel).border_r(1).border_color(t.p.line_strong).child(search).child(list)
}

// Hint is one action of the empty state.
struct Hint {
	id    string
	label string
	cmd   Cmd
	keys  string
	glyph string
}

const hints = [
	Hint{'empty-new', 'New document', .new_doc, 'ctrl-n', kit.icon_plus},
	Hint{'empty-open', 'Open file', .open, 'ctrl-o', kit.icon_folder},
	Hint{'empty-palette', 'Command palette', .palette, 'ctrl-shift-p', kit.icon_command},
]

// section_head is a welcome-page section title: 10 px uppercase mono, muted, then a rule.
fn section_head(title string, t &kit.Theme) core.Element {
	return ui.div().flex_row().items_center().gap(8).px(4).pb(8).child(ui.text(title.to_upper()).family(t.mono_family).size(t.font_size(.xs)).color(t.p.text_2)).child(ui.div().grow().h(1).bg(t.p.line))
}

// empty_state: no document open, a welcome page (specs/15). Sections headed by a small
// uppercase mono label and a hairline; 28 px rows with the shortcut muted at the right.
fn (mut f Folio) empty_state(p &Prefs, t &kit.Theme, mut cx core.ViewCx[Folio]) core.Element {
	mut col :=
		ui.div().test_id('empty').flex_col().gap(2).w(360).child(section_head('Get started', t))
	for h in hints {
		col.child(ui.div().test_id(h.id).flex_row().items_center().gap(8).h(28).px(8).rounded(t.radius_md).cursor(.pointer).hover(ui.Patch{
			background: t.p.panel_hover
		}).child(kit.icon(h.glyph).size(14).color(t.p.text_2)).child(ui.text(h.label)).child(ui.div().grow()).child(ui.text(kit.key_label(h.keys)).size(t.font_size(.sm)).color(t.p.text_2)).on_click(core.listener_with[Folio, Cmd, core.ClickEvent](cx.entity,
			h.cmd, fn (mut f Folio, c Cmd, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
			f.run(c, mut cx)
		})))
	}
	recent := p.recent.filter(os.is_file(it))
	if recent.len > 0 {
		col.child(ui.div().pt(16).child(section_head('Recent', t)))
		for k, path in recent {
			if k >= 5 {
				break
			}
			col.child(ui.div().test_id('recent-${k}').flex_row().items_center().gap(6).h(28).px(8).rounded(t.radius_md).cursor(.pointer).hover(ui.Patch{
				background: t.p.panel_hover
			}).child(ui.text(os.file_name(path))).child(ui.text(display_path(os.dir(path))).size(t.font_size(.sm)).color(t.p.text_2)).on_click(core.listener_with[Folio, string, core.ClickEvent](cx.entity,
				path, fn (mut f Folio, path string, _ &core.ClickEvent, mut cx core.ViewCx[Folio]) {
				f.open_path(path, mut cx)
			})))
		}
	}
	return ui.div().grow().h_full().flex_col().items_center().justify_center().bg(t.p.canvas).child(col)
}
