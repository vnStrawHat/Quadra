// What a terminal tells the app (specs/22): toasts, progress, program status, and the overlays
// and status icons they drive.
module main

import math
import termlog
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.layout
import quadra.prim
import quadra.ui

fn (mut o OneTerm) space_of(m core.Model[term.Terminal]) ?&Space {
	for tab in o.tabs {
		for s in tab.spaces {
			if s.view.id == m.id {
				return s
			}
		}
	}
	return none
}

fn status_from(state ?term.ProgramState) ProgramStatus {
	st := state or { return .idle }
	return match st {
		.working { ProgramStatus.working }
		.blocked { ProgramStatus.blocked }
		.done { ProgramStatus.done }
		.error { ProgramStatus.error }
		.idle { ProgramStatus.idle }
	}
}

fn (mut o OneTerm) on_terminal_info(m core.Model[term.Terminal], e &term.TerminalInfo, mut cx core.ViewCx[OneTerm]) {
	mut s := o.space_of(m) or { return }
	match e.kind {
		.notification {
			toast(mut cx, .info, 'OneTerm', e.text)
		}
		.progress {
			s.progress = e.progress
		}
		.program_status {
			s.status = status_from(cx.peek(m).screen.program_status)
		}
		.changed {
			exited := cx.peek(m).exited != none
			// the process ending ends working and blocked; a result waits to be seen
			if exited && s.status in [.working, .blocked] {
				s.status = .idle
			}
			if exited && s.remote && !s.ssh_closed {
				s.ssh_closed = true
				mut c := cx.Cx
				if c.win == unsafe { nil } && cx.app.windows.len > 0 {
					c.win = cx.app.windows[0]
				}
				kit.push_toast(mut c, .error, 'SSH connection closed.')
				o.sync_active(mut cx)
			}
			o.follow_cwd_of(s, mut cx)
		}
		.shell_mark, .pointer, .icon_name {}
	}

	cx.notify()
}

// toast shows a notice from a callback that has no window (tasks, terminal events).
fn toast(mut cx core.ViewCx[OneTerm], kind kit.ToastKind, title string, body string) {
	mut c := cx.Cx
	if c.win == unsafe { nil } && cx.app.windows.len > 0 {
		c.win = cx.app.windows[0]
	}
	kit.notify(mut c, kind, title, body)
}

// progress_fraction is the bar's fill and color; none when no bar is shown.
fn progress_fraction(p term.Progress, t &kit.Theme) ?(f32, prim.Color) {
	frac := f32(prim.clamp(f32(p.percent), 0, 100) / 100)
	return match p.state {
		.remove { none }
		.set { frac, t.p.info }
		.error { frac, t.p.error }
		.paused { frac, t.p.warn }
		.indeterminate { f32(1), t.p.info }
	}
}

// progress_bar is the 5 px bar along the Space's top edge.
fn progress_bar(p term.Progress, t &kit.Theme) ?core.Element {
	frac, color := progress_fraction(p, t) or { return none }
	return ui.div().test_id('progress').absolute().top(0).left(0).right(0).h(5).bg(t.p.panel.alpha(0.4)).child(ui.div().h_full().w_len(layout.pct(frac * 100)).bg(color))
}

// StatusAnim is the paint-only animation of a status icon: a spin or a pulse. No re-render.
struct StatusAnim {
mut:
	child core.Element
	spin  bool
}

pub fn (mut a StatusAnim) request_layout(mut cx core.LayoutCx) layout.NodeId {
	return a.child.request_layout(mut cx)
}

pub fn (mut a StatusAnim) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	a.child.prepaint(bounds, mut cx)
}

pub fn (mut a StatusAnim) paint(bounds prim.Rect, mut cx core.PaintCx) {
	now := cx.now_ms()
	if a.spin {
		center := prim.Point{bounds.x + bounds.width / 2, bounds.y + bounds.height / 2}
		saved := cx.push_transform(center, prim.Point{}, 1, f32(math.fmod(now, 1000) / 1000 * 360))
		a.child.paint(bounds, mut cx)
		cx.restore_transform(saved)
	} else {
		saved := cx.opacity
		wave := f32(math.cos(math.fmod(now, 1200) / 1200 * math.tau)) * 0.5 + 0.5
		cx.opacity *= 0.35 + 0.65 * wave
		a.child.paint(bounds, mut cx)
		cx.opacity = saved
	}
	cx.request_paint_after(16)
}

// status_icon is a single-Space tab's OSC 7501 icon.
fn status_icon(st ProgramStatus, t &kit.Theme) ?core.Element {
	glyph, color, tip := match st {
		.idle { '', t.p.text, '' }
		.working { kit.icon_loader_circle, t.p.accent, 'Working' }
		.blocked { kit.icon_bell, t.p.warn, 'Waiting for you' }
		.done { kit.icon_circle_check, t.p.ok, 'Done' }
		.error { kit.icon_circle_x, t.p.error, 'Error' }
	}

	if glyph == '' {
		return none
	}
	icon := core.Element(kit.icon(glyph).size(t.sp(12)).color(color))
	if st == .working || st == .blocked {
		return kit.tooltip(StatusAnim{icon, st == .working}, tip)
	}
	return kit.tooltip(icon, tip)
}

fn (o &OneTerm) log_config() termlog.TerminalLogConfig {
	return termlog.TerminalLogConfig{
		enabled:    true
		directory:  o.cfg.logging.directory
		write_mode: if o.cfg.logging.write_mode == 'overwrite' {
			termlog.LogWriteMode.overwrite
		} else {
			termlog.LogWriteMode.append
		}
	}
}

fn (mut x TermExt) start_log(c termlog.TerminalLogConfig) !string {
	return x.log.start(c)!
}
