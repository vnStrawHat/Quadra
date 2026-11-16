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
| `color`, `color2` | Fill or tint; border or shadow colour |
| `radii`, `border` | Corner radii and border widths |
| `uv`, `slice` | Atlas rectangle and the slice of the atlas texture array (a gradient's ramp) |
| `params`, `kind`, `flags` | Per-kind parameters |

There is no per-instance transform: a transform belongs to a layer. The exact packing is fixed
in phase R1 by measuring; it starts from the V layout without its transform index.

A line is an underline (straight or wavy), a segment of a stroke, or **a column of a chart's
line**: an instance one device pixel wide that says how tall the line is in that column and
how far it falls across it. The shader works out from the two how much of each pixel of the
column the line covers, exactly (the cover of a slanted band over a column is a trapezoid
along it), so a line of any slope is as smooth as its pixels allow, and nothing is
rasterized on the CPU. What emits a column keeps the line's place a fraction of a pixel:
its `bounds` are whole pixels across and not down. A place rounded to pixels, or a column
that is not one pixel on the grid, is what draws a slope as steps. A test of pixels holds
it: each column has all of the line's ink, its middle where the line's is to a twentieth
of a pixel, and shades at its ends.

**A stroke** is a line through points, an instance a segment:
`quadra_render::stroke(points, width, color, closed, clip, push)`. The instance holds the
ends of its segment and of the segments before and after it, and the shader shades a pixel
by how far it is from the segment: a stroke of any slope and any length is as wide as it
says, has round ends and round joints, and takes no room in an atlas. A pixel at a joint is
drawn by the nearer of the two segments, once, so a stroke that is not opaque is even there
too. A test of pixels holds its width, its ink and its joints. The links of a diagram and
the strokes of a formula are drawn so (13).

Colours are straight alpha in the instance and blended premultiplied, in the encoded (sRGB)
space, as design tools do. Quadra's own pixel snapshots are the reference (12, "Snapshots").

**A gradient** fills a box with colours at stops along it (`quadra_prim::Gradient`): along a
line at an angle (`linear`, 0 degrees from left to right, 90 from top to bottom), out from the
middle to the corners (`radial`, an ellipse of the box's shape), or round the middle
(`conic`, from an angle, 0 at the top, clockwise). Between two stops the colours are mixed in
Oklab with alpha premultiplied, as design tools mix them: red to blue goes through a violet,
not the dark grey of a mix of encoded values. The mix is not worked out per pixel: it is
baked once into a **ramp**, a row of 256 texels premultiplied in the colour atlas, keyed by
the stops, so one ramp serves every box with those stops and goes when none uses it. The
quad's instance names the ramp (`uv`: the row, inset half a texel; `slice`), and `params`
say the kind (`y`: 1 linear, 2 radial, 3 conic) and the angle (`x`). The shader works out
how far along the gradient a pixel is and samples the ramp there; a box's border and radii
apply as to any fill, in the same instance. A quad that is not a gradient does no more work
than before. A node's gradient is set by `Tree::set_gradient` (`.gradient(..)` on a box);
it is not a state variant.

**A box can blur what is behind it** (`.backdrop_blur(px)`, `Tree::set_backdrop`): frosted
glass under a panel. Such a node is a layer root, so the renderer knows where it starts: its
layer's constants say the blur and the size of its box, and before the first run of that
layer each frame the backend copies what is drawn so far under the box, padded by twice the
blur and aligned to the blur's blocks on the surface, into a texture with mips, and makes
the mips. The box's own quad (flag 16, the blur in `params.z`) samples that texture at the
pixel's place, at the mip level of the blur, and lays its own fill over it: a translucent
background tints the glass. This keeps the one pipeline: the blur is the mip chain, made by
the GPU in one call, and no pass of a shader of its own. It is drawn again each frame that
is drawn: a pane of 600 by 400 with a blur of 16 over a window of 1200 by 800 costs the GPU
25 µs a frame (docs/benchmarks.md), so keeping it while nothing behind changed waits for a
measure that says otherwise. Under a gradient the blur shows through its clear parts
too. Blocks of the blur stay put on the surface, so content that scrolls under the glass
moves under a still pattern. (A blur of passes, smoother than mips, when the look asks for
it.)

**A border is solid, dashed or dotted** (`BorderStyle`, `.border_style(..)` on a box, a
paint value as its colour is). A pixel of the border goes by the side it is nearest to, as
much of that side's width as it is away; along the middle of the border a dash is three
widths long with a gap of three, a dot a disc as wide as the border with a gap of one width.
A side's pattern runs from the middle of one corner's arc to the middle of the next, round
the arcs too, stretched a little so that a whole number of periods fits, with a half dash or
a half dot at each end: two sides meet in one mark at a corner. A gap shows the box's fill.
The quad's flags say which (4 dashed, 8 dotted); a solid border does no more work.

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
the identity; a node that blurs what is behind it; a node cut to a shape; a node marked
`.layer()`.

**A layer can be cut to any shape** (`.clip_shape(svg)`, `Tree::set_clip_shape`): a star, a
squircle, the letters of a word. The shape, a drawing as an icon is one (fitted to the box
with its aspect kept, centred), is rasterized as coverage at the box's size in device pixels
into the coverage atlas, a tile the node holds (no larger than a slice: a larger box has its
mask at a lower resolution, sampled softer). The layer's constants name the tile, and the
step that binds the layer gives the mask's place on the surface; every pixel the layer draws,
and every pixel of the layers inside it that have no mask of their own, is multiplied by the
mask there. So a mask follows the layer as it moves, and costs nothing to the instances: they
are not touched, and a box that is not cut does no more work. A mask follows a translation
(and a scale) of its layer; under a rotation it is the box around the turned one. A mask in
a mask is the inner one alone. A node stops being one when the reason ends. Promotion
and demotion repaint that subtree once.

**A scroll container's content is painted only near what shows.** The layer keeps a band, in
its own space: the part of it the viewport shows, with the viewport's size added on every
side. A node whose box is outside the band emits nothing, nor does anything it holds, and a
text skips its lines outside it. What reaches past its holder's box by more than a viewport
is so not painted while the holder is outside the band. A new band skips the subtree of a
node outside it that emitted nothing in the last one and has nothing to paint: of a long
page only the nodes near the old and the new band are walked, and the rows of a long column
are looked at once each. A node keeps whether its box was outside the band when last looked
at: when layout alone changes that (a box grown into the band), what it holds is walked again. Scrolling inside the band is still one constant (and one more for
the thumb of the scrollbar).
A layer is also turned about its middle by a constant (`Tree::set_layer_turn`): a spinner. When the part that shows leaves the band, the layer
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
in place. An image larger than a tile is several tiles (below), so the list needs no marker
of another kind.

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

**What a frame costs the GPU is measured** before anything is made faster for it, when it is
asked to (`Renderer::measure_gpu(true)`; off by default, since the queries cost the CPU some
15 to 30 µs a frame): the D3D11 renderer puts two timestamps around each frame's draw (in a disjoint query, which says the
clock's rate and whether it held steady) and reads them back without waiting, a few frames
later, from a ring of four. `RenderStats::gpu` is the latest frame's time that came back:
none until one does, and none on a backend that cannot measure. Present is not in it. The
bench turns it on with `BENCH_GPU=1` and prints it with the passes (`gpu` in
`tools/bench.sh`'s line, the mean of the frames that had one); the runner's numbers are taken
without it.

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
- An image is cut into pieces of 512 pixels at most, each a tile of the colour atlas
  (`Fonts::image`): one of any size shows, with no texture and no draw call of its own, and
  the pieces that do not show take no room. Pieces are drawn on whole device pixels; one
  that is scaled samples half a texel inside its tile. The colour atlas tries to stay within
  64 MiB.
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
| Image tiles | The decoded pixels each picture keeps |
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
| Glyph tiles never freed; images in the glyph atlas up to a page | Counted tiles with a budget; an image of any size as pieces |
| GPU errors printed | `Result`, recovery from device loss |
