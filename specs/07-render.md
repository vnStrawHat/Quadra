# 07 — Rendering

The renderer draws instanced primitives kept in layers. The tree repaints only dirty nodes; the
GPU draws every visible layer each frame, which is cheap, from buffers that mostly did not
change.

## Instances

One instance format and one pipeline draw everything: filled and bordered rounded rectangles,
gradients, shadows, lines, glyphs and small images.

| Field | Meaning |
|---|---|
| `bounds` | Rectangle in the layer's device pixels |
| `clip`, clip radius | Rounded clip rectangle, in the same space |
| `color`, `color2` | Fill or tint; border, gradient end or shadow colour |
| `radii`, `border` | Corner radii and border widths |
| `uv`, `slice` | Atlas rectangle and the slice of the atlas texture array |
| `params`, `kind`, `flags` | Per-kind parameters |

There is no per-instance transform: a transform belongs to a layer. The exact packing is fixed
in phase R1 by measuring; it starts from the V layout without its transform index.

Colours are straight alpha in the instance and blended premultiplied, in the encoded (sRGB)
space, as design tools and the V renderer do. The V pixel snapshots are the reference.

## Layers

A layer is a subtree with its own display list and constants:

| Constant | Changed by |
|---|---|
| Transform (2×3 affine, device pixels) | Scroll offset, a transform style, an anchored position |
| Scissor rectangle (in the parent layer's space) | The scroll viewport, an overlay's bounds |
| Opacity | An opacity style |

Changing a constant is the compose pass: one small write, no instance touched.

A node becomes a layer root when it is: a window root or overlay root; scroll content; a
virtual list; a surface widget; a node with a transform or an opacity that is animating or not
the identity; a node marked `.layer()`. A node stops being one when the reason ends. Promotion
and demotion repaint that subtree once.

Limits and choices:

- Translations are rounded to whole device pixels. Under a rotation or a scale, glyphs are drawn
  from the tiles of their layout size and sampled; text is crisp again when the transform ends.
  Zoom is a style change (font size), not a transform.
- Opacity multiplies each instance's alpha; overlapping children of a faded layer show through
  each other. Compositing a group through a texture is on the After-v1 list (behaviour/13).
- A debug build reports a window with more than 256 layers.

## Display lists

A layer's display list is a vector of instances plus a list of items in paint order:

```
Run(range of instances)      drawn in one call
Child(layer)                 the child layer is drawn here, then this layer continues
Image(texture, quad)         an image too large for the atlas
```

Each node stores its range **relative to its parent's range**, covering what it painted before
its children, its children, and what it painted after them.

Repainting a layer after a change:

1. Walk down only through nodes flagged for paint or having flagged descendants.
2. A flagged node's widget paints into a scratch buffer.
3. **Same instance count as before**: the new instances overwrite the old ones in place. Only
   that range is uploaded. Hover, press, focus, a caret, a selection colour take this path.
4. **Different count**: the layer's vector is rebuilt by copying. Clean subtrees are copied as
   whole ranges without being visited, and their relative ranges stay valid. Only nodes on the
   dirty paths get new offsets. The layer is uploaded.

So CPU work follows the dirty paths, and an upload is bounded by one layer.

A surface widget owns its layer's vector. It writes ranges directly and reports them; the
renderer uploads what was reported. A terminal updates the rows that changed.

## Drawing a frame

```
for each root layer of the window, bottom to top:
    draw(layer):
        set the layer's constants and scissor
        for each item: Run -> one instanced draw; Child -> draw(child); Image -> one draw
```

Draw calls are about the number of visible layers plus the runs split by child layers. Layers
entirely outside their scissor are skipped.

The frame is presented with a flip-model swap chain and a frame-latency waitable. Dirty
rectangles at present are not used in the first release.

## Atlases

- Two atlases: coverage (8-bit, glyphs and monochrome icons) and colour (RGBA, colour glyphs,
  small images, rasterized vectors).
- Each is one **texture array**. All slices have the same size; the array starts with one slice
  and grows by creating a larger array and copying. An instance names its slice, so no draw is
  split by a page (01, R5).
- Tiles are packed in shelves by height class.
- **A tile is counted.** Whoever paints with it holds a handle; when the last handle drops, the
  tile moves to a least-recently-used list and stays usable. Space is taken from that list when
  a shelf is needed. A tile in use is never evicted, so a retained display list never points at
  a reused rectangle.
- Each atlas has a byte budget. Beyond it, least-recently-used free tiles are dropped first;
  if everything is in use the atlas grows past the budget and reports it.
- Images above the tile limit get their own texture and an `Image` item.

## Resources and failure

Every GPU resource can be rebuilt from CPU state:

| Resource | Rebuilt from |
|---|---|
| Layer buffers | The layers' instance vectors |
| Atlas contents | Rasterizing again on demand; the atlas epoch changes and text nodes repaint |
| Image textures | The decoded-image cache, or decoding again |
| Pipelines, shaders | Embedded bytecode |

- `Renderer` methods return `Result`. `DeviceLost` makes the app create a new device, bump the
  atlas epoch, flag every layer for upload and draw again. Other errors go to the error hook
  and the log; the window keeps its last frame.
- A window that cannot get a surface at all is closed and the error reported.
- The null renderer can be told to lose its device; a test covers the recovery path.

## Start-up

Unchanged from the V design, because it was right: shaders are precompiled and embedded; the
hardware device is created on a worker thread while the first frame is drawn on the software
device and shown through a plain blit; when the hardware device is ready the renderer moves to
it and every layer is uploaded once.

## The `Renderer` trait

```rust
pub trait Renderer {
    fn create_surface(&mut self, window: WindowHandle, size: DeviceSize) -> Result<SurfaceId>;
    fn resize_surface(&mut self, s: SurfaceId, size: DeviceSize) -> Result<()>;
    fn set_layer(&mut self, layer: LayerId, constants: &LayerConstants);
    fn upload_layer(&mut self, layer: LayerId, instances: &[Instance], dirty: Range<u32>) -> Result<()>;
    fn drop_layer(&mut self, layer: LayerId);
    fn upload_tile(&mut self, atlas: AtlasKind, slice: u32, rect: TileRect, pixels: &[u8]) -> Result<()>;
    fn draw(&mut self, s: SurfaceId, frame: &Frame) -> Result<()>;
    fn present(&mut self, s: SurfaceId) -> Result<()>;
    fn read_pixels(&mut self, s: SurfaceId) -> Result<Image>;      // offscreen surfaces: snapshots
    fn stats(&self) -> RenderStats;
}
```

Implementations: D3D11 (`quadra-native-win`) and null (`quadra-testing`), which keeps what it
was given so tests can read layers, instances and draw counts.

## What this replaces

| V design | Here |
|---|---|
| One scene per window, emitted again by every paint | Display lists per layer, changed where dirty |
| Scroll offset inside every bound | A layer translation |
| Overlap reordering to merge batches across atlas pages | Texture arrays; paint order is draw order |
| Paint patches as a special case | In-place replacement as the normal case |
| Glyph tiles never freed; images in the glyph atlas up to a page | Counted tiles with a budget; large images as textures |
| GPU errors printed | `Result`, recovery from device loss |
