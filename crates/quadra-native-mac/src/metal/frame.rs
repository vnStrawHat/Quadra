//! Drawing a frame (specs/07, "Drawing a frame"): the scene's steps as one command buffer.

use std::ptr::NonNull;

use objc2_metal::{
    MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLLoadAction, MTLPrimitiveType,
    MTLRenderCommandEncoder, MTLRenderPassDescriptor, MTLScissorRect, MTLStoreAction, MTLViewport,
};
use quadra_prim::{Color, Rect};
use quadra_render::{DeviceSize, RenderError, RenderStats, Scene, Step, SurfaceId, World};

use super::MetalRenderer;
use super::backdrop::Backdrop;
use super::gpu::{Commands, Encoder, Texture, finish, made};

/// `Constants` of the shader: the layer of the run and where the run starts.
#[repr(C)]
struct Constants {
    layer_matrix: [f32; 4],
    layer_translate: [f32; 2],
    viewport: [f32; 2],
    layer_opacity: f32,
    instance_offset: u32,
    layer_turned: u32,
    pad: u32,
    /// Where the layer's backdrop was copied from on the surface, and its texture's size.
    backdrop: [f32; 4],
    /// The mask the layer is cut to: its box on the surface, its tile, its slice, and 1 when
    /// there is one.
    mask_rect: [f32; 4],
    mask_uv: [f32; 4],
    mask: [u32; 4],
}

/// A scissor in whole pixels inside the surface; none when nothing of it is inside.
fn scissor(rect: Rect, size: DeviceSize) -> Option<MTLScissorRect> {
    let clamp = |v: f32, max: u32| v.round().clamp(0.0, max as f32) as usize;
    let (left, top) = (clamp(rect.x, size.width), clamp(rect.y, size.height));
    let (right, bottom) = (clamp(rect.right(), size.width), clamp(rect.bottom(), size.height));
    (right > left && bottom > top).then(|| MTLScissorRect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

impl MetalRenderer {
    pub(super) fn frame(
        &mut self,
        surface: SurfaceId,
        scene: &mut Scene,
        clear: Color,
    ) -> Result<(), RenderError> {
        let (size, target) = {
            let surface = self.surface(surface)?;
            (surface.size, surface.texture.clone())
        };
        for layer in scene.take_dropped() {
            self.layers.remove(scene.id(), layer);
        }
        let mut steps = std::mem::take(&mut self.steps);
        scene.steps(Rect::new(0.0, 0.0, size.width as f32, size.height as f32), &mut steps);
        self.stats = RenderStats { frames: self.stats.frames + 1, ..RenderStats::default() };
        let commands = self.gpu.commands()?;
        // The pass that is open; a step that fails leaves it to be closed here.
        let mut pass = None;
        let drawn = self.run(&commands, &target, size, scene, &steps, clear, &mut pass);
        if let Some(open) = pass.take() {
            open.endEncoding();
        }
        self.steps = steps;
        drawn.and(finish(&commands))
    }

    /// Begins a pass into `target` that clears it, or keeps what is there, with what every
    /// draw uses bound.
    fn begin(
        &self,
        commands: &Commands,
        target: &Texture,
        size: DeviceSize,
        clear: Option<Color>,
    ) -> Result<Encoder, RenderError> {
        let about = MTLRenderPassDescriptor::renderPassDescriptor();
        // SAFETY: a pass has a first colour attachment.
        let color = unsafe { about.colorAttachments().objectAtIndexedSubscript(0) };
        color.setTexture(Some(target));
        color.setStoreAction(MTLStoreAction::Store);
        match clear {
            None => color.setLoadAction(MTLLoadAction::Load),
            Some(c) => {
                let a = f64::from(c.a);
                color.setLoadAction(MTLLoadAction::Clear);
                color.setClearColor(MTLClearColor {
                    red: f64::from(c.r) * a,
                    green: f64::from(c.g) * a,
                    blue: f64::from(c.b) * a,
                    alpha: a,
                });
            }
        }
        let encoder = made(commands.renderCommandEncoderWithDescriptor(&about), "render pass")?;
        encoder.setRenderPipelineState(&self.gpu.pipeline);
        encoder.setViewport(MTLViewport {
            originX: 0.0,
            originY: 0.0,
            width: f64::from(size.width),
            height: f64::from(size.height),
            znear: 0.0,
            zfar: 1.0,
        });
        let coverage = self.coverage.as_ref().map_or(&self.gpu.nothing[0], |it| &it.texture);
        let colors = self.color.as_ref().map_or(&self.gpu.nothing[1], |it| &it.texture);
        let behind = self.backdrop.as_ref().map_or(&self.gpu.nothing[2], |it| &it.texture);
        // SAFETY: the sampler and the textures are alive and of the kinds the shader's slots
        // take; the encoder keeps them for the pass.
        unsafe {
            encoder.setFragmentSamplerState_atIndex(Some(&self.gpu.sampler), 0);
            encoder.setFragmentTexture_atIndex(Some(coverage), 1);
            encoder.setFragmentTexture_atIndex(Some(colors), 2);
            encoder.setFragmentTexture_atIndex(Some(behind), 3);
        }
        Ok(encoder)
    }

    /// Runs the steps. `pass` holds the pass that is open, whatever comes back.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        commands: &Commands,
        target: &Texture,
        size: DeviceSize,
        scene: &mut Scene,
        steps: &[Step],
        clear: Color,
        pass: &mut Option<Encoder>,
    ) -> Result<(), RenderError> {
        *pass = Some(self.begin(commands, target, size, Some(clear))?);
        let viewport = [size.width as f32, size.height as f32];
        // The layer the runs are of, where its backdrop was taken from, and whether any of it
        // is inside its scissor.
        let (mut current, mut behind, mut shown): (Option<World>, _, _) = (None, [0.0; 4], false);
        for step in steps {
            match *step {
                Step::Layer { id, world } => {
                    behind = [0.0; 4];
                    if let Some(backdrop) = world.backdrop {
                        // The target is read: the pass ends, the copy is made, and a pass that
                        // keeps what is drawn so far begins.
                        if let Some(open) = pass.take() {
                            open.endEncoding();
                        }
                        behind =
                            Backdrop::take(&mut self.backdrop, &self.gpu, commands, target, size, backdrop)?;
                        *pass = Some(self.begin(commands, target, size, None)?);
                    }
                    let instances = self.layers.sync(&self.gpu, scene, id, &mut self.stats)?;
                    let Some(encoder) = pass.as_ref() else { continue };
                    let cut = scissor(world.scissor, size);
                    shown = cut.is_some();
                    // SAFETY: the buffer holds the layer's instances, as the shader reads
                    // them at both stages.
                    unsafe {
                        encoder.setVertexBuffer_offset_atIndex(Some(&instances), 0, 0);
                        encoder.setFragmentBuffer_offset_atIndex(Some(&instances), 0, 0);
                    }
                    if let Some(cut) = cut {
                        encoder.setScissorRect(cut);
                    }
                    current = Some(world);
                    self.stats.layers_bound += 1;
                }
                Step::Run { start, end } => {
                    let (Some(world), Some(encoder), true) = (&current, pass.as_ref(), shown) else {
                        continue;
                    };
                    let t = world.transform;
                    let constants = Constants {
                        layer_matrix: [t.a, t.b, t.c, t.d],
                        layer_translate: [t.tx, t.ty],
                        viewport,
                        layer_opacity: world.opacity,
                        instance_offset: start,
                        layer_turned: !t.is_translation() as u32,
                        pad: 0,
                        backdrop: behind,
                        mask_rect: world.mask.map_or([0.0; 4], |(_, r)| [r.x, r.y, r.width, r.height]),
                        mask_uv: world.mask.map_or([0.0; 4], |(m, _)| m.uv),
                        mask: world.mask.map_or([0; 4], |(m, _)| [m.slice, 1, 0, 0]),
                    };
                    let (bytes, length) = (NonNull::from(&constants).cast(), size_of::<Constants>());
                    // SAFETY: the constants are copied by the calls, and are as long as they
                    // are told; the pipeline, the layer's instances and the textures are bound.
                    unsafe {
                        encoder.setVertexBytes_length_atIndex(bytes, length, 1);
                        encoder.setFragmentBytes_length_atIndex(bytes, length, 1);
                        encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                            MTLPrimitiveType::TriangleStrip,
                            0,
                            4,
                            (end - start) as usize,
                        );
                    }
                    self.stats.draw_calls += 1;
                    self.stats.instances_drawn += end - start;
                }
            }
        }
        Ok(())
    }
}
