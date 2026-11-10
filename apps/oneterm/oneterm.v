// The window (specs/22): title bar, the dock area (strip or rail, Spaces, right dock), status bar.
module main

import os
import time
import completion
import crash
import sessionui
import sftpui
import ssh
import store
import vault
import update
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.kit.term
import quadra.kit.work
import quadra.ui

// Space is one pane of a tab: empty, or a terminal.
@[heap]
pub struct Space {
pub mut:
	id         string
	number     int
	focus      core.FocusHandle // holds the focus while the Space is empty
	view       core.View[term.Terminal]
	shell      ShellSpec
	channel    string // broadcast channel, '' = none
	pty        &BroadcastPty = unsafe { nil }
	remote     bool // an SSH session (the right dock follows it)
	session    &ssh.Session = unsafe { nil } // the connection of a remote Space
	duplicate  sessionui.DuplicateSpec // how to duplicate it; never holds a secret
	ssh_closed bool                    // the link ended: banner up, input off
	ext        &TermExt = unsafe { nil } // completion, classes, log (term_ext.v)
	status     ProgramStatus
	progress   term.Progress
}

// TermTab is a tab: a tree of Spaces, one of them active.
@[heap]
pub struct TermTab {
pub mut:
	id     string
	name   string // the manual title
	root   work.PaneNode
	spaces []&Space
	active string
}

@[heap]
pub struct OneTerm {
pub mut:
	cfg         TerminalConfig
	ui          UiConfig
	update      UpdateConfig
	dir         string
	themes      []ThemeInfo // refreshed when the settings window opens
	last_light  string = 'One Light'
	last_dark   string = 'One Dark'
	tabs        []&TermTab
	active      string
	next        int          = 1
	spawn       term.SpawnFn = unsafe { nil } // tests install a FakePty; else the system ConPTY
	first_shell string // --elevated-shell
	clock       string
	stats_on    bool // --stats
	stats_text  string
	stats_last  term.Stats
	stats_view  u32
	mem         MemProbe // --mem, --tabs (mem_report.v)
	// the right dock
	dock_open  bool
	dock_pref  f32 = 480
	dock_split f32 = 0.5
	// the status bar's indicators
	git       GitState
	net_text  string
	net_last  ?NetStats
	res       &ResourceBox = unsafe { nil }
	res_busy  bool
	res_ticks int
	res_prev  ResourceSample
	// the HUD and the rail
	fps       int = -1 // -1: no full second counted yet (the HUD shows `-`)
	fps_last  int = -1
	hud       HudGpu
	rail_live f32 // the rail's width during a drag; 0: use the preference
	rail_drag bool
	rail_list &ui.ScrollState = &ui.ScrollState{} // the rail's rows; scrolled: a rule under the header
	// the lock screen
	vault        &vault.VaultState = unsafe { nil }
	lock_input   core.View[input.TextInput]
	font_zoom    f32 // px added to the shared terminal font size by the zoom keys (not saved)
	lock_error   string
	vault_dialog core.Model[VaultDialog] // the dialog waiting for a job, id 0 when none
	last_input   time.Time = time.now()
	launching    bool // a `runas` is outstanding
	// the services
	history        completion.History
	sessions       &store.SessionStore = unsafe { nil }
	upd            UpdateUi
	update_client  ?update.ReleaseClient // tests inject a fake; none: GitHub
	update_install ?UpdateInstaller      // tests inject a fake; none: the real install
	crash_reports  []crash.PendingCrashReport
	crash_shown    bool
	crash_identity string // this process's staging files: never reported as a crash
	// SSH and SFTP (connect.v)
	sess_view     core.View[sessionui.SessionPanel]
	sftp_view     core.View[sftpui.SftpPanel]
	sftp_expanded bool
	attempts      map[u64]&ssh.Cancel
	warn_box      &WarnBox = &WarnBox{}
	known_hosts   string // '' = ~/.ssh/known_hosts; tests always set it
	local_home    string // first folder of SFTP's Local pane; '' = home
	bound_sftp    u64    // the Sftp bound to the panel, 0 = none
	last_local    bool = true // the kind the dock last followed
}

@[markused]
pub fn (mut o OneTerm) render(mut cx core.ViewCx[OneTerm]) core.Element {
	if o.tabs.len == 0 {
		o.setup(mut cx)
	}
	t := kit.theme(cx)
	if o.vault.is_screen_locked() {
		return o.lock_view(t, mut cx)
	}
	mut area := ui.div().flex_row().grow().min_h(0)
	dock := o.dock_open && !is_restricted()
	if dock && o.sftp_expanded { // the expanded browser covers the terminals (specs/22)
		area.child(o.right_dock(t, true, mut cx))
	} else {
		area.child(o.center_area(t, mut cx))
		if dock {
			area.child(o.right_dock(t, false, mut cx))
		}
	}
	title := kit.titlebar('').controls(o.title_items(t))
	root :=
		ui.div().flex_col().size_full().bg(t.p.canvas).child(title).child(area).child(o.status_bar(t, mut cx))
	if o.ui.show_fps {
		root.child(o.fps_hud(t, mut cx))
	}
	return o.with_actions(root, mut cx).on_mouse_move(cx.listener(fn (mut o OneTerm, _ &core.MouseMoveEvent, mut _ core.ViewCx[OneTerm]) {
		o.last_input = time.now()
	})).on_mouse_down(cx.listener(fn (mut o OneTerm, _ &core.MouseDownEvent, mut _ core.ViewCx[OneTerm]) {
		o.last_input = time.now()
	}))
}

// title_items are the title bar's left side: 12 in, the glyph, the app menu and the elevation
// suffix with no gap between them (one group, so the bar's own gap does not apply).
fn (o &OneTerm) title_items(t &kit.Theme) []core.Element {
	mut row := ui.div().flex_row().items_center().pl(t.sp(12)).child(kit.icon(icon_terminal_logo).size(20).color(brand_accent)).child(kit.menu_button_with('titlebar-menu',
		'OneTerm', OneRef{o}, app_menu_items).ghost().size(.sm).compact())
	suffix := elevation_suffix(elevation)
	if suffix != '' {
		row.child(ui.text(suffix).weight(700).color(t.p.warn))
	}
	return [core.Element(row)]
}

// strip is the horizontal tab bar.
fn (mut o OneTerm) strip(mut cx core.ViewCx[OneTerm]) core.Element {
	mut tabs := []work.Tab{cap: o.tabs.len}
	mut leads := map[string][]core.Element{}
	for tab in o.tabs {
		tabs << work.Tab{
			id:    tab.id
			title: o.title_of(tab, &cx.Cx)
		}
		leads[tab.id] = o.tab_lead(tab, kit.theme(cx))
	}
	mut after := []core.Element{}
	if !is_restricted() {
		glyph := if o.dock_open { icon_panel_right } else { icon_panel_right_open }
		after << kit.icon_button('tabs-dock', glyph).ghost().size(.xs).tooltip('Toggle Right Dock').on_click(cx.listener(fn (mut o OneTerm, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
			o.toggle_dock(mut cx)
		}))
	}
	return work.tab_strip('tabs', tabs, o.active).list_heading('Terminal Tabs').leads(leads).actions([
		o.new_tab_button(.xs),
	]).after(after).tab_width(100, 220).dock('tabs',
		cx.listener(fn (mut _ OneTerm, _ &work.DockEvent, mut _ core.ViewCx[OneTerm]) {})).on_select(cx.listener(fn (mut o OneTerm, id &string, mut cx core.ViewCx[OneTerm]) {
		o.select_tab(*id, mut cx)
	})).on_close(cx.listener(fn (mut o OneTerm, id &string, mut cx core.ViewCx[OneTerm]) {
		o.close_tab(*id, mut cx)
	})).on_move(cx.listener(fn (mut o OneTerm, m &work.TabMove, mut _ core.ViewCx[OneTerm]) {
		o.move_tab(m.id, m.to)
	})).on_menu(cx.listener(fn (mut o OneTerm, p &work.TabPoint, mut cx core.ViewCx[OneTerm]) {
		kit.open_context_menu(mut cx.Cx, p.x, p.y, o.tab_menu(p.id, false))
	})).on_rename(cx.listener(fn (mut o OneTerm, id &string, mut cx core.ViewCx[OneTerm]) {
		o.ask_rename(*id, mut cx)
	}))
}

fn (mut o OneTerm) setup(mut cx core.ViewCx[OneTerm]) {
	if !is_restricted() {
		o.load_docks()
	}
	cx.set_interval(1000, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.tick(mut cx)
		if o.stats_on {
			o.sample_stats(mut cx)
		}
		cx.notify()
	})
	o.mount_panels(mut cx)
	o.new_tab(o.first_shell, '', mut cx)
	o.open_probe_tabs(mut cx)
	o.tick(mut cx)
	o.show_crash_reports(mut cx)
	o.start_auto_check(mut cx)
	notice := elevated_settings_notice(is_restricted(), o.ui.persist_blocked)
	if notice != '' {
		kit.notify(mut cx.Cx, .info, notice, '')
	}
}

// tick updates what changes with time: the clock, the indicators, the idle lock.
fn (mut o OneTerm) tick(mut cx core.ViewCx[OneTerm]) {
	o.clock = time.now().format_ss()
	if cx.app.windows.len > 0 {
		// Counted only while the HUD shows, from its first tick: `-` until a whole second ran.
		f := cx.app.windows[0].stats.frames
		o.fps = if o.ui.show_fps && o.fps_last >= 0 { f - o.fps_last } else { -1 }
		o.fps_last = if o.ui.show_fps { f } else { -1 }
	}
	o.tick_indicators(mut cx)
	o.tick_gpu(mut cx)
	o.tick_channels(mut cx)
	o.tick_lock(mut cx)
	o.tick_logs(mut cx)
	o.drain_warnings(mut cx)
	o.tick_mem(mut cx)
}

fn (o &OneTerm) current() &TermTab {
	for tab in o.tabs {
		if tab.id == o.active {
			return tab
		}
	}
	return o.tabs[0]
}

// active_space is the active Space of a tab.
fn (tab &TermTab) active_space() ?&Space {
	for s in tab.spaces {
		if s.id == tab.active {
			return s
		}
	}
	return none
}

// active_cwd is the active Space's working directory, from OSC 7 ('' until the shell reports).
fn (o &OneTerm) active_cwd(mut cx core.ViewCx[OneTerm]) string {
	s := o.current().active_space() or { return '' }
	return if s.view.id != 0 { cx.peek(s.view.model()).screen.cwd } else { '' }
}

// title_of follows specs/22, Tabs.
fn (o &OneTerm) title_of(tab &TermTab, cx &core.Cx) string {
	if tab.name != '' {
		return tab.name
	}
	for s in tab.spaces {
		if s.view.id != 0 {
			title := cx.peek(s.view.model()).screen.title
			if o.cfg.layout.tab_title == 'osc' && title != '' {
				absolute := title.starts_with('/') || title.starts_with('\\')
					|| (title.len > 2 && title[1] == `:`)
				return if absolute { os.file_name(title) } else { title }
			}
			return s.shell.name
		}
	}
	return 'Terminal'
}
