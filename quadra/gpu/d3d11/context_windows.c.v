// Immediate-context commands used by the renderer.
module d3d11

import quadra.platform.win

type FnSetPtrArray = fn (this voidptr, start u32, n u32, items &voidptr)

type FnSetPtr = fn (this voidptr, p voidptr)

type FnSetShader = fn (this voidptr, shader voidptr, classes voidptr, n u32)

type FnSetU32 = fn (this voidptr, v u32)

type FnDrawInstanced = fn (this voidptr, verts u32, insts u32, first_vert u32, first_inst u32)

type FnMap = fn (this voidptr, res voidptr, sub u32, kind u32, flags u32, out &MappedSubresource) int

type FnUnmap = fn (this voidptr, res voidptr, sub u32)

type FnSetRenderTargets = fn (this voidptr, n u32, rtvs &voidptr, dsv voidptr)

type FnSetBlend = fn (this voidptr, state voidptr, factor &f32, mask u32)

type FnSetViewports = fn (this voidptr, n u32, vps &Viewport)

type FnCopy = fn (this voidptr, dst voidptr, src voidptr)

type FnUpdateSub = fn (this voidptr, dst voidptr, sub u32, box voidptr, data voidptr, row u32, depth u32)

type FnClearRtv = fn (this voidptr, rtv voidptr, rgba &f32)

type FnVoid = fn (this voidptr)

pub struct Box {
pub:
	left   u32
	top    u32
	front  u32
	right  u32
	bottom u32
	back   u32 = 1
}

fn (d &Device) ctx_fn(slot_ int) voidptr {
	return win.slot(d.ctx, slot_)
}

pub fn (d &Device) clear(rtv voidptr, r f32, g f32, b f32, a f32) {
	rgba := [r, g, b, a]!
	clear_rtv := unsafe { FnClearRtv(d.ctx_fn(ctx_clear_rtv)) }
	clear_rtv(d.ctx, rtv, &rgba[0])
}

pub fn (d &Device) set_render_target(rtv voidptr, width f32, height f32) {
	mut target := rtv
	om_set_render_targets := unsafe { FnSetRenderTargets(d.ctx_fn(ctx_om_set_render_targets)) }
	om_set_render_targets(d.ctx, 1, &target, unsafe { nil })
	vp := Viewport{
		width:  width
		height: height
	}
	rs_set_viewports := unsafe { FnSetViewports(d.ctx_fn(ctx_rs_set_viewports)) }
	rs_set_viewports(d.ctx, 1, &vp)
}

pub fn (d &Device) set_pipeline(vs voidptr, ps voidptr, blend voidptr, raster voidptr) {
	ia_set_input_layout := unsafe { FnSetPtr(d.ctx_fn(ctx_ia_set_input_layout)) }
	ia_set_input_layout(d.ctx, unsafe { nil })
	ia_set_topology := unsafe { FnSetU32(d.ctx_fn(ctx_ia_set_topology)) }
	ia_set_topology(d.ctx, topology_triangle_strip)
	vs_set_shader := unsafe { FnSetShader(d.ctx_fn(ctx_vs_set_shader)) }
	vs_set_shader(d.ctx, vs, unsafe { nil }, 0)
	ps_set_shader := unsafe { FnSetShader(d.ctx_fn(ctx_ps_set_shader)) }
	ps_set_shader(d.ctx, ps, unsafe { nil }, 0)
	factor := [f32(0), 0, 0, 0]!
	om_set_blend_state := unsafe { FnSetBlend(d.ctx_fn(ctx_om_set_blend_state)) }
	om_set_blend_state(d.ctx, blend, &factor[0], 0xFFFFFFFF)
	rs_set_state := unsafe { FnSetPtr(d.ctx_fn(ctx_rs_set_state)) }
	rs_set_state(d.ctx, raster)
}

pub fn (d &Device) vs_constant_buffer(slot_ u32, buf voidptr) {
	mut b := buf
	vs_set_constant_buffers := unsafe { FnSetPtrArray(d.ctx_fn(ctx_vs_set_constant_buffers)) }
	vs_set_constant_buffers(d.ctx, slot_, 1, &b)
}

pub fn (d &Device) vs_resource(slot_ u32, srv voidptr) {
	mut s := srv
	vs_set_shader_resources := unsafe { FnSetPtrArray(d.ctx_fn(ctx_vs_set_shader_resources)) }
	vs_set_shader_resources(d.ctx, slot_, 1, &s)
}

pub fn (d &Device) ps_resource(slot_ u32, srv voidptr) {
	mut s := srv
	ps_set_shader_resources := unsafe { FnSetPtrArray(d.ctx_fn(ctx_ps_set_shader_resources)) }
	ps_set_shader_resources(d.ctx, slot_, 1, &s)
}

pub fn (d &Device) ps_sampler(slot_ u32, sampler voidptr) {
	mut s := sampler
	ps_set_samplers := unsafe { FnSetPtrArray(d.ctx_fn(ctx_ps_set_samplers)) }
	ps_set_samplers(d.ctx, slot_, 1, &s)
}

pub fn (d &Device) draw_instanced(vertices u32, instances u32, first_instance u32) {
	draw_instanced := unsafe { FnDrawInstanced(d.ctx_fn(ctx_draw_instanced)) }
	draw_instanced(d.ctx, vertices, instances, 0, first_instance)
}

// write_discard replaces a dynamic buffer's content with `size` bytes from `data`.
pub fn (d &Device) write_discard(res voidptr, data voidptr, size int) ! {
	mut m := MappedSubresource{}
	map_ := unsafe { FnMap(d.ctx_fn(ctx_map)) }
	win.check(map_(d.ctx, res, 0, map_write_discard, 0, &m), 'Map')!
	unsafe { vmemcpy(m.data, data, size) }
	unmap := unsafe { FnUnmap(d.ctx_fn(ctx_unmap)) }
	unmap(d.ctx, res, 0)
}

pub fn (d &Device) map_read(res voidptr) !MappedSubresource {
	mut m := MappedSubresource{}
	map_ := unsafe { FnMap(d.ctx_fn(ctx_map)) }
	win.check(map_(d.ctx, res, 0, map_read, 0, &m), 'Map(read)')!
	return m
}

pub fn (d &Device) unmap(res voidptr) {
	unmap := unsafe { FnUnmap(d.ctx_fn(ctx_unmap)) }
	unmap(d.ctx, res, 0)
}

pub fn (d &Device) update_region(tex voidptr, b Box, data voidptr, row_pitch u32) {
	update_subresource := unsafe { FnUpdateSub(d.ctx_fn(ctx_update_subresource)) }
	update_subresource(d.ctx, tex, 0, &b, data, row_pitch, 0)
}

pub fn (d &Device) copy_resource(dst voidptr, src voidptr) {
	copy_resource := unsafe { FnCopy(d.ctx_fn(ctx_copy_resource)) }
	copy_resource(d.ctx, dst, src)
}

pub fn (d &Device) flush() {
	flush := unsafe { FnVoid(d.ctx_fn(ctx_flush)) }
	flush(d.ctx)
}

// unbind_all drops every binding (ClearState) and flushes, so released resources go now.
pub fn (d &Device) unbind_all() {
	clear_state := unsafe { FnVoid(d.ctx_fn(ctx_clear_state)) }
	clear_state(d.ctx)
	d.flush()
}
