// D3D11 and DXGI types, constants and vtable slots. Layouts mirror the SDK
// headers (checked by spikes/m0/layout_check); slots come from tools/com_slots.sh.
module d3d11

import quadra.platform.win

pub struct SampleDesc {
pub mut:
	count   u32 = 1
	quality u32
}

pub struct BufferDesc {
pub mut:
	byte_width            u32
	usage                 u32
	bind_flags            u32
	cpu_access_flags      u32
	misc_flags            u32
	structure_byte_stride u32
}

pub struct SubresourceData {
pub mut:
	sys_mem             voidptr
	sys_mem_pitch       u32
	sys_mem_slice_pitch u32
}

pub struct Texture2dDesc {
pub mut:
	width            u32
	height           u32
	mip_levels       u32 = 1
	array_size       u32 = 1
	format           u32
	sample_desc      SampleDesc
	usage            u32
	bind_flags       u32
	cpu_access_flags u32
	misc_flags       u32
}

// SrvDesc flattens the view-dimension union to its largest member (4 x u32).
pub struct SrvDesc {
pub mut:
	format    u32
	dimension u32
	u0        u32
	u1        u32
	u2        u32
	u3        u32
}

pub struct Viewport {
pub mut:
	x         f32
	y         f32
	width     f32
	height    f32
	min_depth f32
	max_depth f32 = 1
}

pub struct MappedSubresource {
pub mut:
	data        voidptr
	row_pitch   u32
	depth_pitch u32
}

pub struct RtBlendDesc {
pub mut:
	blend_enable     int
	src_blend        u32
	dest_blend       u32
	blend_op         u32
	src_blend_alpha  u32
	dest_blend_alpha u32
	blend_op_alpha   u32
	write_mask       u8
}

pub struct BlendDesc {
pub mut:
	alpha_to_coverage int
	independent_blend int
	rt                [8]RtBlendDesc
}

pub struct RasterizerDesc {
pub mut:
	fill_mode               u32 = 3
	cull_mode               u32 = 1
	front_ccw               int
	depth_bias              int
	depth_bias_clamp        f32
	slope_scaled_depth_bias f32
	depth_clip_enable       int = 1
	scissor_enable          int
	multisample_enable      int
	antialiased_line_enable int
}

pub struct SamplerDesc {
pub mut:
	filter         u32
	address_u      u32 = 3
	address_v      u32 = 3
	address_w      u32 = 3
	mip_lod_bias   f32
	max_anisotropy u32 = 1
	comparison     u32 = 1
	border_color   [4]f32
	min_lod        f32
	max_lod        f32 = 3.402823466e+38
}

pub struct SwapChainDesc1 {
pub mut:
	width        u32
	height       u32
	format       u32
	stereo       int
	sample_desc  SampleDesc
	buffer_usage u32
	buffer_count u32
	scaling      u32
	swap_effect  u32
	alpha_mode   u32
	flags        u32
}

pub const format_unknown = u32(0)
pub const format_r8g8b8a8_unorm = u32(28)
pub const format_r8_unorm = u32(61)
pub const format_b8g8r8a8_unorm = u32(87)

pub const usage_default = u32(0)
pub const usage_dynamic = u32(2)
pub const usage_staging = u32(3)

pub const bind_constant_buffer = u32(0x4)
pub const bind_shader_resource = u32(0x8)
pub const bind_render_target = u32(0x20)

pub const cpu_access_write = u32(0x10000)
pub const cpu_access_read = u32(0x20000)
pub const misc_buffer_structured = u32(0x40)

const srv_dimension_buffer = u32(1)
const map_read = u32(1)
const map_write_discard = u32(4)
const blend_one = u32(2)
const blend_inv_src_alpha = u32(6)
const blend_inv_src1_color = u32(17)
const blend_inv_src1_alpha = u32(19)
const blend_op_add = u32(1)
const filter_linear = u32(0x15)
const topology_triangle_strip = u32(5)

const driver_hardware = 1
const driver_warp = 5
const create_bgra_support = u32(0x20)
const create_debug = u32(0x2)
const create_prevent_threading_optimizations = u32(0x8)
const sdk_version = u32(7)

const dxgi_usage_render_target_output = u32(0x20)
const dxgi_swap_effect_flip_discard = u32(4)
const dxgi_alpha_ignore = u32(3)
const dxgi_flag_frame_latency_waitable = u32(0x40)
const dxgi_mwa_no_alt_enter = u32(2)

// IIDs copied from the SDK headers.
const iid_dxgi_device = win.guid('54ec77fa-1377-44e6-8c32-88fd5f44c84c')
const iid_dxgi_factory2 = win.guid('50c83a1c-e072-4c48-87b0-3630fa36a6d0')
const iid_dxgi_swapchain2 = win.guid('a8be2ac4-199f-4946-b331-79599fb98de7')
const iid_texture2d = win.guid('6f15aaf2-d208-4e89-9ab4-489535d34f9c')

// ID3D11Device slots
const dev_create_buffer = 3
const dev_create_texture2d = 5
const dev_create_srv = 7
const dev_create_rtv = 9
const dev_create_vertex_shader = 12
const dev_create_pixel_shader = 15
const dev_create_blend_state = 20
const dev_create_rasterizer_state = 22
const dev_create_sampler_state = 23

// ID3D11DeviceContext slots
const ctx_vs_set_constant_buffers = 7
const ctx_ps_set_shader_resources = 8
const ctx_ps_set_shader = 9
const ctx_ps_set_samplers = 10
const ctx_vs_set_shader = 11
const ctx_map = 14
const ctx_unmap = 15
const ctx_ia_set_input_layout = 17
const ctx_draw_instanced = 21
const ctx_ia_set_topology = 24
const ctx_vs_set_shader_resources = 25
const ctx_om_set_render_targets = 33
const ctx_om_set_blend_state = 35
const ctx_rs_set_state = 43
const ctx_rs_set_viewports = 44
const ctx_copy_resource = 47
const ctx_update_subresource = 48
const ctx_clear_rtv = 50
const ctx_clear_state = 110
const ctx_flush = 111

// DXGI slots
const dxgi_object_get_parent = 6
const dxgi_device_get_adapter = 7
const dxgi_adapter_get_desc = 8
const dxgi_factory_make_window_association = 8
const dxgi_factory2_create_swapchain_for_hwnd = 15
const sc_present = 8
const sc_get_buffer = 9
const sc_resize_buffers = 13
const sc1_set_background_color = 25
const sc2_set_max_frame_latency = 31
const sc2_get_frame_latency_waitable = 33
