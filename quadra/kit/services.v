// Overlay services: modals, popups (menus, popovers, selects) and toasts.
module kit

import quadra.core
import quadra.prim
import quadra.ui

// icon wraps a Lucide icon body (kit.icon_*) into a tintable SVG element.
// The stroke is 1.8 on the 24 grid: 1.2 px at 16 px, the outline weight of specs/15.
// Kit builds make icons at every layout: the wrapped source is made once per body.
pub fn icon(body string) &ui.SvgEl {
	if src := icon_sources.m[body] {
		return ui.icon(src)
	}
	src := '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${body}</svg>'
	if icon_sources.m.len < 1024 { // bodies are constants; a few made on the fly stay bounded
		mut c := unsafe { icon_sources }
		c.m[body] = src
	}
	return ui.icon(src)
}

struct IconSources {
mut:
	m map[string]string
}

// One per process, like the elements built from it: main thread only.
const icon_sources = &IconSources{}

// add_overlay reserves the next overlay id. The state is mutated in place (no update
// closure, specs/14); callers then call cx.touch_global[Overlays]().
fn add_overlay(cx &core.Cx) ?(&Overlays, int) {
	mut ov := cx.peek_global[Overlays]()?
	ov.next_id++
	return ov, ov.next_id
}

fn win_id(cx &core.Cx) u32 {
	return if cx.win != unsafe { nil } { cx.win.id } else { 0 }
}

pub enum ModalSide {
	center
	left // a drawer: full height at the window's left edge
	right
	bottom // full width at the bottom; `width` is then its height
	top    // a spotlight (command palettes): near the top, no title bar, no dimming
}

pub struct ModalOptions {
pub:
	side   ModalSide
	title  string
	body   fn () core.Element = unsafe { nil }
	footer ?fn () core.Element
	// body_with and footer_with are body and footer without a closure (kit.build_with,
	// kit.view_body): a closure made in a handler is never freed (specs/14).
	body_with   core.BoundRef[Theme, core.Element]
	footer_with core.BoundRef[Theme, core.Element]
	width       f32 = 440
	// muted text under the title (One: the body when there is no body)
	description string
	// an alert (a question that needs an answer): with Controls.card_dialogs it has no
	// close button and outside clicks do not close it
	alert bool
}

// open_modal shows a dialog in the current window; escape closes it.
pub fn open_modal(mut cx core.Cx, opts ModalOptions) int {
	mut ov, id := add_overlay(cx) or { return 0 }
	cx.keep_build()
	mut body := opts.body_with
	if !body.is_set() && opts.body != unsafe { nil } {
		body = build_with(Builder{opts.body}, run_builder)
	}
	mut footer := opts.footer_with
	if f := opts.footer {
		footer = build_with(Builder{f}, run_builder)
	}
	ov.modals << Modal{id, win_id(cx), opts.side, opts.title, body, footer, opts.width, opts.description, opts.alert}
	cx.touch_global[Overlays]()
	return id
}

// build_with pairs data with a builder for ModalOptions.body_with and footer_with:
// `kit.build_with(args, fn (a Args, t &kit.Theme) core.Element {...})`. D must not be a
// pointer type: wrap a pointer in a struct.
pub fn build_with[D](data D, f fn (d D, t &Theme) core.Element) core.BoundRef[Theme, core.Element] {
	return core.bind_fn_ref[D, Theme, core.Element](data, f)
}

// view_body is a modal body showing view v.
pub fn view_body(v core.AnyView) core.BoundRef[Theme, core.Element] {
	return build_with(v, show_view)
}

fn show_view(v core.AnyView, _ &Theme) core.Element {
	return core.view_element(v)
}

pub fn close_modal(mut cx core.Cx, id int) {
	mut ov := cx.peek_global[Overlays]() or { return }
	ov.modals = ov.modals.filter(it.id != id)
	ov.free_idle(mut cx)
	cx.touch_global[Overlays]()
}

// close_modal_on is a listener body closing modal id on any event.
fn close_modal_on[E](id int, _ &E, mut cx core.Cx) {
	close_modal(mut cx, id)
}

// Builder wraps a builder closure that callers made, so it can be bound data.
struct Builder {
	f fn () core.Element = unsafe { nil }
}

fn run_builder(b Builder, _ &Theme) core.Element {
	return b.f()
}

// open_popup shows content next to an anchor rect; outside clicks and escape close it.
pub fn open_popup(mut cx core.Cx, anchor prim.Rect, placement ui.Placement, build fn () core.Element) int {
	return open_popup_with(mut cx, anchor, placement, Builder{build}, run_builder)
}

// open_popup_with is open_popup with the data its builder needs passed in, so the builder
// captures nothing (specs/14). D must not be a pointer type.
pub fn open_popup_with[D](mut cx core.Cx, anchor prim.Rect, placement ui.Placement, data D, build fn (d D, t &Theme) core.Element) int {
	mut ov, id := add_overlay(cx) or { return 0 }
	cx.keep_build()
	focus := if cx.win != unsafe { nil } { cx.win.focused } else { u64(0) }
	ov.popups << Popup{id, win_id(cx), anchor, placement, core.bind_fn_ref[D, Theme, core.Element](data,
		build), focus}
	cx.touch_global[Overlays]()
	return id
}

pub fn close_popup(mut cx core.Cx, id int) {
	mut ov := cx.peek_global[Overlays]() or { return }
	for p in ov.popups {
		if p.id == id {
			restore_focus(mut cx, p)
		}
	}
	ov.popups = ov.popups.filter(it.id != id)
	ov.free_idle(mut cx)
	cx.touch_global[Overlays]()
}

fn close_popup_on[E](id int, _ &E, mut cx core.Cx) {
	close_popup(mut cx, id)
}

pub fn close_popups(mut cx core.Cx) {
	mut ov := cx.peek_global[Overlays]() or { return }
	win := win_id(cx)
	for p in ov.popups {
		if p.win == win {
			restore_focus(mut cx, p) // the first one opened holds the focus from before
			break
		}
	}
	ov.popups = ov.popups.filter(it.win != win)
	ov.free_idle(mut cx)
	cx.touch_global[Overlays]()
}

// restore_focus gives the focus back to what had it when the popup opened (its trigger),
// so keys and actions reach the window again.
fn restore_focus(mut cx core.Cx, p Popup) {
	if cx.win != unsafe { nil } && cx.win.id == p.win {
		cx.win.set_focus(p.focus, cx.win.focus_visible)
	}
}

// free_idle releases the builds kept for overlays (core keep_build) in windows that have none.
fn (ov &Overlays) free_idle(mut cx core.Cx) {
	for w in cx.app.windows {
		if !ov.popups.any(it.win == w.id) && !ov.modals.any(it.win == w.id) {
			cx.app.free_kept(w.id)
		}
	}
}

// push_toast shows a notification at the bottom center for 10 seconds.
pub fn push_toast(mut cx core.Cx, kind ToastKind, text string) {
	mut ov, id := add_overlay(cx) or { return }
	ov.toasts << Toast{id, win_id(cx), kind, text}
	cx.touch_global[Overlays]()
	cx.app.after(if theme_of(cx.app).ctl.tinted_notices { tinted_hide_ms } else { 10000 },
		expire_toast)
}

// expire_toast removes the oldest toast: every toast lives 10 s, so they expire in order.
fn expire_toast(mut app core.App) {
	mut cx := core.Cx{
		app: app
	}
	mut ov := cx.peek_global[Overlays]() or { return }
	if ov.toasts.len > 0 {
		ov.toasts.delete(0)
		cx.touch_global[Overlays]()
	}
}

// modal_frame draws a dialog (specs/15): a level-2 overlay hanging 80 px from the window
// top, no backdrop (a click outside closes it); drawers sit against their edge.
fn modal_frame(m Modal, t &Theme, win prim.Size) core.Element {
	if t.ctl.card_dialogs && m.side == .center {
		return card_dialog(m, t, win)
	}
	id := m.id
	p := t.p
	mut dialog := ui.div().flex_col().bg(p.overlay_bg).border(1).border_color(p.line).shadow_level(2).role(.dialog).label(m.title).test_id('modal-${id}').on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
		cx.stop_propagation() // clicks inside the dialog never reach the backdrop
	}))
	mut backdrop := ui.div().size_full().bg(p.scrim).on_mouse_down(core.handler_with(id,
		close_modal_on[core.MouseDownEvent]))
	if m.side == .top {
		return backdrop.flex_col().items_center().pt(80).child(dialog.w(m.width).max_w(720).rounded(t.radius_xl).overflow_hidden().child(m.body.run(t)))
	}
	dialog.child(ui.div().flex_row().items_center().gap(4).px(12).pt(8).pb(4).child(ui.text(m.title).size(t.scaled(t.text_lg))).child(ui.div().grow()).child(ui.div().test_id('modal-close').size(22).center().rounded(t.radius_md).cursor(.pointer).text_color(p.text_2).hover(ui.Patch{
		background: p.panel_hover
	}).on_click(core.handler_with(id, close_modal_on[core.ClickEvent])).child(icon(icon_x).size(14))))
	mut body := ui.div().px(12).py(8).grow().text_color(p.text_2)
	if m.body.is_set() {
		body.child(m.body.run(t))
	} else {
		body.child(ui.text(m.description))
	}
	dialog.child(body)
	if m.footer.is_set() {
		dialog.child(ui.div().flex_row().justify_end().gap(4).p(8).border_t(1).border_color(p.line).child(m.footer.run(t)))
	}
	match m.side {
		.center {
			dialog.w(m.width).max_w(720).rounded(t.radius_xl)
			backdrop.flex_col().items_center().pt(80)
		}
		.left, .right { // a drawer: full height against the edge
			dialog.w(m.width).h_full()
			backdrop.flex_row()
			if m.side == .right {
				backdrop.justify_end()
			}
		}
		.bottom {
			dialog.w_full().h(m.width)
			backdrop.flex_col().justify_end()
		}
		.top {}
	}

	return backdrop.child(dialog)
}

fn toast_card(toast Toast, t &Theme) core.Element {
	accent := match toast.kind {
		.info { t.p.text_2 }
		.success { t.p.ok }
		.warning { t.p.warn }
		.error { t.p.error }
	}

	glyph := match toast.kind {
		.info { icon_info }
		.success { icon_circle_check }
		.warning { icon_triangle_alert }
		.error { icon_circle_x }
	}

	// A status toast (specs/15): level 2 on the panel tone, a status icon and the text.
	return ui.div().flex_row().items_center().gap(8).max_w(448).pl(10).pr(10).py(6).bg(t.p.panel).rounded(t.radius_xl).border(1).border_color(t.p.line).shadow_level(2).role(.alert).label(toast.text).child(icon(glyph).color(accent).size(14)).child(ui.text(toast.text).size(t.ui_text(.md)))
}
