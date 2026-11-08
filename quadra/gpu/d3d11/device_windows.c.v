// D3D11 device creation and resource factories.
module d3d11

import quadra.platform.win

type FnCreateDevice = fn (adapter voidptr, driver int, software voidptr, flags u32, levels &u32, nlevels u32, sdk u32, device &voidptr, level &u32, ctx &voidptr) int

type FnOut1 = fn (this voidptr, out &voidptr) int

type FnCreate2 = fn (this voidptr, a voidptr, out &voidptr) int // (desc, out)

type FnCreate3 = fn (this voidptr, a voidptr, b voidptr, out &voidptr) int // (desc, init, out)

type FnCreateShader = fn (this voidptr, code voidptr, size usize, linkage voidptr, out &voidptr) int

@[heap]
pub struct Device {
pub:
	device voidptr
	ctx    voidptr
	warp   bool // software rasterizer (forced, or no hardware adapter)
	debug  bool // debug layer active
}

@[params]
pub struct DeviceOptions {
pub:
	force_warp bool
	debug      bool // tries the debug layer, falls back silently when it is not installed
}

pub fn new_device(opts DeviceOptions) !&Device {
	lib := win.load_system_dll('d3d11.dll')!
	create := unsafe { FnCreateDevice(win.dll_proc(lib, 'D3D11CreateDevice')!) }
	levels := [u32(0xb000), 0xa100, 0xa000]
	drivers := if opts.force_warp { [driver_warp] } else { [driver_hardware, driver_warp] }
	// No driver worker thread: measured, the driver's thread cost more CPU than it saved
	// (list scrolling 0.63 -> 0.49 ms a frame, process total) and 1.3 MB.
	base := create_bgra_support | create_prevent_threading_optimizations
	flag_sets := if opts.debug { [base | create_debug, base] } else { [base] }
	mut last := 0
	for driver in drivers {
		for flags in flag_sets {
			mut dev := unsafe { nil }
			mut ctx := unsafe { nil }
			mut level := u32(0)
			last = create(unsafe { nil }, driver, unsafe { nil }, flags, &levels[0],
				u32(levels.len), sdk_version, &dev, &level, &ctx)
			if last >= 0 {
				return &Device{
					device: dev
					ctx:    ctx
					warp:   driver == driver_warp
					debug:  flags & create_debug != 0
				}
			}
		}
	}
	return error('D3D11CreateDevice failed: HRESULT 0x${u32(last).hex()}')
}

fn (d &Device) call2(slot_ int, a voidptr, what string) !voidptr {
	f := unsafe { FnCreate2(win.slot(d.device, slot_)) }
	mut out := unsafe { nil }
	win.check(f(d.device, a, &out), what)!
	return out
}

fn (d &Device) call3(slot_ int, a voidptr, b voidptr, what string) !voidptr {
	f := unsafe { FnCreate3(win.slot(d.device, slot_)) }
	mut out := unsafe { nil }
	win.check(f(d.device, a, b, &out), what)!
	return out
}

pub fn (d &Device) create_buffer(desc &BufferDesc, init &SubresourceData) !voidptr {
	return d.call3(dev_create_buffer, desc, init, 'CreateBuffer')
}

pub fn (d &Device) create_texture2d(desc &Texture2dDesc, init &SubresourceData) !voidptr {
	return d.call3(dev_create_texture2d, desc, init, 'CreateTexture2D')
}

// create_srv passes a nil desc to view the whole resource in its own format.
pub fn (d &Device) create_srv(resource voidptr, desc &SrvDesc) !voidptr {
	return d.call3(dev_create_srv, resource, desc, 'CreateShaderResourceView')
}

pub fn (d &Device) create_rtv(resource voidptr) !voidptr {
	return d.call3(dev_create_rtv, resource, unsafe { nil }, 'CreateRenderTargetView')
}

pub fn (d &Device) create_blend_state(desc &BlendDesc) !voidptr {
	return d.call2(dev_create_blend_state, desc, 'CreateBlendState')
}

pub fn (d &Device) create_rasterizer_state(desc &RasterizerDesc) !voidptr {
	return d.call2(dev_create_rasterizer_state, desc, 'CreateRasterizerState')
}

pub fn (d &Device) create_sampler_state(desc &SamplerDesc) !voidptr {
	return d.call2(dev_create_sampler_state, desc, 'CreateSamplerState')
}

pub fn (d &Device) create_vertex_shader(code []u8) !voidptr {
	f := unsafe { FnCreateShader(win.slot(d.device, dev_create_vertex_shader)) }
	mut out := unsafe { nil }
	win.check(f(d.device, code.data, usize(code.len), unsafe { nil }, &out), 'CreateVertexShader')!
	return out
}

pub fn (d &Device) create_pixel_shader(code []u8) !voidptr {
	f := unsafe { FnCreateShader(win.slot(d.device, dev_create_pixel_shader)) }
	mut out := unsafe { nil }
	win.check(f(d.device, code.data, usize(code.len), unsafe { nil }, &out), 'CreatePixelShader')!
	return out
}

// dxgi_factory walks device -> adapter -> factory, so the swap chain uses the same adapter.
pub fn (d &Device) dxgi_factory() !voidptr {
	dxgi_dev := win.query_interface(d.device, &iid_dxgi_device)!
	defer {
		win.release(dxgi_dev)
	}
	get_adapter := unsafe { FnOut1(win.slot(dxgi_dev, dxgi_device_get_adapter)) }
	mut adapter := unsafe { nil }
	win.check(get_adapter(dxgi_dev, &adapter), 'GetAdapter')!
	defer {
		win.release(adapter)
	}
	get_parent := unsafe { FnCreate2(win.slot(adapter, dxgi_object_get_parent)) }
	mut factory := unsafe { nil }
	win.check(get_parent(adapter, voidptr(&iid_dxgi_factory2), &factory),
		'GetParent(IDXGIFactory2)')!
	return factory
}
