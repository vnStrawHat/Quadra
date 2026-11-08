// UI Automation end to end (platform/win/uia_*.v): a real window, and a UIA client on
// another thread (a client on the window's own thread would block it) that reads the
// tree, invokes a button and toggles a checkbox.
module main

import time
import quadra.core
import quadra.native
import quadra.platform
import quadra.platform.win
import quadra.ui

struct Panel {
mut:
	saves int
	on    bool
}

@[markused]
pub fn (mut p Panel) render(mut cx core.ViewCx[Panel]) core.Element {
	save := ui.div().w(80).h(30).role(.button).label('Save').test_id('save').on_click(cx.listener(fn (mut p Panel, _ &core.ClickEvent, mut _ core.ViewCx[Panel]) {
		p.saves++
	}))
	check := ui.div().w(80).h(30).role(.checkbox).label('Wrap').checked(p.on).on_click(cx.listener(fn (mut p Panel, _ &core.ClickEvent, mut _ core.ViewCx[Panel]) {
		p.on = !p.on
	}))
	return ui.div().size_full().p(20).child(ui.div().flex_col().gap(8).role(.group).label('Editor').child(save).child(check))
}

type FnCoInitEx = fn (r voidptr, flags u32) int

type FnCoCreate = fn (clsid &win.Guid, outer voidptr, ctx u32, iid &win.Guid, out &voidptr) int

type FnOut = fn (this voidptr, out &voidptr) int

type FnArgOut = fn (this voidptr, a voidptr, out &voidptr) int

type FnFindAll = fn (this voidptr, scope int, cond voidptr, out &voidptr) int

type FnIntOut = fn (this voidptr, out &int) int

type FnIdxOut = fn (this voidptr, i int, out &voidptr) int

type FnCall = fn (this voidptr) int

@[heap]
struct Probe {
mut:
	names  []string
	types  []int
	ids    []string
	toggle int = -1 // Wrap's toggle state after the toggle
	err    string
}

fn call_out(obj voidptr, slot int) voidptr {
	mut out := unsafe { nil }
	f := unsafe { FnOut(win.slot(obj, slot)) }
	f(obj, &out)
	return out
}

fn call_int(obj voidptr, slot int) int {
	mut out := -1
	f := unsafe { FnIntOut(win.slot(obj, slot)) }
	f(obj, &out)
	return out
}

fn call_idx(obj voidptr, slot int, i int) voidptr {
	mut out := unsafe { nil }
	f := unsafe { FnIdxOut(win.slot(obj, slot)) }
	f(obj, i, &out)
	return out
}

fn call0(obj voidptr, slot int) {
	f := unsafe { FnCall(win.slot(obj, slot)) }
	f(obj)
}

fn bstr(p voidptr) string {
	return if p == unsafe { nil } { '' } else { unsafe { string_from_wide(&u16(p)) } }
}

fn client(hwnd voidptr, mut p Probe) {
	ole := win.load_system_dll('ole32.dll') or { panic(err) }
	init := FnCoInitEx(win.dll_proc(ole, 'CoInitializeEx') or { panic(err) })
	create := FnCoCreate(win.dll_proc(ole, 'CoCreateInstance') or { panic(err) })
	init(unsafe { nil }, 0) // multithreaded apartment
	mut auto := unsafe { nil }
	clsid := win.guid('ff48dba4-60ef-4201-aa87-54103eef594e')
	iid := win.guid('30cbe57d-d9d0-452a-ab13-7ac5ac4825ee')
	if create(&clsid, unsafe { nil }, 1, &iid, &auto) < 0 {
		p.err = 'no CUIAutomation'
		return
	}
	mut root := unsafe { nil }
	from_handle := unsafe { FnArgOut(win.slot(auto, 6)) }
	from_handle(auto, hwnd, &root)
	cond := call_out(auto, 21) // CreateTrueCondition
	find_all := unsafe { FnFindAll(win.slot(root, 6)) }
	mut els := []voidptr{}
	for _ in 0 .. 100 { // the tree arrives with the frame after UIA's first ask
		mut arr := unsafe { nil }
		find_all(root, 4, cond, &arr) // TreeScope_Descendants
		n := call_int(arr, 3)
		els.clear()
		for i in 0 .. n {
			els << call_idx(arr, 4, i)
		}
		names := els.map(bstr(call_out(it, 23)))
		if 'Wrap' in names {
			p.names = names
			break
		}
		time.sleep(50 * time.millisecond)
	}
	for k, e in els {
		p.types << call_int(e, 21)
		p.ids << bstr(call_out(e, 29))
		pattern := match p.names[k] {
			'Save' { 10000 }
			'Wrap' { 10015 }
			else { 0 }
		}

		if pattern == 0 {
			continue
		}
		pat := call_idx(e, 16, pattern) // GetCurrentPattern
		if pat == unsafe { nil } {
			p.err = 'no pattern ${pattern} on ${p.names[k]}'
			return
		}
		call0(pat, 3) // Invoke / Toggle
		if pattern == 10015 {
			for _ in 0 .. 100 {
				st := call_int(pat, 4)
				if st == 1 {
					p.toggle = st
					break
				}
				time.sleep(20 * time.millisecond)
			}
		}
	}
}

fn test_screen_readers_see_and_operate_the_tree() {
	mut app := core.new_app(native.backends(force_warp: true)!, core.AppOptions{})!
	v := app.open_window(platform.WindowOptions{ title: 'UIA', width: 300, height: 200 }, Panel{})!
	mut p := &Probe{}
	app.present_hooks << fn [mut p] (mut a core.App) {
		hwnd := a.windows[0].pw.native_handle()
		mut app_ := a
		spawn fn [hwnd, mut p, mut app_] () {
			client(hwnd, mut p)
			app_.plat.post(fn [mut app_] () {
				app_.quit()
			})
		}()
	}
	app.run()
	pw := app.windows[0].pw
	if pw is win.WinWindow {
		assert pw.uia_foreign_calls() == 0 // COM brought every call to the UI thread
	} else {
		assert false, 'not a Win32 window'
	}
	assert p.err == ''
	for want in ['Editor', 'Save', 'Wrap'] {
		assert want in p.names
	}
	save := p.names.index('Save')
	assert p.types[save] == 50000 // Button
	assert p.ids[save] == 'save'
	assert p.types[p.names.index('Wrap')] == 50002 // CheckBox
	assert p.toggle == 1
	mut cx := core.Cx{
		app: app
	}
	assert cx.peek(v.model()).saves == 1
	assert cx.peek(v.model()).on
}
