// Div: the flex/grid container element. Builder methods live in div_*.v.
module ui

import quadra.core
import quadra.prim
import quadra.gpu
import quadra.layout
import quadra.platform

pub struct BoxShadow {
pub:
	offset_x f32
	offset_y f32
	blur     f32
	spread   f32
	color    prim.Color
}

// Visual is the paint-only part of a style.
pub struct Visual {
pub mut:
	background     prim.Color
	gradient_to    ?prim.Color
	gradient_angle f32
	border_color   prim.Color
	radii          prim.Corners
	shadows        []BoxShadow
	opacity        f32 = 1
	translate      prim.Point // paint-only transform (div_motion.v)
	scale          f32 = 1
	rotate         f32 // degrees, clockwise
}

// Patch overrides parts of a Visual (and the text color) while hovered, pressed or focused.
pub struct Patch {
pub:
	background   ?prim.Color
	border_color ?prim.Color
	text_color   ?prim.Color
	opacity      ?f32
	shadows      ?[]BoxShadow
	translate    ?prim.Point
	scale        ?f32
	rotate       ?f32
}

pub struct Handler {
pub:
	kind     core.HandlerKind
	capture  bool
	listener core.AnyListener
}

@[heap]
pub struct Div {
pub mut:
	style    layout.Style
	visual   Visual
	states   &States = unsafe { nil } // hover, active and focus patches (most divs have none)
	text     TextPatch
	children []core.Element
	key      string
	test_id  string
	cursor   ?platform.Cursor
	handlers []Handler
	keys     []KeySpec
	focus_id u64
	tab_idx  int
	key_ctx  string
	sem      Semantics
	region   platform.HitRegion // custom titlebar role
	wheel_x  bool               // the plain wheel scrolls sideways (strips that only scroll horizontally)
	// scroll_at: the scroll position the app owns (scroll_with), else element state.
	scroll_at &ScrollState = unsafe { nil }
mut:
	node              layout.NodeId
	hitbox            int = -1
	scroll            prim.Point
	scroll_target     ScrollTarget
	own_scroll_hitbox bool
	spare             &States = unsafe { nil } // kept by an arena div for its next use (2.6 KB)
}

// States are the patches a Div applies while hovered, pressed or focused.
pub struct States {
pub mut:
	hover  ?Patch
	active ?Patch
	focus  ?Patch
	drop   ?Patch // while an accepted drag is over this drop target
	// hover groups (div_states.v): group_hb is the group the patches follow, set in prepaint
	group        bool
	group_hover  ?Patch
	group_active ?Patch
	group_hb     int = -1
	ring_w       f32 // ring(): a band outside the border box
	ring_color   prim.Color
	// drag and drop (core/drag.v)
	drag      ?core.DragSource
	drop_kind string
	// transitions (div_motion.v)
	transition f32 // ms
	easing     core.Easing
	tr         &Transition = unsafe { nil }
	motion     &Motion     = unsafe { nil } // animate (div_timeline.v)
	clock      &Clock      = unsafe { nil }
}

@[inline]
fn (d &Div) drags() bool {
	return d.states != unsafe { nil } && (d.states.drag != none || d.states.drop_kind != '')
}

@[inline]
fn (d &Div) has_focus_patch() bool {
	return d.states != unsafe { nil } && d.states.focus != none
}

// div makes a container. During a view's render it comes from the view's render arena
// (arena.v): it belongs to that render, do not keep it (specs/07).
pub fn div() &Div {
	return new_div()
}

fn (d &Div) interactive() bool {
	return d.handlers.len > 0 || d.restyles() || d.cursor != none || d.drags() || d.test_id != ''
		|| d.focus_id != 0 || d.region != .client
}

pub fn (mut d Div) request_layout(mut cx core.LayoutCx) layout.NodeId {
	saved := cx.text_style
	cx.text_style = d.text.apply(saved)
	mut kids := []layout.NodeId{cap: d.children.len}
	for mut c in d.children {
		kids << c.request_layout(mut cx)
	}
	cx.text_style = saved
	d.node = cx.tree.node(d.style, kids)
	return d.node
}

fn (d &Div) clips() bool {
	return d.style.overflow_x != .visible || d.style.overflow_y != .visible
}

pub fn (mut d Div) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	cx.enter(d.key)
	defer {
		cx.leave()
	}
	d.hitbox = -1
	d.own_scroll_hitbox = false
	if d.interactive() {
		d.hitbox = cx.insert_hitbox(bounds, core.HitboxOptions{
			cursor:          d.cursor
			hover_sensitive: d.restyles()
			focus_id:        d.focus_id
			region:          d.region
			drag:            if d.states != unsafe { nil } { d.states.drag } else { none }
			drop_kind:       if d.states != unsafe { nil } { d.states.drop_kind } else { '' }
		})
		for h in d.handlers {
			cx.on_mouse(d.hitbox, h.kind, h.listener, h.capture)
		}
		cx.push_hitbox(d.hitbox)
	}
	saved_group := d.enter_group(mut cx)
	if d.test_id != '' {
		cx.register_test_id(d.test_id, bounds)
	}
	key_node, sem := d.prepaint_input(bounds, mut cx)
	if d.states != unsafe { nil } && (d.states.transition > 0 || d.states.motion != unsafe { nil }) {
		d.prepaint_transition(mut cx)
	}
	if d.clips() {
		cx.push_clip(bounds)
	}
	saved := cx.text_style
	cx.text_style = d.text.apply(saved)
	kids := cx.tree.children(d.node)
	visible := cx.clip()
	for i, mut c in d.children {
		r := cx.tree.rect(kids[i]).translate(bounds.x - d.scroll.x, bounds.y - d.scroll.y)
		// Children outside the visible area cost nothing (long scrolled lists).
		if d.clips() && !r.intersects(visible) && r.width > 0 {
			continue
		}
		c.prepaint(r, mut cx)
	}
	cx.text_style = saved
	cx.group = saved_group
	if d.clips() {
		cx.pop_clip()
	}
	if sem >= 0 {
		cx.pop_semantics()
	}
	if key_node >= 0 {
		cx.pop_key_node()
	}
	if d.hitbox >= 0 {
		cx.pop_hitbox()
	}
}

pub fn (mut d Div) paint(bounds prim.Rect, mut cx core.PaintCx) {
	restyles := d.restyles()
	site := if restyles { cx.begin_patch(d.hitbox, voidptr(d), repaint_div, bounds) } else { -1 }
	d.paint_box(bounds, restyles, mut cx)
	cx.end_patch(site)
}

// repaint_div paints one Div again for a paint patch.
fn repaint_div(el voidptr, bounds prim.Rect, mut cx core.PaintCx) {
	mut d := unsafe { &Div(el) }
	d.paint_box(bounds, true, mut cx)
}

fn (mut d Div) paint_box(bounds prim.Rect, restyles bool, mut cx core.PaintCx) {
	mut v, tp := d.resolved(cx)
	if d.states != unsafe { nil } && d.states.tr != unsafe { nil } {
		v = d.transitioned(v, mut cx)
	}
	if d.states != unsafe { nil } && d.states.clock != unsafe { nil } {
		v = d.animated(v, mut cx)
	}
	saved_opacity := cx.opacity
	cx.opacity *= v.opacity
	moved := v.scale != 1 || v.rotate != 0 || v.translate != prim.Point{}
	saved_xf := if moved {
		cx.push_transform(prim.Point{bounds.x + bounds.width / 2, bounds.y + bounds.height / 2},
			v.translate, v.scale, v.rotate)
	} else {
		cx.xf
	}
	for s in v.shadows {
		cx.shadow(gpu.Shadow{
			bounds: prim.rect(bounds.x + s.offset_x - s.spread, bounds.y + s.offset_y - s.spread,

				bounds.width + s.spread * 2, bounds.height + s.spread * 2)
			radii:  v.radii
			blur:   s.blur
			color:  s.color
		})
	}
	q := gpu.Quad{
		bounds:         bounds
		background:     v.background
		gradient_to:    v.gradient_to
		gradient_angle: v.gradient_angle
		border_widths:  d.style.border
		border_color:   v.border_color
		radii:          v.radii
	}
	if restyles {
		cx.quad_kept(q) // same primitive count in every state, so the patch fits
	} else {
		cx.quad(q)
	}
	if d.clips() {
		cx.push_clip(bounds, v.radii.top_left)
	}
	saved := cx.text_style
	cx.text_style = tp.apply(saved)
	kids := cx.win.tree.children(d.node)
	visible := cx.clip_rect()
	for i, mut c in d.children {
		r := cx.win.tree.rect(kids[i]).translate(bounds.x - d.scroll.x, bounds.y - d.scroll.y)
		if d.clips() && !r.intersects(visible) && r.width > 0 {
			continue
		}
		c.paint(r, mut cx)
	}
	cx.text_style = saved
	if d.clips() {
		cx.pop_clip()
	}
	d.paint_thumbs(bounds, mut cx)
	if d.states != unsafe { nil } && d.states.ring_w > 0 {
		d.paint_ring(bounds, v.radii, mut cx)
	}
	if d.focus_id != 0 && !d.has_focus_patch() && cx.is_focused(d.focus_id) && cx.focus_visible() {
		// Default focus mark: the box's own 1 px edge in the focus color, focus-visible only.
		cx.quad(gpu.Quad{
			bounds:        bounds
			border_widths: prim.edges(1)
			border_color:  cx.win.focus_color
			radii:         v.radii
		})
	}
	cx.restore_transform(saved_xf)
	cx.opacity = saved_opacity
}
