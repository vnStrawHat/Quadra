// Notifications (specs/15): cards stacked at the window's bottom right, 448 wide and 12
// from the edges, above the status bar. Unlike toasts they stay until closed, unless
// given a timeout; they may offer actions.
module kit

import quadra.core
import quadra.prim
import quadra.ui

pub struct Notice {
pub:
	id      int
	win     u32
	kind    ToastKind
	title   string
	body    string
	actions []NoticeAction
	expires f64 // app clock ms when it closes by itself; 0 never
}

// NoticeAction is a button on a notification; running it also closes the notification.
pub struct NoticeAction {
pub:
	label   string
	primary bool // the tinted button; the others are outlined
	run     fn (mut cx core.Cx) = unsafe { nil }
}

@[params]
pub struct NoticeOptions {
pub:
	actions    []NoticeAction
	timeout_ms int // closes by itself after this long; 0 stays until closed
}

// notify shows a notification in the current window and returns its id.
pub fn notify(mut cx core.Cx, kind ToastKind, title string, body string, opts NoticeOptions) int {
	mut ov, id := add_overlay(cx) or { return 0 }
	mut timeout := opts.timeout_ms
	if timeout == 0 && opts.actions.len == 0 && theme_of(cx.app).ctl.tinted_notices {
		timeout = tinted_hide_ms
	}
	expires := if timeout > 0 { cx.app.plat.now_ms() + timeout } else { 0 }
	ov.notices << Notice{id, win_id(cx), kind, title, body, opts.actions, expires}
	cx.touch_global[Overlays]()
	if timeout > 0 {
		cx.app.after(timeout, expire_notices)
	}
	return id
}

// expire_notices closes the notifications whose time is up (no closure per notification).
fn expire_notices(mut app core.App) {
	mut cx := core.Cx{
		app: app
	}
	mut ov := cx.peek_global[Overlays]() or { return }
	now := app.plat.now_ms()
	n := ov.notices.len
	ov.notices = ov.notices.filter(it.expires == 0 || it.expires > now)
	if ov.notices.len != n {
		cx.touch_global[Overlays]()
	}
}

struct NoticeRun {
	id  int
	run fn (mut cx core.Cx) = unsafe { nil }
}

fn run_notice_action(a NoticeRun, _ &core.ClickEvent, mut cx core.Cx) {
	dismiss_notice(mut cx, a.id)
	if a.run != unsafe { nil } {
		a.run(mut cx)
	}
}

pub fn dismiss_notice(mut cx core.Cx, id int) {
	mut ov := cx.peek_global[Overlays]() or { return }
	ov.notices = ov.notices.filter(it.id != id)
	cx.touch_global[Overlays]()
}

fn dismiss_on(id int, _ &core.ClickEvent, mut cx core.Cx) {
	dismiss_notice(mut cx, id)
}

// notice_stack is the column of cards, newest at the bottom, anchored to the bottom right.
fn notice_stack(notices []Notice, t &Theme, sz prim.Size) core.Element {
	mut stack := ui.div().flex_col().gap(8).w(448)
	for n in notices {
		stack.child(notice_card(n, t))
	}
	return ui.anchored(stack).at(prim.rect(sz.width - 12, sz.height - t.statusbar_h - 12, 0, 0)).placement(.top_end).layer(5001)
}

// notice_card: level 2 on the panel tone, 12 padding, the status icon, the title over the
// muted body, a close button at the top right.
fn notice_card(n Notice, t &Theme) core.Element {
	p := t.p
	color, glyph := match n.kind {
		.info { p.accent_text, icon_info }
		.success { p.ok, icon_circle_check }
		.warning { p.warn, icon_triangle_alert }
		.error { p.error, icon_circle_x }
	}

	mut text := ui.div().flex_col().gap(2).grow().min_w(0).child(ui.text(n.title))
	if n.body != '' {
		text.child(ui.text(n.body).color(p.text_2))
	}
	if n.actions.len > 0 {
		mut row := ui.div().flex_row().justify_end().gap(4).pt(8)
		for i, a in n.actions {
			mut b := button('notice-${n.id}-${i}').label(a.label).size(.sm).on_click(core.handler_with(NoticeRun{n.id, a.run},
				run_notice_action))
			row.child(if a.primary { b.primary() } else { b.outline() })
		}
		text.child(row)
	}
	close := ui.div().test_id('notice-close-${n.id}').size(22).center().shrink_0().rounded(t.radius_md).cursor(.pointer).text_color(p.text_2).hover(ui.Patch{
		background: p.panel_hover
	}).role(.button).label('Close').on_click(core.handler_with(n.id, dismiss_on)).child(icon(icon_x).size(14))
	return ui.div().test_id('notice-${n.id}').flex_row().items_start().gap(8).p(12).bg(p.panel).rounded(t.radius_xl).border(1).border_color(p.line).shadow_level(2).text_size(t.ui_text(.md)).text_color(p.text).role(.alert).label(n.title).child(ui.div().pt(3).child(icon(glyph).size(16).color(color))).child(text).child(close)
}
