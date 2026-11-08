// IME composition through IMM32 (loaded at runtime). The default composition
// window is hidden: the app draws the preedit text itself at the caret.
module win

import quadra.platform

type FnImmGetContext = fn (hwnd voidptr) voidptr

type FnImmReleaseContext = fn (hwnd voidptr, himc voidptr) int

type FnImmGetCompositionStringW = fn (himc voidptr, index u32, buf voidptr, len u32) int

type FnImmSetCandidateWindow = fn (himc voidptr, form &CandidateForm) int

type FnImmSetCompositionWindow = fn (himc voidptr, form &CompositionForm) int

type FnImmAssociateContextEx = fn (hwnd voidptr, himc voidptr, flags u32) int

struct CandidateForm {
	index u32
	style u32
	pos   C.POINT
	area  C.RECT
}

struct CompositionForm {
	style u32
	pos   C.POINT
	area  C.RECT
}

const gcs_compstr = u32(0x0008)
const gcs_cursorpos = u32(0x0080)
const gcs_resultstr = u32(0x0800)
const isc_showuicompositionwindow = isize(u32(0x80000000))
const cfs_point = u32(0x0002)
const cfs_exclude = u32(0x0080)

struct Ime {
mut:
	get_context     FnImmGetContext            = unsafe { nil }
	release         FnImmReleaseContext        = unsafe { nil }
	get_string      FnImmGetCompositionStringW = unsafe { nil }
	set_candidate   FnImmSetCandidateWindow    = unsafe { nil }
	set_comp_window FnImmSetCompositionWindow  = unsafe { nil }
	associate       FnImmAssociateContextEx    = unsafe { nil }
	caret           C.RECT // device px, client coords
}

fn load_ime() !Ime {
	imm := load_system_dll('imm32.dll')!
	return Ime{
		get_context:     unsafe { FnImmGetContext(dll_proc(imm, 'ImmGetContext')!) }
		release:         unsafe { FnImmReleaseContext(dll_proc(imm, 'ImmReleaseContext')!) }
		get_string:      unsafe {
			FnImmGetCompositionStringW(dll_proc(imm, 'ImmGetCompositionStringW')!)
		}
		set_candidate:   unsafe { FnImmSetCandidateWindow(dll_proc(imm, 'ImmSetCandidateWindow')!) }
		set_comp_window: unsafe { FnImmSetCompositionWindow(dll_proc(imm, 'ImmSetCompositionWindow')!) }
		associate:       unsafe { FnImmAssociateContextEx(dll_proc(imm, 'ImmAssociateContextEx')!) }
	}
}

fn (ime &Ime) read(himc voidptr, kind u32) []u16 {
	bytes := ime.get_string(himc, kind, unsafe { nil }, 0)
	if bytes <= 0 {
		return []u16{}
	}
	mut buf := []u16{len: bytes / 2}
	ime.get_string(himc, kind, buf.data, u32(bytes))
	return buf
}

// on_composition turns one WM_IME_COMPOSITION into commit and/or preedit events.
fn (mut ime Ime) on_composition(hwnd voidptr, lparam isize) []platform.Event {
	mut out := []platform.Event{}
	himc := ime.get_context(hwnd)
	if himc == unsafe { nil } {
		return out
	}
	if u32(lparam) & gcs_resultstr != 0 {
		units := ime.read(himc, gcs_resultstr)
		if units.len > 0 {
			out << platform.Event(platform.ImeEvent{
				kind: .commit
				text: utf16_to_string(&u16(units.data), units.len)
			})
		}
	}
	if u32(lparam) & gcs_compstr != 0 {
		units := ime.read(himc, gcs_compstr)
		cursor16 := ime.get_string(himc, gcs_cursorpos, unsafe { nil }, 0)
		text := if units.len > 0 { utf16_to_string(&u16(units.data), units.len) } else { '' }
		out << platform.Event(platform.ImeEvent{
			kind:   .preedit
			text:   text
			cursor: if units.len > 0 { utf8_offset_of_utf16(units, cursor16) } else { 0 }
		})
	}
	ime.place(himc)
	ime.release(hwnd, himc)
	return out
}

// place moves the OS candidate window next to the caret rectangle.
fn (ime &Ime) place(himc voidptr) {
	comp := CompositionForm{
		style: cfs_point
		pos:   C.POINT{ime.caret.left, ime.caret.top}
	}
	ime.set_comp_window(himc, &comp)
	cand := CandidateForm{
		style: cfs_exclude
		pos:   C.POINT{ime.caret.left, ime.caret.bottom}
		area:  ime.caret
	}
	ime.set_candidate(himc, &cand)
}

fn (mut ime Ime) set_caret(hwnd voidptr, r C.RECT) {
	ime.caret = r
	himc := ime.get_context(hwnd)
	if himc != unsafe { nil } {
		ime.place(himc)
		ime.release(hwnd, himc)
	}
}

// enable turns IME input on (default context) or off (no context) for a window.
fn (ime &Ime) enable(hwnd voidptr, on bool) {
	ime.associate(hwnd, unsafe { nil }, if on { u32(0x10) } else { u32(0) }) // IACE_DEFAULT
}
