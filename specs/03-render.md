# 03 — GPU Rendering

Module `quadra.gpu`. Core produces a `Scene` of plain data, the `Renderer` draws it. Nothing above `gpu` issues draw calls.

## Primitives (device px)

| Primitive | Fields | Used for |
|---|---|---|
| `Quad` | bounds, fill (solid or 2-stop gradient), border widths[4], border color, corner radii[4] | backgrounds, borders, rounded boxes |
| `Shadow` | bounds, corner radii, blur, offset, color | drop shadows (analytic Gaussian on a rounded-rect SDF) |
| `MonoSprite` | bounds, atlas tile, color | glyphs, kit icons |
| `PolySprite` | bounds, atlas tile, corner radii, opacity, grayscale | images, SVGs, color emoji, ClearType glyphs (blended per channel, 04-text) |
| `Line` | bounds, color, thickness, mode (solid, wavy, chart column, stroke segment) | underlines, IME preedit, separators, chart lines (one antialiased column per device px, specs/19), strokes |

Every primitive carries a `clip` content mask (rect plus corner radii), applied in the pixel shader. There is no scissor stack, so clips never split batches.

**Rotation.** Primitives stay axis-aligned rects in their own space. A rotated subtree adds one rigid transform (cos, sin, tx, ty in device px) to the scene's `xforms`, a second structured buffer; entry 0 is the identity. An instance names its transform and its clip's transform (`Instance.xform`: low and high 16 bits), so `Instance` stays 144 bytes. The vertex shader rotates the corners; the pixel shader evaluates every SDF in the primitive's own space and the clip in the clip's space (exact for nested rotations). ClearType glyphs under a rotation blend as grayscale. Overlap layers use the rotated bounding box. A clip pushed in a rotated space intersects its parent only when both share the space; otherwise it keeps its own rect (content of a clipped, rotated box inside a scroll view can show past the scroll edge at the rotated corners).

**Strokes** (`Scene.stroke`, `PaintCx.stroke`: canvas lines and stroked paths, the spinner) are `Line` instances in stroke mode, one per segment. The pixel shader takes the distance to the segment (round joins and caps) and leaves a pixel to a neighbouring segment when that one is nearer, so a joint is covered once and translucent strokes stay even. The segments of one stroke never stack in overlap layers. Nothing goes through the atlas, so animated strokes cost no memory. Wavy underlines measure distance across the wave, so they keep their width on the slopes. **Filled paths** from `canvas` are rasterized on the CPU in pure V (coverage, antialiased) and cached in the atlas by geometry and size. No MSAA is needed.

## Chunks: the unit of caching
A `Scene` is built from **chunks**. Each cached view and each scroll container records its primitives into its own chunk, in local coordinates.

```v
pub struct Chunk {
pub mut:
	prims   PrimitiveLists   // per-kind arrays, local coords, reused capacity
	bounds  Rect             // union of all primitive bounds
	version u32              // bumped when the content changes
}
```
Each frame core composes the scene from chunk references plus their window offsets and clips. A clean chunk is copied with an offset add; nothing is re-emitted. When a scroll offset changes, only that chunk's offset changes.

## Draw ordering (overlap layers)
Paint order is tree order, but draw order is computed by overlap:
1. Each primitive takes `layer = 1 + max layer of earlier primitives it overlaps`, or 0 if it overlaps nothing.
2. Overlap queries use a uniform spatial hash over the window (cells of 64 device px), cleared each frame with retained capacity.
3. Primitives are bucketed by `(layer, kind, texture)`. Each bucket becomes one instanced draw call, drawn in layer order.

A list of 1,000 buttons ends up as about 3 layers and a handful of draw calls, instead of one draw call per button part. Overlays start in a layer above all main-tree layers.

## Renderer interface

```v
pub interface Renderer {
mut:
	resize(w int, h int)
	draw(scene &Scene, atlases &AtlasSet) !
	frame_waitable() voidptr          // handed to the platform loop
	create_texture(kind AtlasKind, w int, h int) TextureId
	upload_tile(tex TextureId, r Rect, pixels []u8)
	stats() RenderStats                // draw calls, instances, upload bytes, gpu time
}
pub fn new_renderer(native_handle voidptr, opts RendererOptions) !&Renderer
```

## Atlases
- Pure V. Glyphs use shelf packing per size class. Images use a skyline packer.
- Kinds: `mono` (R8) for glyphs and kit icons, `poly` (RGBA8 premultiplied) for images, SVGs and emoji.
- Textures are 2048² and added on demand. Tiles unused for 600 frames are evicted when a texture is full, then the texture is repacked during idle time.
- New tiles are uploaded in one batch before drawing, using `UpdateSubresource` on dirty rects only.

## Images and SVG
- Decoding goes through platform services: WIC on Windows for PNG, JPEG, GIF, WebP, ICO, BMP. SVG is rendered by Direct2D's SVG document support into a bitmap at the exact device size.
- Decoding and SVG rasterization run on worker threads, each with its own Direct2D context, because Direct2D contexts must not be shared across threads. The element shows a placeholder until the bitmap arrives, then requests a paint.
- Decoded images are cached by `(source, device size)` with a byte budget (default 128 MB) and LRU eviction.

## D3D11 backend
- `D3D11CreateDevice` with HARDWARE, fallback to WARP, and `PREVENT_INTERNAL_THREADING_OPTIMIZATIONS` (no driver worker thread: less process CPU per frame, measured in docs/benchmarks.md).
- WARP-first start-up: a cold hardware driver start takes 100+ ms, WARP about 10 ms. The first frames draw on WARP while one worker thread creates the WARP device, then the hardware one (one after the other: the driver start holds the DLL loader lock). Meanwhile a window has no swap chain: it draws into an offscreen texture that `present` copies to the window with GDI (`SetDIBitsToDevice`, gdi32 loaded at run time; no waitable). A repeating 25 ms internal timer calls `Device.try_upgrade`. Once the hardware device is ready, the windows' offscreen textures move to it, the atlas uploads made so far are replayed into new pages, the WARP device is released, and every window repaints; that draw creates the window's swap chain first thing, and the core sends its waitable to the platform after the frame. Two things measured in the grid scene left about 1% of frames at two intervals for good, so they are avoided: replacing a window's flip swap chain, and creating it between frames (from the timer) instead of right before the frame that presents to it. With no hardware device, or with offscreen surfaces bound to WARP, the swap chain is made on WARP.
- Flip-model swap chain (`FLIP_DISCARD`), BGRA8, `SetMaximumFrameLatency(1)`, and the frame-latency waitable is exposed to the platform loop.
- `Present1` with dirty rects (the union of changed chunk bounds) so DWM recomposes less. GPU redraw is full-frame in v1, because it costs well under a millisecond.
- One vertex shader expands instances from `SV_VertexID`. Per-kind instance data sits in dynamic structured buffers (`MAP_WRITE_DISCARD`), sized by doubling.
- Shaders are precompiled `.cso` files (SM 5.0, so feature level 11_0 is the minimum) built by `tools/build_shaders.sh` and embedded with `$embed_file`. A dev flag `-d quadra_shader_dev` compiles HLSL at runtime with `D3DCompile` for quick iteration.
- Device loss: recreate the device, textures and swap chain, then mark every chunk for upload.

## Tests
`order_test.v` for overlap layers, `atlas_test.v` for packing and eviction, `tessellate_test.v` for path fills and strokes, and a WARP-backed snapshot test in `testing/` (11-devtools-testing).
