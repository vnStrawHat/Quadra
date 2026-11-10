// Hooks: how an app extends a terminal without closures (specs/17, App hooks).
module term

import quadra.core
import quadra.prim

pub type KeyHook = fn (holder prim.Holder, mut t Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[Terminal]) bool

pub type OverlayHook = fn (holder prim.Holder, mut t Terminal, mut cx core.ViewCx[Terminal]) ?core.Element

pub type RowsHook = fn (holder prim.Holder, mut t Terminal, top int)

pub type OutputHook = fn (holder prim.Holder, mut t Terminal, data []u8)

// Hooks are function pointers with a holder: the app's per-terminal state.
pub struct Hooks {
pub:
	holder  prim.Holder // prim.hold(state); hooks read it with prim.held[State]
	key     KeyHook     = unsafe { nil } // sees a key first; true consumes it
	overlay OverlayHook = unsafe { nil } // drawn over the grid
	rows    RowsHook    = unsafe { nil } // fills Terminal.classes for this frame
	output  OutputHook  = unsafe { nil } // after program output is parsed, even off screen
}

// CellTint is the style of a class byte in `Terminal.classes`.
pub struct CellTint {
pub:
	has_fg        bool
	fg            prim.Color
	has_bg        bool
	bg            prim.Color
	bold          bool
	italic        bool
	underline     bool
	override_ansi bool // the class color beats an explicit ANSI color
}

@[inline]
fn (t &Terminal) tint_of(cls []u8, x int) CellTint {
	return if x < cls.len && cls[x] != 0 { t.tints[cls[x]] } else { CellTint{} }
}

// default_fg says whether a cell's text is the terminal's default foreground, the only color
// a class may replace unless it overrides. Inverse and reverse video swap the two defaults.
@[inline]
fn (t &Terminal) default_fg(c Cell) bool {
	inverse := c.flags & f_inverse != 0
	if inverse != t.screen.reverse_video {
		return false
	}
	return if inverse { c.bg == 0 } else { c.fg == 0 }
}

// tinted applies a class over the cell's own colors (specs/17).
fn (t &Terminal) tinted(c Cell, tint CellTint, fg prim.Color) prim.Color {
	if !tint.has_fg || (!tint.override_ansi && !t.default_fg(c)) {
		return fg
	}
	return if c.flags & f_faint != 0 { tint.fg.alpha(tint.fg.a * 0.6) } else { tint.fg }
}
