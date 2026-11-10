// Notifications with Controls.tinted_notices (specs/15): one stack with the toasts at the
// window's bottom right, 16 from the edges, 382 wide; each card tinted with its kind.
module kit

import quadra.core
import quadra.prim
import quadra.ui

const tinted_hide_ms = 5000 // notifications without actions hide after this long

fn tinted_stack(notices []Notice, toasts []Toast, t &Theme, sz prim.Size) core.Element {
	mut stack := ui.div().flex_col().gap(t.sp(8)).w(t.sp(382))
	for toast in toasts {
		stack.child(tinted_card(Notice{
			id:   -toast.id
			kind: toast.kind
			body: toast.text
		}, t))
	}
	for n in notices {
		stack.child(tinted_card(n, t))
	}
	return ui.anchored(stack).at(prim.rect(sz.width - 16, sz.height - 16, 0, 0)).placement(.top_end).layer(5001)
}

// tinted_card: the kind color at 4 % with a 30 % border, the title (14 semibold) and the
// text in that color, the icon at 16 / 18, a 20 close button at the top right while hovered.
fn tinted_card(n Notice, t &Theme) core.Element {
	p := t.p
	color, glyph := match n.kind {
		.info { p.info, icon_info }
		.success { p.ok, icon_circle_check }
		.warning { p.warn, icon_triangle_alert }
		.error { p.error, icon_circle_x }
	}

	mut text := ui.div().flex_col().grow().min_w(0).pl(t.sp(24))
	if n.title != '' {
		text.child(ui.div().font_weight(600).child(ui.text(n.title)))
	}
	if n.body != '' {
		text.child(ui.text(n.body))
	}
	if n.actions.len > 0 {
		mut row := ui.div().flex_row().justify_end().gap(t.sp(4)).pt(t.sp(8))
		for i, a in n.actions {
			mut b := button('notice-${n.id}-${i}').label(a.label).size(.sm).on_click(core.handler_with(NoticeRun{n.id, a.run},
				run_notice_action))
			row.child(if a.primary { b.primary() } else { b.outline() })
		}
		text.child(row)
	}
	name := if n.title != '' { n.title } else { n.body }
	mut box :=
		ui.div().group().test_id('notice-${n.id}').flex_row().gap(t.sp(12)).px(t.sp(16)).py(t.sp(14)).bg(color.alpha(0.04)).rounded(t.radius_xl).border(1).border_color(color.alpha(0.3)).shadow_level(2).text_size(t.ui_text(.md)).text_color(color).role(.alert).label(name)
	box.child(ui.div().absolute().top(t.sp(18)).left(t.sp(16)).child(icon(glyph).size(16).color(color))).child(text)
	if n.id > 0 { // toasts (negative ids) expire on their own
		// Shown only while the card is hovered (a hover group).
		box.child(ui.div().absolute().top(t.sp(4)).right(t.sp(4)).opacity(0).group_hover(ui.Patch{
			opacity: 1
		}).test_id('notice-close-${n.id}').size(t.height(.xs)).center().rounded(t.radius_md).cursor(.pointer).text_color(p.text_2).hover(ui.Patch{
			background: p.panel_hover.alpha(0.5)
		}).role(.button).label('Close').on_click(core.handler_with(n.id, dismiss_on)).child(icon(icon_x).size(12)))
	}
	return box
}
