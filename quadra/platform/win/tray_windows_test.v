module win

fn test_notify_icon_data_matches_the_sdk() {
	assert sizeof(NotifyIconData) == 976 // NOTIFYICONDATAW on x64
}

// The real shell: add, update and remove the icon (no notification: it would show).
fn test_tray_icon_add_update_remove() {
	mut p := new_platform()!
	assert p.set_tray('Quadra test')
	assert p.set_tray('Quadra test: updated')
	assert p.set_tray('')
	assert !p.tray_on
}

fn test_tray_events_decode() {
	e := tray_event(usize(u32(200) << 16 | 100), 0x007B) or { panic('none') }
	assert e.action == .menu && e.x == 100 && e.y == 200
	assert tray_event(0, 0x0200) == none // mouse moves are not reported
}
