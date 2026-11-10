// Contexts. Cx is the base every phase shares; ViewCx[T] runs on behalf of model T
// (render, event handlers, updates) and knows its type, so typed APIs infer T.
module core

import quadra.prim
import quadra.platform

pub struct Cx {
pub mut:
	app    &App    = unsafe { nil }
	win    &Window = unsafe { nil } // nil outside window work
	entity u32 // the model this context acts for (0 = app)
}

pub struct ViewCx[T] {
	Cx
}

// handle returns the handle of the view or model this context belongs to, for callbacks
// that must reach it later (menu items, palette commands).
pub fn (cx &ViewCx[T]) handle() View[T] {
	return View[T]{cx.entity, cx.app.store.slots[cx.entity].gen}
}

pub fn (cx &Cx) read[T](m Model[T]) &T {
	mut app := cx.app
	p := app.store.ptr(m.id, m.gen, 'model')
	app.track(m.id)
	return prim.held[T](p)
}

// try_read returns none for released models instead of panicking.
pub fn (cx &Cx) try_read[T](m Model[T]) ?&T {
	if !cx.app.store.live(m.id, m.gen) {
		return none
	}
	return cx.read(m)
}

pub fn (cx &Cx) peek[T](m Model[T]) &T {
	return prim.held[T](cx.app.store.ptr(m.id, m.gen, 'model'))
}

pub fn (mut cx Cx) new_model[T](value T) Model[T] {
	cx.app.escaped()
	id, gen := cx.app.store.alloc(heap_copy(value), cx.entity, typeof[T]().name)
	return Model[T]{id, gen}
}

pub fn (mut cx Cx) new_view[T](value T) View[T] {
	cx.app.escaped()
	id, gen := cx.app.store.alloc(heap_copy(value), cx.entity, typeof[T]().name)
	cx.app.store.slots[id].render = render_view[T]
	if id == 0 {
		// Never runs: a direct call keeps V's unused-code pass from dropping T.render,
		// which is otherwise reached only through the function pointer above.
		render_view[T](mut cx.app, cx.win, id)
	}
	if cx.win != unsafe { nil } {
		cx.app.store.slots[id].window = cx.win.id
	}
	return View[T]{id, gen}
}

// update mutates a model. Updating means changed: every dependent is invalidated.
pub fn (mut cx Cx) update[T](m Model[T], f fn (mut v T, mut cx ViewCx[T])) {
	cx.run_update(m, f, true)
}

// update_with is update for a callback that needs data: f takes it as an argument, so the
// caller makes no closure (V frees none; a captured secret would stay in memory for good).
pub fn (mut cx Cx) update_with[T, D](m Model[T], data D, f fn (mut v T, d D, mut cx ViewCx[T])) {
	mut app := cx.app
	mut v := prim.held[T](app.store.ptr(m.id, m.gen, 'model'))
	mut vcx := ViewCx[T]{
		Cx: Cx{
			app:    app
			win:    cx.win
			entity: m.id
		}
	}
	app.depth++
	f(mut v, data, mut vcx)
	app.changed(m.id)
	app.depth--
	app.flush()
}

// update_if is update_with for a callback that reports whether it changed anything: false
// skips the notification, so writing a value the model already holds renders nothing. The
// framework cannot see field writes, hence the opt-in (core.assign makes the report easy).
pub fn (mut cx Cx) update_if[T, D](m Model[T], data D, f fn (mut v T, d D, mut cx ViewCx[T]) bool) {
	mut app := cx.app
	mut v := prim.held[T](app.store.ptr(m.id, m.gen, 'model'))
	mut vcx := ViewCx[T]{
		Cx: Cx{
			app:    app
			win:    cx.win
			entity: m.id
		}
	}
	app.depth++
	if f(mut v, data, mut vcx) {
		app.changed(m.id)
	}
	app.depth--
	app.flush()
}

// assign sets dst to value and reports whether that changed it (update_if callbacks).
pub fn assign[V](mut dst V, value V) bool {
	if dst == value {
		return false
	}
	dst = value
	return true
}

// update_silent mutates without notifying (bookkeeping nothing renders).
pub fn (mut cx Cx) update_silent[T](m Model[T], f fn (mut v T, mut cx ViewCx[T])) {
	cx.run_update(m, f, false)
}

fn (mut cx Cx) run_update[T](m Model[T], f fn (mut v T, mut cx ViewCx[T]), notify bool) {
	mut app := cx.app
	mut v := prim.held[T](app.store.ptr(m.id, m.gen, 'model'))
	mut vcx := ViewCx[T]{
		Cx: Cx{
			app:    app
			win:    cx.win
			entity: m.id
		}
	}
	app.depth++
	f(mut v, mut vcx)
	if notify {
		app.changed(m.id)
	}
	app.depth--
	app.flush()
}

pub fn (mut cx Cx) update_view[T](v View[T], f fn (mut v T, mut cx ViewCx[T])) {
	cx.update(v.model(), f)
}

pub fn (cx &Cx) read_view[T](v View[T]) &T {
	return cx.read(v.model())
}

pub fn (mut cx Cx) release[T](m Model[T]) {
	cx.app.release_id(m.id)
}

// notify marks the current model as changed (its readers and, for views, itself re-render).
pub fn (mut cx Cx) notify() {
	cx.app.changed(cx.entity)
	cx.app.flush()
}

// touch marks a model as changed after it was mutated through peek. Live sources use it
// instead of update, whose closure argument would be a new closure on every tick.
pub fn (mut cx Cx) touch[T](m Model[T]) {
	cx.app.changed(m.id)
	cx.app.flush()
}

// set_global stores an app-wide value keyed by its type; reading it is tracked.
pub fn (mut cx Cx) set_global[T](value T) {
	cx.app.escaped()
	name := typeof[T]().name
	if id := cx.app.globals[name] {
		unsafe {
			*(prim.held[T](cx.app.store.holder(id))) = value
		}
		cx.app.changed(id)
		cx.app.flush()
		return
	}
	mut root := Cx{
		app: cx.app
	}
	m := root.new_model(value)
	cx.app.globals[name] = m.id
}

pub fn (cx &Cx) global[T]() ?&T {
	id := cx.app.globals[typeof[T]().name] or { return none }
	return cx.read(Model[T]{id, cx.app.store.slots[id].gen})
}

// peek_global reads a global without recording a dependency.
pub fn (cx &Cx) peek_global[T]() ?&T {
	id := cx.app.globals[typeof[T]().name] or { return none }
	return prim.held[T](cx.app.store.holder(id))
}

// update_global mutates a global in place (readers re-render).
pub fn (mut cx Cx) update_global[T](f fn (mut v T, mut cx ViewCx[T])) {
	cx.app.escaped()
	id := cx.app.globals[typeof[T]().name] or { return }
	cx.update(Model[T]{id, cx.app.store.slots[id].gen}, f)
}

// touch_global marks a global changed after it was mutated through peek_global: what
// update_global does, without a closure (specs/14). What is stored may hold closures of a
// render, so renders are pinned as for update_global.
pub fn (mut cx Cx) touch_global[T]() {
	cx.app.escaped()
	id := cx.app.globals[typeof[T]().name] or { return }
	cx.app.changed(id)
	cx.app.flush()
}

pub fn (cx &Cx) now_ms() f64 {
	app := cx.app
	return app.plat.now_ms()
}

pub fn (mut cx Cx) clipboard_write(s string) {
	cx.app.plat.clipboard_write(s)
}

pub fn (mut cx Cx) clipboard_read() ?string {
	app := cx.app
	return app.plat.clipboard_read()
}

pub fn (mut cx Cx) stop_propagation() {
	if cx.win != unsafe { nil } {
		cx.win.stopped = true
	}
}

// capture_mouse routes moves and the release to the element handling this mouse-down,
// wherever the pointer goes (sliders, splitters, drag handles).
pub fn (mut cx Cx) capture_mouse() {
	if cx.win != unsafe { nil } && cx.win.dispatching >= 0
		&& cx.win.dispatching < cx.win.hitboxes.len {
		cx.win.capture = cx.win.hitboxes[cx.win.dispatching].id
	}
}

// target_bounds is the bounds of the element whose mouse handler is running.
pub fn (cx &Cx) target_bounds() prim.Rect {
	return if cx.win != unsafe { nil } { cx.win.target_bounds } else { prim.Rect{} }
}

pub fn (mut cx Cx) set_cursor(c platform.Cursor) {
	if cx.win != unsafe { nil } {
		cx.win.pw.set_cursor(c)
	}
}

// view_element places a type-erased view where no context is at hand (overlay builders).
pub fn view_element(v AnyView) Element {
	return &ViewElement{
		view: v
	}
}

// child_any places a type-erased view (window roots, kit hosts).
pub fn (cx &Cx) child_any(v AnyView) Element {
	return &ViewElement{
		view: v
	}
}

// child_view places a view inside another view's element tree.
pub fn (cx &Cx) child_view[T](v View[T]) Element {
	return &ViewElement{
		view: v.any()
	}
}

fn render_view[T](mut app App, win &Window, id u32) Element {
	mut v := prim.held[T](app.store.holder(id))
	mut vcx := ViewCx[T]{
		Cx: Cx{
			app:    app
			win:    win
			entity: id
		}
	}
	app.begin_tracking(id)
	el := v.render(mut vcx)
	app.end_tracking()
	return el
}
