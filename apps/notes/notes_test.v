// Harness tests for the main Notes flows (headless: no window station, GPU or real disk
// outside .tmp).
module main

import os
import quadra.core
import quadra.platform
import quadra.testing

struct Run {
mut:
	h     &testing.Harness
	shell core.View[Shell]
	root  string
}

fn start(name string) !Run {
	root := os.join_path(os.temp_dir(), 'quadra-tests', 'notes', name)
	os.rmdir_all(root) or {}
	mut h := testing.new_harness(width: 1100, height: 720)!
	opts, shell := prepare(mut h.app, root, [])!
	v := h.open(shell, width: opts.width, height: opts.height)!
	h.wait_tasks()! // the library loads in a task
	return Run{h, v, root}
}

fn (r &Run) s() &Shell {
	cx := core.Cx{
		app: r.h.app
	}
	return cx.peek(r.shell.model())
}

fn (r &Run) notes() &Notes {
	cx := core.Cx{
		app: r.h.app
	}
	return cx.peek(r.s().notes)
}

fn (r &Run) notes_dir() string {
	return os.join_path(r.root, 'data', 'notes')
}

fn test_first_run_and_autosave() {
	mut r := start('autosave')!
	assert r.h.has_text('0 notes')
	assert os.is_dir(os.join_path(r.notes_dir(), 'Personal'))
	r.h.press('ctrl-n')!
	r.h.type_text('Shopping\nmilk, eggs')
	assert r.h.has_text('Shopping') // the list shows the title at once
	assert r.h.has_text('Unsaved changes')
	r.h.advance(autosave_ms)
	r.h.wait_tasks()!
	assert r.h.has_text('Saved')
	files := os.ls(os.join_path(r.notes_dir(), 'Personal'))!
	assert files.len == 1
	assert os.read_file(os.join_path(r.notes_dir(), 'Personal', files[0]))! == 'Shopping\nmilk, eggs'
}

fn test_reload_search_and_folders() {
	mut r := start('search')!
	for text in ['Họp ở Đà Nẵng\nthứ hai', 'Groceries\nbread'] {
		r.h.click('new-note')!
		r.h.type_text(text)
	}
	r.h.advance(autosave_ms)
	r.h.wait_tasks()!
	r.h.click('search')!
	r.h.type_text('da nang') // diacritics are optional
	r.h.wait_tasks()!
	assert r.h.has_text('Họp ở Đà Nẵng')
	assert !r.h.has_text('Groceries')
	r.h.click('folder-Work')!
	assert r.h.has_text('No notes match')
}

fn test_language_switch_and_settings_persist() {
	mut r := start('settings')!
	r.h.press('ctrl-,')!
	r.h.click('set-language-1')!
	assert r.h.has_text('Cài đặt')
	assert r.h.has_text('Ghi chú mới')
	r.h.advance(1000) // the settings save is debounced
	text := os.read_file(os.join_path(r.root, 'config', 'prefs.json'))!
	assert text.contains('"language":"vi"')
}

fn test_close_asks_to_save_changes() {
	mut r := start('close')!
	r.h.press('ctrl-n')!
	r.h.type_text('Draft')
	r.h.inject(platform.CloseRequested{})
	assert !r.h.window().pw.is_closed()
	assert r.h.has_text('Save changes?')
	r.h.click('confirm-yes')!
	assert r.h.window().pw.is_closed()
	files := os.ls(os.join_path(r.notes_dir(), 'Personal'))!
	assert files.len == 1
	assert os.exists(os.join_path(r.root, 'config', 'windows.json'))
}

fn test_note_window_shares_the_model() {
	mut r := start('window')!
	r.h.press('ctrl-n')!
	r.h.type_text('Shared')
	id := r.s().selected
	mut cx := core.Cx{
		app: r.h.app
	}
	open_note_window(mut cx, r.s().notes, id)!
	r.h.frame()
	assert r.h.app.windows.len == 2
	main_win := r.h.win
	r.h.win = r.h.app.windows[1].id
	r.h.click('window-editor')!
	r.h.press('end')!
	r.h.type_text(' note')
	r.h.win = main_win
	assert r.notes().find(id)?.body == 'Shared note'
	assert r.h.has_text('Shared note') // the main list re-rendered
	// Deleting the note in the main window closes its own window, not the main one.
	r.h.click('note-0')!
	r.h.press('ctrl-shift-backspace')!
	assert r.h.app.windows[1].pw.is_closed()
	assert !r.h.window().pw.is_closed()
}

fn test_move_before_autosave_leaves_one_file() {
	mut r := start('move')!
	r.h.press('ctrl-n')!
	r.h.type_text('First version')
	r.h.advance(autosave_ms)
	r.h.wait_tasks()! // saved in Personal
	r.h.type_text(', edited')
	mut cx := core.Cx{
		app: r.h.app
	}
	move_to(mut cx, r.s().notes, r.s().selected, 'Work') // dirty: its save is still pending
	r.h.wait_tasks()!
	r.h.advance(autosave_ms)
	r.h.wait_tasks()!
	assert os.ls(os.join_path(r.notes_dir(), 'Personal'))!.len == 0
	work := os.ls(os.join_path(r.notes_dir(), 'Work'))!
	assert work.len == 1
	assert os.read_file(os.join_path(r.notes_dir(), 'Work', work[0]))! == 'First version, edited'
}

fn test_edit_shortcuts_need_a_focused_input() {
	mut r := start('nofocus')!
	r.h.press('ctrl-n')!
	r.h.type_text('Some text')
	mut cx := core.Cx{
		app: r.h.app
		win: r.h.window()
	}
	cx.blur()
	r.h.press('ctrl-a')! // nothing focused: no input may take the edit action
	assert !cx.peek(r.s().editor.model()).buf.has_selection()
}

fn test_table_view_and_file_drop() {
	mut r := start('table')!
	src := os.join_path(r.root, 'import')
	os.mkdir_all(src)!
	os.write_file(os.join_path(src, 'a.md'), 'Imported one')!
	os.write_file(os.join_path(src, 'b.txt'), 'Imported two')!
	r.h.inject(platform.FileDropEvent{
		paths: [os.join_path(src, 'a.md'), os.join_path(src, 'b.txt'),
			os.join_path(src, 'c.png')]
	})
	r.h.wait_tasks()!
	assert r.notes().lib.notes.len == 2
	r.h.press('ctrl-2')!
	assert r.h.has_text('Imported one')
	assert r.h.has_text('Folder')
	r.h.press('alt-left')! // back to the notes page
	assert r.h.has_text('2 notes')
}

fn test_custom_titlebar() {
	mut r := start('titlebar')!
	bar := r.h.find('titlebar') or { panic('no titlebar') }
	max := r.h.find('win-max') or { panic('no maximize button') }
	id := r.h.win
	assert r.h.app.hit_test(id, bar.x + 200, bar.y + 10) == .caption // drags the window
	assert r.h.app.hit_test(id, max.x + 5, max.y + 5) == .maximize // snap layouts
	tb := r.h.find('new-note') or { panic('') }
	assert r.h.app.hit_test(id, tb.x + 5, tb.y + 5) == .client
	r.h.click('titlebar-menu')! // the hamburger menu runs the bound actions
	r.h.click('menu-6')! // Settings
	assert r.s().router.current() == .settings
	r.h.click('titlebar-menu')!
	r.h.click('menu-4')! // Notes
	assert r.s().router.current() == .notes
	r.h.click('win-max')!
	assert r.h.window().pw.is_maximized()
	// The close button goes through the close flow like the OS button.
	r.h.press('ctrl-n')!
	r.h.type_text('unsaved')
	r.h.click('win-close')!
	assert !r.h.window().pw.is_closed()
	assert r.h.has_text('Save changes?')
}

fn test_pixels() {
	root := os.join_path(os.temp_dir(), 'quadra-tests', 'notes', 'pixels')
	os.rmdir_all(root) or {}
	out := os.join_path(os.dir(@FILE), '..', '..', '.tmp', 'out')
	os.mkdir_all(out)!
	mut h := testing.new_pixel_harness()!
	opts, shell := prepare(mut h.app, root, [])!
	h.open(shell, width: opts.width, height: opts.height)!
	h.wait_tasks()!
	for text in ['Kế hoạch tuần\nHoàn thiện M5, viết test cho Notes',
		'Shopping\nmilk, eggs, bread', 'Ý tưởng\nỨng dụng ghi chú viết bằng Quadra'] {
		h.click('new-note')!
		h.type_text(text)
	}
	h.advance(autosave_ms)
	h.wait_tasks()!
	h.save_png(os.join_path(out, 'notes_main.png'))!
	h.press('ctrl-2')!
	h.save_png(os.join_path(out, 'notes_table.png'))!
	h.press('ctrl-,')!
	h.click('set-language-1')!
	h.click('set-theme-2')!
	h.save_png(os.join_path(out, 'notes_settings_vi_dark.png'))!
	h.press('ctrl-1')!
	h.press('ctrl-p')!
	h.save_png(os.join_path(out, 'notes_palette.png'))!
}
