// Crash reports (specs/22, Services): the handler at start, the dialog on the next start. An
// elevated window keeps its reports in crashes/elevated/ and shows none.
module main

import os
import crash
import update
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

// install_crash_capture runs first thing: the platform filter, then the crash module, which
// chains it. Nothing is lost if either fails: the app still runs.
fn install_crash_capture(mut a core.App, dir string) string {
	elevated := is_restricted()
	store_dir := crash.crashes_dir_in(dir, elevated)
	a.plat.install_crash_handler(store_dir, crash_extra)
	paths := crash.prepare_capture_paths(dir, elevated) or {
		eprintln('crash capture: ${err.msg()}')
		return ''
	}
	crash.install(paths, version, true) or { eprintln('crash capture: ${err.msg()}') }
	return paths.identity
}

fn crash_extra() string {
	return ''
}

// show_crash_reports opens the newest report; each dismissal opens the next.
fn (mut o OneTerm) show_crash_reports(mut cx core.ViewCx[OneTerm]) {
	if is_restricted() || o.crash_shown {
		return
	}
	o.crash_shown = true
	home := if os.home_dir() != '' { ?string(os.home_dir()) } else { ?string(none) }
	o.crash_reports = crash.load_pending_reports(o.dir, false, home, o.crash_identity) or {
		eprintln('crash reports: ${err.msg()}')
		return
	}
	o.open_next_report(mut cx)
}

fn (mut o OneTerm) open_next_report(mut cx core.ViewCx[OneTerm]) {
	if o.crash_reports.len == 0 {
		return
	}
	report := o.crash_reports[0]
	o.crash_reports.delete(0)
	// As many lines as fill the original's 360 px box (less the field's padding and border).
	th := kit.theme(cx)
	lines := int((360 - 18) / (th.font_size(.md) * th.line_height))
	field := input.new_input(mut cx.Cx,
		id:        'crash-text'
		value:     report.contents
		multiline: true
		rows:      lines
		max_rows:  lines
		readonly:  true
	)
	mut holder := &ModalRef{}
	url := '${crash.new_issue_url(update.repository_default)}?title=Crash%20report'
	self_ := cx.self_model()
	holder.id = kit.open_modal(mut cx.Cx, kit.ModalOptions{
		title:       'OneTerm closed unexpectedly'
		width:       760
		body_with:   kit.build_with(field.any(), fn (field core.AnyView, _ &kit.Theme) core.Element {
			return ui.div().flex_col().gap(12).w_full().child(ui.text('OneTerm detected a crash report from a previous run.')).child(ui.div().flex_col().w_full().child(core.view_element(field)))
		})
		footer_with: kit.build_with(CrashFoot{CrashAct{self_, report, holder}, url}, crash_foot)
	})
	cx.notify()
}

struct CrashFoot {
	act CrashAct
	url string
}

fn crash_foot(f CrashFoot, _ &kit.Theme) core.Element {
	return ui.div().flex_row().justify_end().gap(8).child(kit.button('crash-dismiss').label('Dismiss').on_click(core.handler_with(f.act,
		crash_dismiss))).child(kit.button('crash-copy').label('Copy').on_click(core.handler_with(f.act.report.contents,
		crash_copy))).child(kit.button('crash-issue').label('Create Issue').primary().on_click(core.handler_with(f.url,
		crash_issue)))
}

struct CrashAct {
	one    core.Model[OneTerm]
	report crash.PendingCrashReport
	holder &ModalRef = unsafe { nil }
}

fn crash_dismiss(a CrashAct, _ &core.ClickEvent, mut cx core.Cx) {
	kit.close_modal(mut cx, a.holder.id)
	mut o := cx.peek(a.one)
	crash.delete_pending_report(o.dir, false, a.report.path) or {
		eprintln('crash report not deleted: ${err.msg()}')
	}
	cx.update_view(core.View[OneTerm]{a.one.id, a.one.gen}, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.open_next_report(mut cx)
	})
}

fn crash_copy(contents string, _ &core.ClickEvent, mut cx core.Cx) {
	cx.clipboard_write(contents)
}

fn crash_issue(url string, _ &core.ClickEvent, mut cx core.Cx) {
	cx.open_external(url)
}
