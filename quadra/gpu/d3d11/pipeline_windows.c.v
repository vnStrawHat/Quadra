// The uber-instance pipeline state: shaders, blend/raster/sampler states and buffers.
module d3d11

import quadra.platform.win
import quadra.gpu

const vs_code = $embed_file('shaders/prims_vs.cso')
const ps_code = $embed_file('shaders/prims_ps.cso')

struct Pipeline {
mut:
	vs        voidptr
	ps        voidptr
	blend     voidptr
	raster    voidptr
	sampler   voidptr
	globals   voidptr
	instances voidptr
	srv       voidptr
	capacity  int
	held      voidptr // the scene whose instances the buffer holds
	held_rev  u64
	xforms    voidptr // Scene.xforms (t3)
	xsrv      voidptr
	xcap      int
}

fn new_pipeline(d &Device) !Pipeline {
	mut blend := BlendDesc{}
	blend.rt[0] = RtBlendDesc{
		blend_enable: 1
		// Dual-source blending (SV_Target1 is a per-channel weight) so ClearType text blends
		// each subpixel on its own; for everything else it is premultiplied alpha.
		src_blend:        blend_one
		dest_blend:       blend_inv_src1_color
		blend_op:         blend_op_add
		src_blend_alpha:  blend_one
		dest_blend_alpha: blend_inv_src1_alpha
		blend_op_alpha:   blend_op_add
		write_mask:       0xF
	}
	return Pipeline{
		vs:      d.create_vertex_shader(vs_code.to_bytes())!
		ps:      d.create_pixel_shader(ps_code.to_bytes())!
		blend:   d.create_blend_state(&blend)!
		raster:  d.create_rasterizer_state(&RasterizerDesc{})!
		sampler: d.create_sampler_state(&SamplerDesc{
			filter: filter_linear
		})!
		globals: d.create_buffer(&BufferDesc{
			byte_width:       16
			usage:            usage_dynamic
			bind_flags:       bind_constant_buffer
			cpu_access_flags: cpu_access_write
		}, unsafe { nil })!
	}
}

// upload_xforms sends the scene's transforms (whole: they are few and change only with a
// new scene) into a structured buffer sized by doubling.
fn (mut p Pipeline) upload_xforms(d &Device, xs []gpu.Rigid) ! {
	if xs.len == 1 && p.xcap > 0 {
		return
	}
	if xs.len > p.xcap {
		mut cap := if p.xcap == 0 { 64 } else { p.xcap }
		for cap < xs.len {
			cap *= 2
		}
		win.release(p.xsrv)
		win.release(p.xforms)
		stride := u32(sizeof(gpu.Rigid))
		p.xforms = d.create_buffer(&BufferDesc{
			byte_width:            u32(cap) * stride
			usage:                 usage_default
			bind_flags:            bind_shader_resource
			misc_flags:            misc_buffer_structured
			structure_byte_stride: stride
		}, unsafe { nil })!
		p.xsrv = d.create_srv(p.xforms, &SrvDesc{
			format:    format_unknown
			dimension: srv_dimension_buffer
			u0:        0
			u1:        u32(cap)
		})!
		p.xcap = cap
	}
	d.update_region(p.xforms, Box{
		right:  u32(xs.len * int(sizeof(gpu.Rigid)))
		bottom: 1
	}, unsafe { &xs[0] }, 0)
}

fn (mut p Pipeline) ensure_capacity(d &Device, n int) ! {
	if n <= p.capacity {
		return
	}
	mut cap := if p.capacity == 0 { 1024 } else { p.capacity }
	for cap < n {
		cap *= 2
	}
	win.release(p.srv)
	win.release(p.instances)
	p.instances = d.create_buffer(&BufferDesc{
		byte_width:            u32(cap * int(sizeof(gpu.Instance)))
		usage:                 usage_default // UpdateSubresource can write any range
		bind_flags:            bind_shader_resource
		misc_flags:            misc_buffer_structured
		structure_byte_stride: u32(sizeof(gpu.Instance))
	}, unsafe { nil })!
	p.srv = d.create_srv(p.instances, &SrvDesc{
		format:    format_unknown
		dimension: srv_dimension_buffer
		u0:        0
		u1:        u32(cap)
	})!
	p.capacity = cap
	p.held = unsafe { nil } // the new buffer holds nothing yet
}

struct Globals {
	width  f32
	height f32
	offset u32
	pad    f32
}

// draw renders every batch of the scene into rtv. Pages are looked up per batch.
fn (mut p Pipeline) draw(d &Device, rtv voidptr, width int, height int, scene &gpu.Scene, mono []AtlasPage, poly []AtlasPage) !int {
	d.set_render_target(rtv, f32(width), f32(height))
	d.set_pipeline(p.vs, p.ps, p.blend, p.raster)
	if scene.instances.len == 0 {
		return 0
	}
	p.ensure_capacity(d, scene.instances.len)!
	// The instance buffer keeps the last scene: a patched frame of that scene sends only its
	// dirty range (a hover restyle is a few instances instead of the whole window).
	stride := int(sizeof(gpu.Instance))
	mut start, mut end := 0, scene.instances.len
	if voidptr(scene) == p.held && scene.revision == p.held_rev {
		start, end = scene.dirty_range()
	} else if scene.xforms.len > 0 {
		p.upload_xforms(d, scene.xforms)!
	}
	if end > start {
		d.update_region(p.instances, Box{
			left:   u32(start * stride)
			right:  u32(end * stride)
			bottom: 1
		}, unsafe { &scene.instances[start] }, 0)
	}
	p.held, p.held_rev = voidptr(scene), scene.revision
	d.vs_resource(0, p.srv)
	d.ps_resource(0, p.srv)
	d.vs_resource(3, p.xsrv)
	d.ps_resource(3, p.xsrv)
	d.ps_sampler(0, p.sampler)
	d.vs_constant_buffer(0, p.globals)
	batches := scene.batches()
	for b in batches {
		g := Globals{f32(width), f32(height), u32(b.start), 0}
		d.write_discard(p.globals, &g, int(sizeof(Globals)))!
		if b.mono_page >= 0 && b.mono_page < mono.len {
			d.ps_resource(1, mono[b.mono_page].srv)
		}
		if b.poly_page >= 0 && b.poly_page < poly.len {
			d.ps_resource(2, poly[b.poly_page].srv)
		}
		d.draw_instanced(4, u32(b.count), 0)
	}
	return batches.len
}
