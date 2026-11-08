// Element-level events delivered to listeners. Coordinates are window logical px.
module core

import quadra.platform

pub struct MouseDownEvent {
pub:
	x      f32
	y      f32
	button platform.MouseButton
	clicks int
	mods   platform.Mods
}

pub struct MouseUpEvent {
pub:
	x      f32
	y      f32
	button platform.MouseButton
	mods   platform.Mods
}

pub struct ClickEvent {
pub:
	x      f32
	y      f32
	button platform.MouseButton
	clicks int // 2 for a double click
	mods   platform.Mods
}

pub struct MouseMoveEvent {
pub:
	x       f32
	y       f32
	pressed bool // a button is held (dragging)
	mods    platform.Mods
}

pub struct ScrollEvent {
pub:
	dx      f32 // logical px
	dy      f32
	precise bool
	x       f32
	y       f32
	mods    platform.Mods
}

pub struct KeyDownEvent {
pub:
	key    platform.Key
	mods   platform.Mods
	repeat bool
}

pub struct KeyUpEvent {
pub:
	key  platform.Key
	mods platform.Mods
}

pub struct TextInputEvent {
pub:
	text string
}

pub struct ImeInputEvent {
pub:
	kind   platform.ImeKind
	text   string
	cursor int
}

pub struct FocusChangeEvent {
pub:
	focused bool
}

pub enum HandlerKind {
	mouse_down
	mouse_up
	mouse_move
	click
	scroll
	drop // a drag released over this target (drag.v)
}
