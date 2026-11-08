// M0 spike: compares V struct layouts with the real SDK headers.
// Build with MSVC only (it has d3d11.h); the framework itself never includes these headers.
module main

import quadra.gpu.d3d11

#include <d3d11.h>
#include <dxgi1_3.h>

@[typedef]
struct C.D3D11_BUFFER_DESC {}

@[typedef]
struct C.D3D11_SUBRESOURCE_DATA {}

@[typedef]
struct C.D3D11_TEXTURE2D_DESC {}

@[typedef]
struct C.D3D11_SHADER_RESOURCE_VIEW_DESC {}

@[typedef]
struct C.D3D11_VIEWPORT {}

@[typedef]
struct C.D3D11_MAPPED_SUBRESOURCE {}

@[typedef]
struct C.D3D11_RENDER_TARGET_BLEND_DESC {}

@[typedef]
struct C.D3D11_BLEND_DESC {}

@[typedef]
struct C.D3D11_RASTERIZER_DESC {}

@[typedef]
struct C.D3D11_SAMPLER_DESC {}

@[typedef]
struct C.DXGI_SWAP_CHAIN_DESC1 {}

fn check(name string, v_size u32, c_size u32) bool {
	ok := v_size == c_size
	println('${if ok { 'PASS' } else { 'FAIL' }} ${name}: V ${v_size} / C ${c_size}')
	return ok
}

fn main() {
	mut all := true
	all = check('BufferDesc', sizeof(d3d11.BufferDesc), sizeof(C.D3D11_BUFFER_DESC)) && all
	all = check('SubresourceData', sizeof(d3d11.SubresourceData), sizeof(C.D3D11_SUBRESOURCE_DATA))
		&& all
	all = check('Texture2dDesc', sizeof(d3d11.Texture2dDesc), sizeof(C.D3D11_TEXTURE2D_DESC)) && all
	all = check('SrvDesc', sizeof(d3d11.SrvDesc), sizeof(C.D3D11_SHADER_RESOURCE_VIEW_DESC)) && all
	all = check('Viewport', sizeof(d3d11.Viewport), sizeof(C.D3D11_VIEWPORT)) && all
	all =
		check('MappedSubresource', sizeof(d3d11.MappedSubresource), sizeof(C.D3D11_MAPPED_SUBRESOURCE))
		&& all
	all = check('RtBlendDesc', sizeof(d3d11.RtBlendDesc), sizeof(C.D3D11_RENDER_TARGET_BLEND_DESC))
		&& all
	all = check('BlendDesc', sizeof(d3d11.BlendDesc), sizeof(C.D3D11_BLEND_DESC)) && all
	all = check('RasterizerDesc', sizeof(d3d11.RasterizerDesc), sizeof(C.D3D11_RASTERIZER_DESC))
		&& all
	all = check('SamplerDesc', sizeof(d3d11.SamplerDesc), sizeof(C.D3D11_SAMPLER_DESC)) && all
	all = check('SwapChainDesc1', sizeof(d3d11.SwapChainDesc1), sizeof(C.DXGI_SWAP_CHAIN_DESC1))
		&& all
	exit(if all { 0 } else { 1 })
}
