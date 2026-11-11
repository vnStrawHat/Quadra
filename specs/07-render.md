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

A layer's space starts at its root's border box. So a layer that moves keeps every instance:
its place is part of its transform, and only that changes. A scroll container's content is a
node of its own, a layer cut to the container's padding box; the scroll offset is added to
its place.

A node becomes a layer root when it is: a window root or overlay root; scroll content; a
virtual list; a surface widget; a node with a transform or an opacity that is animating or not
the identity; a node marked `.layer()`. A node stops being one when the reason ends. Promotion
and demotion repaint that subtree once.

**A scroll container's content is painted only near what shows.** The layer keeps a band, in
its own space: the part of it the viewport shows, with the viewport's size added on every
side. A node outside the band emits nothing, and a text skips its lines outside it. Scrolling
inside the band is still one constant (and one more for the thumb of the scrollbar). When the part that shows leaves the band, the layer
gets a new band around it and is painted again: once per viewport scrolled, for three
viewports of content, whatever the length of the page. So a long page costs what its visible
part costs, when it is first shown and whenever all of it changes (a new width). A virtual
list does not need this (its rows outside the window do not exist) and does not use it.

Limits and choices:

- Translations are rounded to whole device pixels. Under a rotation or a scale, glyphs are drawn
  from the tiles of their layout size and sampled; text is crisp again when the transform ends.
  Zoom is a style change (font size), not a transform.
- Opacity multiplies each instance's alpha; overlapping children of a faded layer show through
  each other. Compositing a group through a texture is on the After-v1 list (behaviour/13).
- A debug build reports a window with more than 256 layers.

## Display lists

A layer's display list is one vector of instances in paint order. Where a child layer is drawn,
the vector holds a **marker**: an entry of kind "layer" that names the child and is never drawn
itself. The instances between two markers are a run, drawn in one call. Because markers live in
the vector, copying a clean subtree carries its child layers along; the list of breaks is found
again by one scan when the vector is rebuilt, and never changes when instances are overwritten
in place. An image too large for the atlas will be a marker of its own kind (with images).

Each node stores its range **relative to its parent's range**, covering what it painted before
its children, its children, and what it painted after them.

Repainting a layer after a change:

1. Walk down only through nodes flagged for paint or having flagged descendants.
2. A flagged node's widget paints into a scratch buffer.
3. **Same instance count as before**: the new instances overwrite the old ones in place. Only
   that range is uploaded. Hover, press, focus, a caret, a selection colour take this path.
   A node whose style has state variants emits its box even when there is nothing of it to
   see (no fill, no border that shows): the count stays the same when a state fills it, so a
   ghost control under the pointer takes this path too. A border with no colour is no border.
4. **Different count**: the layer's vector is rebuilt by copying. Clean subtrees are copied as
   whole ranges without being visited, and their relative ranges stay valid. Only nodes on the
   dirty paths get new offsets. The layer is uploaded.

So CPU work follows the dirty paths, and an upload is bounded by one layer.

Two things force more than the flagged node:

- **A node that moved paints its whole subtree again.** Instances are in the layer's
  coordinates, so everything under a node that changed place has new positions. The counts
  are usually unchanged, so this is still an overwrite in place. It is the paint side of the
  ceiling layout has: a row that grows in a list that is not virtualized repaints the rows
  after it.
- **Showing or hiding a node, or changing a list of children, rebuilds the layer**, because
  ranges move.

The tree can paint everything from scratch and compare it with what the incremental passes
left (`Tree::verify_paint`): instances and every node's range. The test suite runs it after
every frame.

A surface widget owns its layer's vector. It writes ranges directly and reports them; the
renderer uploads what was reported. A terminal updates the rows that changed.

## Drawing a frame

```
for each root layer of the window, bottom to top:
    draw(layer):
        set the layer's constants and scissor
        for each run and marker, in order: a run is one instanced draw; a marker is draw(child)
```

A layer's transform, scissor and opacity are given in its parent layer's space. The scene works
out what each layer has on screen (transforms composed, scissors intersected, opacities
multiplied) and hands backends a flat list of steps, so no backend walks the layers itself.

Draw calls are about the number of visible layers plus the runs split by child layers. A layer
whose scissor leaves nothing on screen is skipped with everything inside it.

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
- Images above the tile limit get their own texture and a marker of their own kind.
- An atlas is keyed by whoever uses it (a glyph cache by face, glyph, size and phase) and a
  tile carries what its owner knows about it (a glyph's offsets). The packing and the
  upload queue do not depend on either: that part is what a renderer is given
  (`Renderer::sync_atlas`).

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

The idea of the V design, because it was right: shaders are precompiled and embedded; the
hardware device is created on a worker thread while the first frame is drawn on the software
device and shown through a plain blit; when the hardware device is ready the renderer moves to
it and every layer is uploaded once. A hardware driver takes over a hundred milliseconds to
start; the software device takes ten.

One thread makes both devices, the software one first (a driver that starts holds up whatever
loads a library meanwhile). The renderer is there at once: the UI thread opens its window and
builds its view while the software device is made, and the first call that needs a device
waits for it.

The move is not a mechanism of its own. A renderer that has a better device ready says its
device is lost: the app then does what it does for a lost device (new device, tiles cleared,
everything painted and uploaded again), and `recover` takes the device that is waiting instead
of making one. Until then a window's surface is a texture copied to the window; with the
hardware device it gets its swap chain, once (a window whose swap chain is replaced keeps
presenting late). The worker wakes the loop when the device is ready, and the renderer then
wants a frame (`Renderer::wants_frame`), so an app that draws nothing more still moves.

## The `Renderer` trait

```rust
pub trait Renderer {
    fn create_surface(&mut self, target: SurfaceTarget, size: DeviceSize) -> Result<SurfaceId>;
    fn resize_surface(&mut self, s: SurfaceId, size: DeviceSize) -> Result<()>;
    fn drop_surface(&mut self, s: SurfaceId);
    fn draw(&mut self, s: SurfaceId, scene: &mut Scene, clear: Color) -> Result<()>;
    fn present(&mut self, s: SurfaceId) -> Result<()>;
    fn read_pixels(&mut self, s: SurfaceId) -> Result<Pixels>;     // offscreen surfaces: snapshots
    fn forget_scene(&mut self, scene: SceneId);                    // a window closed
    fn recover(&mut self) -> Result<()>;                           // after DeviceLost
    fn stats(&self) -> RenderStats;
}
```

A window's layers live in its `Scene`. `draw` pulls from it: for each layer on screen the
backend uploads what the layer says changed (a range, or all of it), takes note of layers the
scene dropped, and draws the steps. Nobody else tells the backend what to upload, so the dirty
bookkeeping is in one place. `SurfaceTarget` (a window of some windowing system, or offscreen)
is defined here, not by the platform crate: the two do not know each other (02), and the native
crate converts.

Implementations: D3D11 (`quadra-native-win`) and null (in `quadra-render`, used by
`quadra-testing`). The null renderer keeps a copy of what it was told to upload, so a test can
check that copy against the scene, read layers and instances, and count draws.

## What this replaces

| V design | Here |
|---|---|
| One scene per window, emitted again by every paint | Display lists per layer, changed where dirty |
| Scroll offset inside every bound | A layer translation |
| Overlap reordering to merge batches across atlas pages | Texture arrays; paint order is draw order |
| Paint patches as a special case | In-place replacement as the normal case |
| Glyph tiles never freed; images in the glyph atlas up to a page | Counted tiles with a budget; large images as textures |
| GPU errors printed | `Result`, recovery from device loss |
