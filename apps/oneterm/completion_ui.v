// The completion overlay (specs/22): cursor-anchored, flipped above the row when it does not
// fit below, a badge per row. Reads the controller only; keys are in term_ext.v.
module main

import completion
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.prim
import quadra.ui

fn (x &TermExt) overlay_row(s completion.Suggestion, selected bool, t &kit.Theme) core.Element {
	split := if s.match_len < s.text.len { s.match_len } else { s.text.len }
	body := if selected { prim.white } else { t.p.text }
	matched := if selected { prim.white } else { t.p.accent }
	muted := if selected { prim.white } else { t.p.text_2 }
	weight := if selected { 700 } else { 400 }
	mut row :=
		ui.div().flex_row().items_center().gap(16).px(8).py(2).rounded(4).child(ui.div().flex_row().child(ui.text(s.text[..split]).color(matched).weight(weight)).child(ui.text(s.text[split..]).color(body).weight(weight))).child(ui.div().grow())
	if selected { // the theme's base blue: the first series colour
		row.bg(if t.series.len > 0 { t.series[0] } else { t.p.info })
	}
	if s.description != '' {
		row.child(ui.text(s.description).italic().color(muted))
	}
	// The kind as a muted lowercase word, white on the selected row.
	row.child(ui.text(s.kind.str()).size(t.font_size(.xs)).color(muted))
	return row
}

// overlay_element is the positioned list, none while hidden or scrolled back.
fn (mut x TermExt) overlay_element(tm &term.Terminal, t &kit.Theme) ?core.Element {
	if !x.has_anchor || !x.ctl.is_visible() || tm.offset > 0 {
		return none
	}
	all := x.ctl.suggestions()
	offset, count := completion.visible_window(all.len, x.ctl.selected(), x.ctl.max_visible())
	above, below := offset, all.len - offset - count
	muted := t.p.text_2
	mut box :=
		ui.div().test_id('completion').flex_col().min_w(180).rounded(6).overflow_hidden().p(2).border(1).border_color(t.p.line).bg(t.p.overlay_bg).shadow_level(2).text_size(t.font_size(.sm))
	crumb := x.ctl.breadcrumb(x.line, x.line.len)
	if crumb.len > 0 {
		box.child(ui.div().px(8).py(2).child(ui.text(crumb.join(' › ') + ' ›').size(t.font_size(.xs)).color(muted)))
	}
	if above > 0 {
		box.child(ui.div().px(8).py(2).child(ui.text('↑ ${above} more').size(t.font_size(.xs)).color(muted)))
	}
	for i in offset .. offset + count {
		box.child(x.overlay_row(all[i], i == x.ctl.selected(), t))
	}
	if below > 0 {
		box.child(ui.div().px(8).py(2).child(ui.text('↓ ${below} more').size(t.font_size(.xs)).color(muted)))
	}
	cell := prim.rect(tm.bounds.x + f32(x.anchor.col) * tm.cell.width, tm.bounds.y +
		f32(x.anchor.line) * tm.cell.height, tm.cell.width, tm.cell.height)
	rows := count + (if above > 0 { 1 } else { 0 }) + (if below > 0 { 1 } else { 0 })
	est := tm.cell.height * f32(rows) + 10 // rows times line height, as in the original
	fits := est <= tm.bounds.y + tm.bounds.height - cell.y - tm.cell.height
	return ui.anchored(box).at(cell).placement(if fits {
		ui.Placement.bottom_start
	} else {
		ui.Placement.top_start
	}).gap(0).layer(3)
}
