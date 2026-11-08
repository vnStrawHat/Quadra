// Open/save file dialogs (IFileOpenDialog, IFileSaveDialog) and the shell: open a URL or a
// file with its default app, show a file in Explorer.
module win

import quadra.platform

type FnCoInit = fn (reserved voidptr) int

type FnCoCreate = fn (clsid &Guid, outer voidptr, ctx u32, iid &Guid, out &voidptr) int

type FnCoFree = fn (p voidptr)

type FnShellExecute = fn (hwnd voidptr, op &u16, file &u16, params &u16, dir &u16, show int) voidptr

type FnParseName = fn (path &u16, bind voidptr, iid &Guid, out &voidptr) int

type FnCallP = fn (this voidptr, a voidptr) int

type FnCallU = fn (this voidptr, a u32) int

type FnCallUP = fn (this voidptr, a u32, b voidptr) int

type FnCallOut = fn (this voidptr, out &voidptr) int

type FnCallUOut = fn (this voidptr, a u32, out &voidptr) int

type FnCount = fn (this voidptr, n &u32) int

const clsid_file_open = guid('DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7')
const clsid_file_save = guid('C0B4E2F3-BA21-4773-8DBA-335EC946EB8B')
const iid_file_open = guid('d57c7288-d4ad-4768-be02-9d969532d960')
const iid_file_save = guid('84bccd23-5fde-4cdb-aea4-af64b83d78ab')
const iid_shell_item = guid('43826d1e-e718-42ee-bc55-a1e261c37bfe')

// Vtable slots (tools/com_slots.sh IFileOpenDialog, IShellItem, IShellItemArray).
const fd_show = 3
const fd_set_file_types = 4
const fd_set_options = 9
const fd_get_options = 10
const fd_set_folder = 12
const fd_set_file_name = 15
const fd_set_title = 17
const fd_get_result = 20
const fd_get_results = 27
const si_get_display_name = 5
const sia_get_count = 7
const sia_get_item_at = 8

const fos_overwriteprompt = u32(0x2)
const fos_pickfolders = u32(0x20)
const fos_forcefilesystem = u32(0x40)
const fos_allowmultiselect = u32(0x200)
const fos_pathmustexist = u32(0x800)
const fos_filemustexist = u32(0x1000)
const sigdn_filesyspath = u32(0x80058000)

struct FilterSpec {
	name &u16
	spec &u16
}

struct Ole {
	co_create  FnCoCreate     = unsafe { nil }
	co_free    FnCoFree       = unsafe { nil }
	execute    FnShellExecute = unsafe { nil }
	parse_name FnParseName    = unsafe { nil }
	ok         bool
}

// ole loads COM on first use: the dialogs need a single-threaded apartment on the UI thread.
fn (mut p WinPlatform) ole() Ole {
	if p.ole_.ok {
		return p.ole_
	}
	ole32 := load_system_dll('ole32.dll') or { return Ole{} }
	shell := load_system_dll('shell32.dll') or { return Ole{} }
	init := FnCoInit(dll_proc(ole32, 'OleInitialize') or { return Ole{} })
	init(unsafe { nil })
	p.ole_ = Ole{
		co_create:  FnCoCreate(dll_proc(ole32, 'CoCreateInstance') or { return Ole{} })
		co_free:    FnCoFree(dll_proc(ole32, 'CoTaskMemFree') or { return Ole{} })
		execute:    FnShellExecute(dll_proc(shell, 'ShellExecuteW') or { return Ole{} })
		parse_name: FnParseName(dll_proc(shell, 'SHCreateItemFromParsingName') or { return Ole{} })
		ok:         true
	}
	return p.ole_
}

fn (mut p WinPlatform) owner_hwnd(id platform.WindowId) voidptr {
	for w in p.windows {
		if w.id == id {
			return w.hwnd
		}
	}
	return unsafe { nil }
}

// pick_files shows a modal open or save dialog; it returns the chosen paths, or none when
// the user cancels.
pub fn (mut p WinPlatform) pick_files(opts platform.FileDialog, owner platform.WindowId) []string {
	o := p.ole()
	if !o.ok {
		return []
	}
	mut dlg := unsafe { nil }
	clsid, iid := if opts.save {
		clsid_file_save, iid_file_save
	} else {
		clsid_file_open, iid_file_open
	}
	if o.co_create(&clsid, unsafe { nil }, 1, &iid, &dlg) < 0 { // CLSCTX_INPROC_SERVER
		return []
	}
	defer {
		release(dlg)
	}
	mut flags := u32(0)
	com_count(dlg, fd_get_options, &flags)
	flags |= fos_forcefilesystem | fos_pathmustexist
	if opts.save {
		flags |= fos_overwriteprompt
	} else if opts.folders {
		flags |= fos_pickfolders
	} else {
		flags |= fos_filemustexist
	}
	if opts.multiple && !opts.save {
		flags |= fos_allowmultiselect
	}
	com_u(dlg, fd_set_options, flags)
	if opts.title != '' {
		com_p(dlg, fd_set_title, opts.title.to_wide())
	}
	if opts.default_name != '' {
		com_p(dlg, fd_set_file_name, opts.default_name.to_wide())
	}
	if opts.filters.len > 0 && !opts.folders {
		specs := opts.filters.map(FilterSpec{it.name.to_wide(), it.patterns.to_wide()})
		com_up(dlg, fd_set_file_types, u32(specs.len), specs.data)
	}
	if opts.dir != '' {
		mut item := unsafe { nil }
		if o.parse_name(opts.dir.to_wide(), unsafe { nil }, &iid_shell_item, &item) >= 0 {
			com_p(dlg, fd_set_folder, item)
			release(item)
		}
	}
	if com_p(dlg, fd_show, p.owner_hwnd(owner)) < 0 {
		return [] // cancelled
	}
	if opts.multiple && !opts.save {
		mut items := unsafe { nil }
		if com_out(dlg, fd_get_results, &items) < 0 {
			return []
		}
		defer {
			release(items)
		}
		mut n := u32(0)
		com_count(items, sia_get_count, &n)
		mut out := []string{cap: int(n)}
		for i in 0 .. n {
			mut item := unsafe { nil }
			if com_uout(items, sia_get_item_at, i, &item) >= 0 {
				out << item_path(o, item)
				release(item)
			}
		}
		return out.filter(it != '')
	}
	mut item := unsafe { nil }
	if com_out(dlg, fd_get_result, &item) < 0 {
		return []
	}
	path := item_path(o, item)
	release(item)
	return if path == '' { [] } else { [path] }
}

fn item_path(o Ole, item voidptr) string {
	mut name := &u16(unsafe { nil })
	if com_uout(item, si_get_display_name, sigdn_filesyspath, voidptr(&name)) < 0 {
		return ''
	}
	s := unsafe { string_from_wide(name) }
	o.co_free(name)
	return s
}

// open_external opens a URL or a file with its default app.
pub fn (mut p WinPlatform) open_external(target string) bool {
	o := p.ole()
	if !o.ok {
		return false
	}
	r := o.execute(unsafe { nil }, 'open'.to_wide(), target.to_wide(), unsafe { nil },
		unsafe { nil }, 1)
	return u64(r) > 32 // ShellExecute returns a value above 32 on success
}

// reveal opens Explorer on the folder of path, with the file selected.
pub fn (mut p WinPlatform) reveal(path string) bool {
	o := p.ole()
	if !o.ok {
		return false
	}
	r := o.execute(unsafe { nil }, 'open'.to_wide(), 'explorer.exe'.to_wide(),
		'/select,"${path}"'.to_wide(), unsafe { nil }, 1)
	return u64(r) > 32
}

// Calls of COM methods by vtable slot, by signature.
fn com_count(obj voidptr, i int, n &u32) int {
	f := unsafe { FnCount(slot(obj, i)) }
	return f(obj, n)
}

fn com_u(obj voidptr, i int, a u32) int {
	f := unsafe { FnCallU(slot(obj, i)) }
	return f(obj, a)
}

fn com_p(obj voidptr, i int, a voidptr) int {
	f := unsafe { FnCallP(slot(obj, i)) }
	return f(obj, a)
}

fn com_up(obj voidptr, i int, a u32, b voidptr) int {
	f := unsafe { FnCallUP(slot(obj, i)) }
	return f(obj, a, b)
}

fn com_out(obj voidptr, i int, out &voidptr) int {
	f := unsafe { FnCallOut(slot(obj, i)) }
	return f(obj, out)
}

fn com_uout(obj voidptr, i int, a u32, out &voidptr) int {
	f := unsafe { FnCallUOut(slot(obj, i)) }
	return f(obj, a, out)
}

fn C.MessageBoxW(hwnd voidptr, text &u16, caption &u16, flags u32) int

pub fn (mut p WinPlatform) message_box(m platform.MessageBox, owner platform.WindowId) platform.MessageResult {
	icon := match m.kind {
		.info { u32(0x40) } // MB_ICONINFORMATION
		.warning { 0x30 }
		.error { 0x10 }
		.question { 0x20 }
	}

	buttons := match m.buttons {
		.ok { u32(0) }
		.ok_cancel { 1 }
		.yes_no { 4 }
		.yes_no_cancel { 3 }
	}

	r := C.MessageBoxW(p.owner_hwnd(owner), m.text.to_wide(), m.title.to_wide(), icon | buttons)
	return match r {
		1 { platform.MessageResult.ok } // IDOK
		6 { .yes }
		7 { .no }
		else { .cancel }
	}
}
