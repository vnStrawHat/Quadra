// Broadcast input channels (specs/22): members write what is typed in any of them.
module main

import time
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.prim
import quadra.ui

const channels = ['A', 'B', 'C', 'D', 'E']

// BroadcastPty decorates a Space's PTY: input from the user also goes to the other members.
// ponytail: a filter on what looks like input, since the PTY sees replies and mouse reports
// too; a real input tap in kit/term would remove it. Alternate-screen wheel arrows still fan out.
@[heap]
pub struct BroadcastPty {
mut:
	one   &OneTerm  = unsafe { nil }
	space &Space    = unsafe { nil }
	app   &core.App = unsafe { nil }
	inner platform.Pty
}

fn is_reply_or_report(data []u8, replies []u8) bool {
	if data == replies {
		return true
	}
	s := data.bytestr()
	if s.len > 3 && s.starts_with('\x1b[') && s[..s.len - 1].ends_with(':3') {
		return true // a key release is never broadcast
	}
	return s.starts_with('\x1b[<') || s.starts_with('\x1b[M') || s.starts_with('\x1b]52;')
		|| s == '\x1b[I' || s == '\x1b[O'
}

pub fn (mut b BroadcastPty) write(data []u8) ! {
	b.inner.write(data)!
	cx := core.Cx{
		app: b.app
	}
	t := cx.peek[term.Terminal](b.space.view.model())
	if is_reply_or_report(data, t.screen.replies) {
		return
	}
	b.one.last_input = time.now()
	if b.space.channel == '' {
		return
	}
	for mut tab in b.one.tabs {
		for mut m in tab.spaces {
			if m.id != b.space.id && m.channel == b.space.channel && m.pty != unsafe { nil } {
				m.pty.inner.write(data) or {}
			}
		}
	}
}

pub fn (mut b BroadcastPty) resize(cols int, rows int) ! {
	b.inner.resize(cols, rows)!
}

pub fn (mut b BroadcastPty) close() {
	b.inner.close()
}

fn alive(s &Space, cx &core.Cx) bool {
	return s.view.id != 0 && cx.peek(s.view.model()).exited == none
}

fn (o &OneTerm) tab_of(id string) &TermTab {
	for tab in o.tabs {
		if tab.id == id {
			return tab
		}
	}
	return o.current()
}

fn channel_target(arg string) (string, string) {
	return arg.all_before('|'), arg.all_after('|')
}

// join_channel puts the tab's active Space in a channel; arg is 'tabid|X' ('|X': the current tab).
fn (mut o OneTerm) join_channel(arg string, mut cx core.ViewCx[OneTerm]) {
	id, letter := channel_target(arg)
	mut tab := if id == '' { o.current() } else { o.tab_of(id) }
	if mut s := tab.active_space() {
		if alive(s, &cx.Cx) {
			s.channel = letter
		}
	}
}

fn (mut o OneTerm) leave_channel(mut cx core.ViewCx[OneTerm]) {
	if mut s := o.current().active_space() {
		s.channel = ''
	}
}

// close_channel removes every Space from the active Space's channel.
fn (mut o OneTerm) close_channel(mut cx core.ViewCx[OneTerm]) {
	letter := if s := o.current().active_space() { s.channel } else { '' }
	if letter == '' {
		return
	}
	for mut tab in o.tabs {
		for mut s in tab.spaces {
			if s.channel == letter {
				s.channel = ''
			}
		}
	}
}

fn (mut o OneTerm) join_all(arg string, mut cx core.ViewCx[OneTerm]) {
	id, letter := channel_target(arg)
	mut tab := if id == '' { o.current() } else { o.tab_of(id) }
	for mut s in tab.spaces {
		if alive(s, &cx.Cx) {
			s.channel = letter
		}
	}
}

fn (mut o OneTerm) leave_all(mut cx core.ViewCx[OneTerm]) {
	mut tab := o.current()
	for mut s in tab.spaces {
		s.channel = ''
	}
}

// channel_menu is the tab menu's "Input Channel" row, disabled while it has nothing to act on.
fn (o &OneTerm) channel_menu(tab &TermTab) kit.MenuItem {
	s := tab.active_space() or { return kit.submenu('Input Channel', []) }
	mut rows := channels.map(item(if s.channel == it { '* Channel ${it}' } else { 'Channel ${it}' },
		.join_channel, '${tab.id}|${it}'))
	if s.channel != '' {
		rows << kit.separator()
		rows << item('Leave Channel', .leave_channel, '')
		rows << item('Close Channel ${s.channel}', .close_channel, '')
	}
	if tab.spaces.len > 1 {
		rows << kit.separator()
		letter := if s.channel != '' { s.channel } else { 'A' }
		rows << item('Join All Spaces In Tab To ${letter}', .join_all, '${tab.id}|${letter}')
		rows << item('Leave With All Spaces In Tab', .leave_all, '')
	}
	return kit.MenuItem{
		label:    'Input Channel'
		items:    rows
		disabled: s.view.id == 0
	}
}

// tick_channels drops Spaces whose session ended.
fn (mut o OneTerm) tick_channels(mut cx core.ViewCx[OneTerm]) {
	for mut tab in o.tabs {
		for mut s in tab.spaces {
			if s.channel != '' && !alive(s, &cx.Cx) {
				s.channel = ''
			}
		}
	}
}

pub fn channel_color(letter string, t &kit.Theme) prim.Color {
	i := channels.index(letter)
	return t.series[if i < 0 { 0 } else { i } % t.series.len]
}

fn channel_chip(letter string, t &kit.Theme) core.Element {
	return ui.div().size(16).shrink_0().center().bg(channel_color(letter, t)).text_size(12).font_weight(700).text_color(t.colors.background).child(ui.text(letter))
}

// channel_badge is a member Space's chip, in its top-right corner.
fn channel_badge(letter string, t &kit.Theme) core.Element {
	return ui.div().absolute().top(5).right(5).child(channel_chip(letter, t))
}

// tab_lead is what a tab draws before its title: chips in A..E order, recording dot, status.
fn (o &OneTerm) tab_lead(tab &TermTab, t &kit.Theme) []core.Element {
	mut out := []core.Element{}
	for c in channels {
		if tab.spaces.any(it.channel == c) {
			out << channel_chip(c, t)
		}
	}
	if s := tab.active_space() {
		if is_recording(s) {
			out << recording_dot(t.p.error, t)
		}
		if tab.spaces.len == 1 {
			if el := status_icon(program_status(s), t) {
				out << el
			}
		}
	}
	return out
}

// status_icon is the OSC 7501 mark. A split tab shows none (the caller checks).
