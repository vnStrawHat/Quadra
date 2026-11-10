// Window: the frame pipeline with its invalidation levels and platform events.
//   render  : a view changed        -> render dirty views, layout, prepaint, paint
//   prepaint: scroll, overlay moves -> prepaint and paint with the existing layout
//   paint   : hover, press, focus   -> paint only
module core

import time
import quadra.prim
import quadra.gpu
import quadra.layout
import quadra.platform

pub struct WindowStats {
pub mut:
	frames        int
	builds        int         // render + layout + prepaint + paint
	prepaints     int         // prepaint + paint (scrolling)
	paints        int         // paint only (focus, timers)
	patches       int         // hover/press restyles patched into the last scene
	cache_hits    int         // views reused without rendering
	renders       map[u32]int // per view id
	last_build_us i64
	phase_us      [4]i64 // last build: render, layout, prepaint, paint
	phase_bytes   [4]u64 // bytes allocated by each phase of the last build
	first_present u64    // time.sys_mono_now() when the first frame was presented
	last_frame_us i64
	present_us    i64 // Present of the last frame (blocks while the queue is full)
	draw_calls    int
	instances     int
	layout_nodes  int
}

@[heap]
pub struct Window {
pub:
	id platform.WindowId
pub mut:
	app             &App = unsafe { nil }
	pw              platform.PlatformWindow
	surface         gpu.Surface
	root            AnyView
	tree            layout.Tree
	scene           gpu.Scene
	clear_color     prim.Color = prim.white
	focus_color     prim.Color = prim.rgba(0x3B82F6CC) // default focus ring; themes set it
	selection_color prim.Color = prim.rgba(0x3B82F655) // selected text; themes set it
	scroll_color    prim.Color = prim.rgba(0x8088904C) // scroll thumbs; themes set it
	thumb_style     ThumbStyle // the rest of the thumbs' look (scrollbar.v); themes set it
	dirty           bool = true
	stopped         bool
	stats           WindowStats
	hitboxes        []Hitbox
	handlers        []HandlerEntry
	key_nodes       []KeyNode
	key_handlers    []KeyHandler
	focused         u64
	focus_visible   bool
	test_ids        map[string]prim.Rect
	texts           []TextEntry
	semantics       []SemanticNode
	a11y_sent       bool // the platform has had a tree since a11y went on (a11y.v)
	mouse           prim.Point
	mouse_in        bool
	hovered         map[u64]bool
	pressed         map[u64]bool
	debug           DebugFlags
mut:
	down_path      []u64
	capture        u64       // hitbox id receiving moves and the release while dragging
	target_bounds  prim.Rect // bounds of the hitbox whose handler is running
	dispatching    int
	paint_timer    u64
	prepaint_timer u64
	in_prepaint    bool
	skipped        bool
	last_click     f64
	last_pos       prim.Point
	click_count    int
	hash           gpu.SpatialHash
	closed         bool
	root_el        ?Element
	root_node      layout.NodeId
	needs_prepaint bool
	needs_paint    bool
	pending_keys   []Keystroke
	deferred       []Deferred
	state          map[u64]voidptr
	state_used     map[u64]bool
	fresh_views    []prim.Rect         // bounds of views rendered this frame (paint flashing)
	close_handlers map[u32]AnyListener // per view: may call cx.prevent_close()
	drop_handlers  map[u32]AnyListener
	close_vetoed   bool
	patches        []PatchSite
	patch_scene    gpu.Scene
	patchable      bool  // the scene kept paint order (no layer reordering)
	restyled       []int // hitbox indices to patch
	full_paint     bool  // something other than hover/press styles needs a paint
	drag           DragState
	surface_ok     bool       // the GPU surface exists (created at the first frame)
	tracking       Tracking   // closure lifetimes of layout and prepaint builds
	layout_len     int        // layout nodes made by the last build (prepaint adds scratch nodes)
	thumbs         ThumbState // auto-hiding scroll thumbs (scrollbar.v)
}

pub fn (w &Window) size() prim.Size {
	return w.pw.size()
}

pub fn (w &Window) scale() f32 {
	return w.pw.scale()
}

pub fn (mut w Window) invalidate() {
	w.dirty = true
	w.pw.request_frame()
}

pub fn (mut w Window) invalidate_prepaint() {
	w.needs_prepaint = true
	w.pw.request_frame()
}

pub fn (mut w Window) invalidate_paint() {
	w.needs_paint = true
	w.full_paint = true
	w.pw.request_frame()
}

fn (w &Window) device_size() (int, int) {
	s := w.pw.size()
	k := w.pw.scale()
	return int(s.width * k + 0.5), int(s.height * k + 0.5)
}

// draw_frame runs the cheapest pipeline that covers what changed, then draws and presents.
pub fn (mut w Window) draw_frame() {
	if w.closed {
		return
	}
	sw := time.new_stopwatch()
	w.a11y_poll()
	if w.dirty {
		w.build()
	} else if w.needs_prepaint {
		w.prepaint_phase()
		w.paint_phase()
		w.stats.prepaints++
	} else if w.needs_paint {
		w.needs_paint = false
		if !w.try_patch() {
			w.paint_phase()
			w.stats.paints++
		}
	}
	mut app := w.app
	if !w.surface_ok {
		dw, dh := w.device_size()
		if dw <= 0 || dh <= 0 {
			return
		}
		w.attach_surface() or {
			// No GPU surface means nothing can ever be drawn: close the window rather than
			// leave an invisible one running (the last window closing quits the app).
			eprintln('quadra: cannot create the window surface: ${err}')
			w.pw.close()
			return
		}
	}
	for u in app.text.take_uploads() {
		app.device.upload(u) or { eprintln('quadra: atlas upload failed: ${err}') }
	}
	app.device.draw(mut w.surface, &w.scene, w.clear_color) or {
		eprintln('quadra: draw failed: ${err}')
	}
	w.scene.mark_drawn()
	p0 := sw.elapsed().microseconds()
	w.surface.present() or { eprintln('quadra: present failed: ${err}') }
	w.stats.present_us = sw.elapsed().microseconds() - p0
	if w.stats.first_present == 0 {
		w.stats.first_present = time.sys_mono_now()
	}
	st := app.device.stats()
	w.stats.frames++
	w.stats.draw_calls = st.draw_calls
	w.stats.instances = st.instances
	w.stats.last_frame_us = sw.elapsed().microseconds()
	app.after_frame(mut w)
}

fn (mut w Window) build() {
	sw := time.new_stopwatch()
	w.dirty = false
	mut app := w.app
	w.fresh_views.clear()
	b0 := gc_heap_usage().total_bytes
	mut root := app.render_any(w.root.id, w)
	t_render := sw.elapsed().microseconds()
	b_render := gc_heap_usage().total_bytes
	w.tree.clear()
	mut lcx := LayoutCx{
		app:        app
		win:        w
		tree:       &w.tree
		text:       app.text
		text_style: app.base_text_style()
	}
	w.begin_pass(.layout)
	node := root.request_layout(mut lcx)
	w.tracking.phase = .idle
	w.tree.compute(node, w.pw.size())
	t_layout := sw.elapsed().microseconds()
	b_layout := gc_heap_usage().total_bytes
	w.root_el = root
	w.root_node = node
	w.layout_len = w.tree.len()
	w.prepaint_phase()
	t_prepaint := sw.elapsed().microseconds()
	b_prepaint := gc_heap_usage().total_bytes
	w.paint_phase()
	t_paint := sw.elapsed().microseconds()
	b_paint := gc_heap_usage().total_bytes
	w.stats.phase_bytes = [u64(b_render - b0), u64(b_layout - b_render), u64(b_prepaint - b_layout),
		u64(b_paint - b_prepaint)]!
	w.stats.phase_us = [t_render, t_layout - t_render, t_prepaint - t_layout, t_paint - t_prepaint]!
	app.text.end_frame()
	w.end_pass(.layout) // the previous tree is gone: free what building it allocated
	w.stats.builds++
	w.stats.layout_nodes = w.tree.len()
	w.stats.last_build_us = sw.elapsed().microseconds()
}

fn (mut w Window) prepaint_phase() {
	w.needs_prepaint = false
	mut root := w.root_el or { return }
	mut app := w.app
	w.hitboxes.clear()
	w.handlers.clear()
	w.key_nodes.clear()
	w.key_handlers.clear()
	w.test_ids.clear()
	w.texts.clear()
	w.semantics.clear()
	w.deferred.clear()
	w.state_used.clear()
	w.drag.sources.clear()
	// Prepaint lays out rows and overlays again; drop the nodes the last prepaint added.
	w.tree.truncate(w.layout_len)
	w.begin_pass(.prepaint)
	mut pcx := PrepaintCx{
		app:        app
		win:        w
		tree:       &w.tree
		text:       app.text
		text_style: app.base_text_style()
	}
	root.prepaint(w.tree.rect(w.root_node), mut pcx)
	w.prepaint_drag(mut pcx)
	// Overlays prepaint last so their hitboxes are on top.
	w.deferred.sort(a.priority < b.priority)
	for i in 0 .. w.deferred.len {
		mut d := w.deferred[i]
		d.el.prepaint(d.bounds, mut pcx)
	}
	w.drop_unused_state()
	w.send_a11y()
	w.in_prepaint = true
	w.update_hover()
	w.in_prepaint = false
	w.end_pass(.prepaint)
}

fn (mut w Window) paint_phase() {
	w.needs_paint = false
	w.full_paint = false
	w.thumbs.next_paint()
	w.restyled.clear()
	w.patches.clear()
	for mut hb in w.hitboxes {
		hb.patch_site = -1 // culled elements record nothing this time
	}
	mut root := w.root_el or { return }
	mut app := w.app
	w.scene.clear()
	mut cx := PaintCx{
		app:        app
		win:        w
		scene:      &w.scene
		text:       app.text
		scale:      w.pw.scale()
		text_style: app.base_text_style()
	}
	root.paint(w.tree.rect(w.root_node), mut cx)
	for i in 0 .. w.deferred.len {
		mut d := w.deferred[i]
		d.el.paint(d.bounds, mut cx)
	}
	w.paint_debug(mut cx)
	w.patchable = !w.scene.optimize(mut w.hash)
}
