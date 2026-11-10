// Small helpers shared by the dialogs (specs/26): form fields, the scrolling body, toasts.
module sessionui

import quadra.core
import quadra.kit
import quadra.prim
import quadra.ui

fn labelled(label string, required bool, control core.Element) core.Element {
	return kit.field(label, control, '', '', required)
}

// dialog_body is the scrolling form column; the footer stays outside it, in the dialog frame.
fn dialog_body(t &kit.Theme, win_h f32, rows []core.Element) core.Element {
	mut col := ui.div().flex_col().gap(12).w_full().text_color(t.p.text)
	for r in rows {
		col.child(r)
	}
	return ui.div().id('dialog-body').w_full().max_h(prim.max(win_h - 260, 240)).overflow_y_scroll().child(col)
}

// inline_error is the failure of the last attempt, inside the dialog.
fn inline_error(t &kit.Theme, message string, note string) core.Element {
	mut box :=
		ui.div().flex_col().gap(t.sp(4)).px(12).py(8).rounded(t.radius_md).bg(t.p.error.alpha(0.1)).role(.alert).child(ui.text(message).size(t.font_size(.sm)).color(t.p.error))
	if note != '' {
		box.child(ui.text(note).size(t.font_size(.xs)).color(t.p.text_2))
	}
	return box
}

fn warn(mut cx core.Cx, text string) {
	kit.push_toast(mut cx, .warning, text)
}

fn info(mut cx core.Cx, text string) {
	kit.push_toast(mut cx, .info, text)
}

fn success(mut cx core.Cx, text string) {
	kit.push_toast(mut cx, .success, text)
}
