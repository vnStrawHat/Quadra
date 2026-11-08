// OS-independent window events delivered by every platform implementation.
module platform

pub type WindowId = u32

@[flag]
pub enum Mods {
	shift
	ctrl
	alt
	super
}

pub enum MouseButton {
	none
	left
	right
	middle
	back
	forward
}

pub enum MouseAction {
	move
	down
	up
	leave
}

// Coordinates are logical px (device px divided by the window scale).
pub struct MouseEvent {
pub:
	action MouseAction
	button MouseButton
	x      f32
	y      f32
	mods   Mods
}

pub struct WheelEvent {
pub:
	dx      f32 // in lines when precise is false, in logical px otherwise
	dy      f32
	precise bool
	x       f32
	y       f32
	mods    Mods
}

pub struct KeyEvent {
pub:
	key    Key
	code   u32 // raw OS key code, for diagnostics only
	mods   Mods
	down   bool
	repeat bool
}

// Committed text, UTF-8. Surrogate pairs are already joined.
pub struct TextEvent {
pub:
	text string
}

pub enum ImeKind {
	start
	preedit
	commit
	end
}

pub struct ImeEvent {
pub:
	kind   ImeKind
	text   string
	cursor int // byte offset of the IME cursor inside text, for preedit
}

pub struct ResizeEvent {
pub:
	width  f32 // logical px
	height f32
}

pub struct ScaleEvent {
pub:
	scale f32
}

pub struct FocusEvent {
pub:
	focused bool
}

pub struct CloseRequested {}

// WindowClosed is sent once the OS window is gone.
pub struct WindowClosed {}

// AppearanceChanged is sent (to every window) when dark mode, accent or contrast change.
pub struct AppearanceChanged {}

// FileDropEvent is sent when files are dropped on the window (logical px position).
pub struct FileDropEvent {
pub:
	paths []string
	x     f32
	y     f32
}

// InstanceArgs is sent to the first instance of a single-instance app when another launch
// forwarded its command line (to the app's first window).
pub struct InstanceArgs {
pub:
	args []string
}

pub type Event = A11yEvent
	| TrayEvent
	| CloseRequested
	| FileDropEvent
	| InstanceArgs
	| FocusEvent
	| ImeEvent
	| KeyEvent
	| MouseEvent
	| ResizeEvent
	| ScaleEvent
	| TextEvent
	| WheelEvent
	| WindowClosed
	| AppearanceChanged

// EventSink is implemented by the app. Platforms call it instead of closures,
// so no state has to be captured by value.
pub interface EventSink {
mut:
	on_event(win WindowId, e Event)
	on_frame(win WindowId) // the window may produce a frame now
	on_timer(id TimerId)
	// hit_test reports the titlebar region at a point (logical px) of a window with a
	// custom titlebar. Platforms call it while the OS asks where the mouse is.
	hit_test(win WindowId, x f32, y f32) HitRegion
}
