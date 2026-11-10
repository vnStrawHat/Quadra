// Centered dialogs with Controls.card_dialogs (specs/15): the canvas fill, a tenth of the
// window from the top, padding 16, a semibold title, a close button at the top right,
// a footer without a line. Alerts have no close button and ignore outside clicks.
module kit

import quadra.core
import quadra.prim
import quadra.ui

fn card_dialog(m Modal, t &Theme, win prim.Size) core.Element {
	p := t.p
	mut dialog := ui.div().flex_col().w(prim.min(t.sp(m.width), win.width - 32)).p(t.sp(16)).bg(p.canvas).rounded(t.radius_xl).border(1).border_color(p.line).shadow_level(2).text_color(p.text).role(.dialog).label(m.title).test_id('modal-${m.id}').on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
		cx.stop_propagation() // clicks inside the dialog never reach the backdrop
	}))
	mut head := ui.div().flex_col().gap(t.sp(4))
	if m.title != '' {
		head.child(ui.div().role(.heading).font_weight(600).child(ui.text(m.title).size(t.ui_text(.lg)).line_height(1.25)))
	}
	if m.description != '' {
		head.child(ui.text(m.description).size(t.ui_text(.md)).color(p.text_2))
	}
	dialog.child(head)
	if m.body.is_set() {
		dialog.child(ui.div().flex_col().grow().pt(t.sp(8)).child(m.body.run(t)))
	}
	if m.footer.is_set() {
		dialog.child(ui.div().flex_row().justify_end().gap(t.sp(8)).pt(t.sp(16)).child(m.footer.run(t)))
	}
	if !m.alert {
		dialog.child(ui.div().absolute().top(8).right(8).test_id('modal-close').size(t.height(.sm)).center().rounded(t.radius_md).cursor(.pointer).hover(ui.Patch{
			background: p.panel_hover.alpha(0.5)
		}).on_click(core.handler_with(m.id, close_modal_on[core.ClickEvent])).child(icon(icon_x).size(14)))
	}
	mut backdrop := ui.div().size_full().bg(p.scrim).flex_col().items_center().pt(win.height / 10)
	if !m.alert {
		backdrop.on_mouse_down(core.handler_with(m.id, close_modal_on[core.MouseDownEvent]))
	}
	return backdrop.child(dialog)
}
