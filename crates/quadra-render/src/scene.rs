//! A window's layers, and the flat list of steps that draws them (specs/07, "Drawing a
//! frame").

use std::sync::atomic::{AtomicU64, Ordering};

use quadra_prim::{Affine, Rect};

use crate::layer::{Layer, LayerId};

/// Identifies a scene to backends, which keep buffers per scene and layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(u64);

static NEXT_SCENE: AtomicU64 = AtomicU64::new(1);

/// Deeper nesting of layers than this is not drawn: a marker cycle must not hang a frame.
const MAX_DEPTH: usize = 64;

/// What a layer has on screen: its constants composed with those of the layers around it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct World {
    /// From the layer's space to the surface, in device pixels.
    pub transform: Affine,
    /// In surface device pixels.
    pub scissor: Rect,
    pub opacity: f32,
    /// On the first step of a layer that blurs what is behind it: its box on the surface, and
    /// the blur. The backend takes the backdrop then, before the layer's first run.
    pub backdrop: Option<(Rect, f32)>,
}

/// One step of drawing a scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// The instances that follow are of this layer: bind its buffer, set its constants.
    Layer { id: LayerId, world: World },
    /// Draw instances `start..end` of the current layer in one call.
    Run { start: u32, end: u32 },
}

pub struct Scene {
    id: SceneId,
    layers: Vec<Option<Layer>>,
    free: Vec<u32>,
    serial: u64,
    roots: Vec<LayerId>,
    dropped: Vec<LayerId>,
}

impl Default for Scene {
    fn default() -> Scene {
        Scene::new()
    }
}

impl Scene {
    pub fn new() -> Scene {
        let id = SceneId(NEXT_SCENE.fetch_add(1, Ordering::Relaxed));
        Scene { id, layers: Vec::new(), free: Vec::new(), serial: 0, roots: Vec::new(), dropped: Vec::new() }
    }

    pub fn id(&self) -> SceneId {
        self.id
    }

    pub fn add_layer(&mut self) -> LayerId {
        self.serial += 1;
        // Serials are unique across scenes too: the scene id is in the high bits.
        let layer = Some(Layer::new(self.id.0 << 40 | self.serial));
        if let Some(index) = self.free.pop() {
            self.layers[index as usize] = layer;
            return LayerId::from_raw(index);
        }
        self.layers.push(layer);
        LayerId::from_raw(self.layers.len() as u32 - 1)
    }

    /// Removes a layer. A marker that still names it draws nothing.
    pub fn remove_layer(&mut self, id: LayerId) {
        if self.layers.get_mut(id.index()).and_then(Option::take).is_some() {
            self.free.push(id.raw());
            self.dropped.push(id);
            self.roots.retain(|&r| r != id);
        }
    }

    pub fn get(&self, id: LayerId) -> Option<&Layer> {
        self.layers.get(id.index())?.as_ref()
    }

    pub fn layer(&self, id: LayerId) -> &Layer {
        self.get(id).expect("a layer was used after it was removed")
    }

    pub fn layer_mut(&mut self, id: LayerId) -> &mut Layer {
        let layer = self.layers.get_mut(id.index()).and_then(Option::as_mut);
        layer.expect("a layer was used after it was removed")
    }

    /// The number of layers.
    pub fn len(&self) -> usize {
        self.layers.len() - self.free.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The layers drawn directly on the surface, bottom to top: the main tree, then overlays.
    pub fn set_roots(&mut self, roots: &[LayerId]) {
        self.roots.clear();
        self.roots.extend_from_slice(roots);
    }

    pub fn roots(&self) -> &[LayerId] {
        &self.roots
    }

    /// The layers removed since the last call: a backend frees their buffers.
    pub fn take_dropped(&mut self) -> Vec<LayerId> {
        std::mem::take(&mut self.dropped)
    }

    /// Makes backends upload every layer again (a new device).
    pub fn invalidate_all(&mut self) {
        self.layers.iter_mut().flatten().for_each(Layer::invalidate);
    }

    /// Fills `out` with the steps that draw the scene on a surface of the given bounds.
    pub fn steps(&self, surface: Rect, out: &mut Vec<Step>) {
        out.clear();
        let world = World { transform: Affine::IDENTITY, scissor: surface, opacity: 1.0, backdrop: None };
        for &root in &self.roots {
            self.visit(root, &world, out, 0);
        }
    }

    fn visit(&self, id: LayerId, parent: &World, out: &mut Vec<Step>, depth: usize) {
        let Some(layer) = self.get(id) else { return };
        let c = layer.constants;
        let scissor = match c.scissor {
            Some(rect) => parent.scissor.intersect(parent.transform.bbox(rect)),
            None => parent.scissor,
        };
        if scissor.is_empty() || depth > MAX_DEPTH {
            return;
        }
        let transform = parent.transform.then(c.transform);
        let backdrop = c.backdrop.map(|b| {
            (transform.bbox(Rect::new(0.0, 0.0, b.size.width, b.size.height)).intersect(scissor), b.blur)
        });
        let world = World { transform, scissor, opacity: parent.opacity * c.opacity, backdrop: None };
        let (mut bound, mut first) = (false, true);
        let mut run = |out: &mut Vec<Step>, bound: &mut bool, start: u32, end: u32| {
            if end > start {
                if !*bound {
                    let backdrop = if std::mem::take(&mut first) { backdrop } else { None };
                    out.push(Step::Layer { id, world: World { backdrop, ..world } });
                    *bound = true;
                }
                out.push(Step::Run { start, end });
            }
        };
        let mut at = 0;
        for &(index, child) in layer.breaks() {
            run(out, &mut bound, at, index);
            let before = out.len();
            self.visit(child, &world, out, depth + 1);
            // The child drew: this layer must be bound again before its next run.
            bound &= out.len() == before;
            at = index + 1;
        }
        run(out, &mut bound, at, layer.instances().len() as u32);
    }
}
