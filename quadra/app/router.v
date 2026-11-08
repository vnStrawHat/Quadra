// Router: a history of typed routes with push, replace, back and forward. Keep it in a
// view or model; the root view renders router.current().
module app

import quadra.core

pub struct Router[R] {
pub mut:
	stack []R
	index int
}

pub fn new_router[R](initial R) Router[R] {
	return Router[R]{
		stack: [initial]
	}
}

pub fn (r &Router[R]) current() R {
	return r.stack[r.index]
}

// push opens a route; the forward history is dropped, as in a browser.
pub fn (mut r Router[R]) push(route R) {
	r.stack = r.stack[..r.index + 1].clone()
	r.stack << route
	r.index++
}

pub fn (mut r Router[R]) replace(route R) {
	r.stack[r.index] = route
}

pub fn (r &Router[R]) can_back() bool {
	return r.index > 0
}

pub fn (r &Router[R]) can_forward() bool {
	return r.index < r.stack.len - 1
}

pub fn (mut r Router[R]) back() bool {
	if r.index == 0 {
		return false
	}
	r.index--
	return true
}

pub fn (mut r Router[R]) forward() bool {
	if r.index >= r.stack.len - 1 {
		return false
	}
	r.index++
	return true
}

// Actions bound by bind_navigation; handle them with on_action in the root view.
pub struct NavigateBack {}

pub struct NavigateForward {}

// bind_navigation binds alt-left and alt-right to NavigateBack and NavigateForward.
pub fn bind_navigation(mut a core.App) ! {
	a.bind('alt-left', NavigateBack{})!
	a.bind('alt-right', NavigateForward{})!
}
