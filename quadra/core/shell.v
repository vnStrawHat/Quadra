// The desktop shell from app code: file dialogs, opening URLs and files, showing files in
// the file manager (Platform; headless records them for tests).
module core

import quadra.platform

// pick_files shows a modal open or save dialog over the current window and returns the
// chosen paths (empty: cancelled).
pub fn (mut cx Cx) pick_files(opts platform.FileDialog) []string {
	owner := if cx.win != unsafe { nil } { cx.win.id } else { platform.WindowId(0) }
	return cx.app.plat.pick_files(opts, owner)
}

// open_external opens a URL or a file with its default app.
pub fn (mut cx Cx) open_external(target string) bool {
	return cx.app.plat.open_external(target)
}

// reveal shows a file, selected, in the file manager.
pub fn (mut cx Cx) reveal(path string) bool {
	return cx.app.plat.reveal(path)
}

// message_box shows a native modal message over the current window and returns the button
// pressed (cancel when closed).
pub fn (mut cx Cx) message_box(m platform.MessageBox) platform.MessageResult {
	owner := if cx.win != unsafe { nil } { cx.win.id } else { platform.WindowId(0) }
	return cx.app.plat.message_box(m, owner)
}

// set_tray shows the app's icon in the notification area with a tooltip ('' removes it);
// on_tray gets its clicks.
pub fn (mut cx Cx) set_tray(tooltip string) bool {
	return cx.app.plat.set_tray(tooltip)
}

pub fn (mut cx Cx) on_tray(l Listener[platform.TrayEvent]) {
	cx.app.tray_handlers[l.any.view] = l.any
}

// show_notification shows a system notification (from the tray icon, which it adds when
// needed).
pub fn (mut cx Cx) show_notification(title string, body string) bool {
	return cx.app.plat.notify(title, body)
}
