// Helpers shared by kit components.
module kit

import quadra.core
import quadra.prim

pub struct FocusSlot {
mut:
	id u64
}

// stable_focus gives a component a focus id that survives re-renders, kept in element
// state under its position (builders are recreated every render).
pub fn stable_focus(mut cx core.PrepaintCx, key string) u64 {
	cx.enter('focus:${key}')
	mut slot := cx.state(FocusSlot{})
	if slot.id == 0 {
		mut c := core.Cx{
			app: cx.app
			win: cx.win
		}
		slot.id = c.focus_handle().id
	}
	id := slot.id
	cx.leave()
	return id
}

// fire invokes a change listener with a value (discrete: the listening view re-renders).
// Values travel by pointer, so one non-generic helper serves every payload type.
pub fn fire[T](l ?core.AnyListener, value T, mut cx core.Cx) {
	if listener := l {
		v := value
		cx.app.invoke(listener, &v, cx.win, true)
	}
}

// Fire is a change listener and the value to fire it with: the data of the closure-free
// handlers below (specs/14, avoiding closures).
struct Fire[V] {
	change ?core.AnyListener
	value  V
}

fn fire_click_cb[V](f Fire[V], _ &core.ClickEvent, mut cx core.Cx) {
	fire(f.change, f.value, mut cx)
}

fn fire_keys_cb[V](f Fire[V], e &core.KeyDownEvent, mut cx core.Cx) {
	if activates(e) {
		fire(f.change, f.value, mut cx)
		cx.stop_propagation()
	}
}

// fire_click is a click handler that fires change with value.
pub fn fire_click[V](change ?core.AnyListener, value V) core.Listener[core.ClickEvent] {
	return core.handler_with(Fire[V]{change, value}, fire_click_cb[V])
}

// fire_keys is a key handler that fires change with value on enter or space.
pub fn fire_keys[V](change ?core.AnyListener, value V) core.Listener[core.KeyDownEvent] {
	return core.handler_with(Fire[V]{change, value}, fire_keys_cb[V])
}

fn dim(c prim.Color, disabled bool) prim.Color {
	return if disabled { c.alpha(c.a * 0.5) } else { c }
}

fn activates(e &core.KeyDownEvent) bool {
	return e.key == .enter || e.key == .space
}

// Press is what a button's click and key handlers need (closure-free handlers, specs/14).
struct Press {
	click   ?core.AnyListener
	toggled ?core.AnyListener
	pressed bool
}

fn (p Press) press(e &core.ClickEvent, mut cx core.Cx) {
	if l := p.click {
		cx.app.invoke(l, e, cx.win, true)
	}
	fire(p.toggled, !p.pressed, mut cx)
}

// Button fills (button.v).
struct ButtonColors {
	bg     prim.Color
	fg     prim.Color
	hover  prim.Color
	active prim.Color
	border prim.Color
}

// tinted fills a button with a tint; hover and press pull the tint toward its color.
fn tinted(tint prim.Color, color prim.Color, fg prim.Color) ButtonColors {
	step := tint.lerp(color, 0.08)
	return ButtonColors{tint, fg, step, step, prim.transparent}
}
