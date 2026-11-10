// Paint patches: hover and press restyle single elements. During a full paint, every
// hover-sensitive element records where its primitives went. When a frame only restyles
// such elements, each is painted again on its own and overwrites its old range in place,
// instead of painting the whole window (specs/12: "a hover repaints one chunk").
module core

import quadra.prim
import quadra.gpu
import quadra.text

pub type PatchPaint = fn (el prim.Holder, bounds prim.Rect, mut cx PaintCx)

struct PatchSite {
	el      prim.Holder
	paint   PatchPaint = unsafe { nil }
	bounds  prim.Rect
	start   int
	mask    gpu.ContentMask
	style   text.TextStyle
	opacity f32
	xf      Xform
mut:
	end int
}

// begin_patch starts recording the primitives of a hover-sensitive element (hitbox is the
// element's hitbox index). Returns a handle for end_patch, or -1 when not recording.
pub fn (mut cx PaintCx) begin_patch(hitbox int, el prim.Holder, f PatchPaint, bounds prim.Rect) int {
	// Under a rotation the patch would need the scene's transforms: paint everything.
	if cx.repatching || cx.xf.id != 0 || hitbox < 0 || hitbox >= cx.win.hitboxes.len {
		return -1
	}
	mut w := cx.win
	w.patches << PatchSite{
		el:      el
		paint:   f
		bounds:  bounds
		start:   cx.scene.instances.len
		mask:    cx.masks.last()
		style:   cx.text_style
		opacity: cx.opacity
		xf:      cx.xf
	}
	w.hitboxes[hitbox].patch_site = w.patches.len - 1
	return w.patches.len - 1
}

pub fn (mut cx PaintCx) end_patch(site int) {
	if site >= 0 {
		mut w := cx.win
		w.patches[site].end = cx.scene.instances.len
	}
}

// restyle asks for a paint of hover-sensitive hitboxes (indices, valid until the next
// prepaint, which repaints everything anyway) whose hover or press state changed.
fn (mut w Window) restyle(hitboxes []int) {
	w.restyled << hitboxes
	w.needs_paint = true
	w.pw.request_frame()
}

// try_patch repaints only the restyled elements. It returns false, with the scene possibly
// half patched, when that is not possible; the caller then paints everything.
fn (mut w Window) try_patch() bool {
	if w.full_paint || !w.patchable || w.restyled.len == 0 || w.debug.flashing || w.debug.inspector
		|| w.debug.profiler {
		return false
	}
	mut app := w.app
	for hb in w.restyled {
		if hb >= w.hitboxes.len || w.hitboxes[hb].patch_site < 0 {
			return false
		}
		site := w.patches[w.hitboxes[hb].patch_site]
		w.patch_scene.clear()
		mut cx := PaintCx{
			app:        app
			win:        w
			scene:      &w.patch_scene
			text:       app.text
			scale:      w.pw.scale()
			text_style: site.style
			opacity:    site.opacity
			xf:         site.xf
			repatching: true
			masks:      [site.mask]
		}
		site.paint(site.el, site.bounds, mut cx)
		n := site.end - site.start
		if w.patch_scene.instances.len != n || w.patch_scene.xforms.len > 1 {
			return false // the element now draws a different number of primitives
		}
		for k in 0 .. n {
			a, b := w.scene.instances[site.start + k], w.patch_scene.instances[k]
			if a.kind != b.kind || a.page != b.page {
				return false // batches would change
			}
			w.scene.instances[site.start + k] = b
		}
		w.scene.touch(site.start, site.end)
	}
	w.restyled.clear()
	w.stats.patches++
	return true
}
