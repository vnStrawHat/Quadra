// The closure leak detector (specs/14, avoiding closures): outside -prod builds, every
// framework dispatch that runs app code outside a closure lifetime frame (listeners, timer
// callbacks, task completions, observers, subscribers, inbox chunks, main-thread calls)
// counts the closures alive before and after. A closure made there and still alive when the
// dispatch returns is never freed, with everything it captured. Closures made outside any
// dispatch (main, app and window set-up) are made once and are never reported. Set
// QUADRA_TRACE_CLOSURES=1 to print a backtrace wherever a dispatch makes a closure.
// -prod builds compile all of this away.
module core

import os
import quadra.core.life

struct LeakProbe {
	before int
mut:
	nested int // growth already reported by dispatches nested in this one
}

struct LeakCheck {
mut:
	stack    []LeakProbe
	reported map[string]bool
	leaks    []string
	trace    int // QUADRA_TRACE_CLOSURES: 0 not read yet, 1 off, 2 print where dispatches make closures
}

fn (app &App) closures_alive() int {
	return life.live_closures()
}

@[inline]
fn (mut app App) probe_begin() {
	$if !prod {
		app.probe_open()
	}
}

// probe_end closes the innermost probe; kind and owner name the dispatch in the warning.
@[inline]
fn (mut app App) probe_end(kind string, owner u32) {
	$if !prod {
		app.probe_close(kind, owner)
	}
}

fn (mut app App) probe_open() {
	mut lc := &app.leakcheck
	if lc.trace == 0 {
		lc.trace = if os.getenv('QUADRA_TRACE_CLOSURES') != '' { 2 } else { 1 }
	}
	if lc.trace == 2 {
		life.trace(true)
	}
	lc.stack << LeakProbe{
		before: app.closures_alive()
	}
}

fn (mut app App) probe_close(kind string, owner u32) {
	mut lc := &app.leakcheck
	if lc.stack.len == 0 {
		return
	}
	p := lc.stack.pop()
	if lc.trace == 2 && lc.stack.len == 0 {
		life.trace(false)
	}
	total := app.closures_alive() - p.before
	if lc.stack.len > 0 && total > 0 {
		lc.stack[lc.stack.len - 1].nested += total
	}
	grown := total - p.nested
	if grown <= 0 {
		return
	}
	mut name := 'the app'
	if owner != 0 && int(owner) < app.store.slots.len {
		name = app.store.slots[owner].type_name
	}
	key := '${kind} of ${name}'
	if lc.reported[key] {
		return
	}
	lc.reported[key] = true
	lc.leaks << key
	eprintln('quadra: a ${key} made ${grown} closure(s) that will never be freed, with what they capture: pass the data with a `_with` variant instead (specs/14, avoiding closures)')
}

// closure_leaks lists the dispatches that leaked closures so far ("timer of Notes"), each
// once. Always empty in -prod builds. Leak tests assert it stays empty.
pub fn (app &App) closure_leaks() []string {
	return app.leakcheck.leaks
}
