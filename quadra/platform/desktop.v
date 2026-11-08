// Desktop integration types: native message boxes (Platform.message_box).
module platform

pub enum MessageKind {
	info
	warning
	error
	question
}

pub enum MessageButtons {
	ok
	ok_cancel
	yes_no
	yes_no_cancel
}

pub enum MessageResult {
	cancel // also: closed without choosing
	ok
	yes
	no
}

// MessageBox is a native, modal message with a fixed set of buttons. For in-app questions
// the kit's confirm dialog fits better; this one also works before a window exists and
// looks like the system's own.
pub struct MessageBox {
pub:
	title   string
	text    string
	kind    MessageKind
	buttons MessageButtons
}

pub enum TrayAction {
	click
	double_click
	menu // right click or the menu key: show a context menu at x, y
}

// TrayEvent reports a click on the tray icon (Platform.set_tray), at screen coordinates in
// physical pixels.
pub struct TrayEvent {
pub:
	action TrayAction
	x      int
	y      int
}
