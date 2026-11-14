//! Layers: a display list with its own transform, scissor and opacity (specs/07, "Layers",
//! "Display lists").

use std::ops::Range;

use quadra_prim::{Affine, Rect, Size};

use crate::instance::Instance;

/// A layer of a [`crate::Scene`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayerId(u32);

impl LayerId {
    pub(crate) fn from_raw(raw: u32) -> LayerId {
        LayerId(raw)
    }

    pub(crate) fn raw(self) -> u32 {
        self.0
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// What a layer applies to everything in it, given in its parent layer's space. Changing them
/// touches no instance: scrolling and transform or opacity animation cost one small write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerConstants {
    pub transform: Affine,
    /// What the layer is cut to, or `None`.
    pub scissor: Option<Rect>,
    pub opacity: f32,
    /// What is behind the layer's box is blurred under it (specs/07, "Instances").
    pub backdrop: Option<Backdrop>,
    /// What the layer and the layers in it are cut to (specs/07, "Layers").
    pub mask: Option<Mask>,
}

/// A shape a layer is cut to: a tile of the coverage atlas (`uv`, `slice`) stretched over a
/// box from the layer's origin, in device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mask {
    pub uv: [f32; 4],
    pub slice: u32,
    pub size: Size,
}

/// A blur of what is behind a layer: its box, from the layer's origin, and the blur, in
/// device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Backdrop {
    pub size: Size,
    pub blur: f32,
}

impl Default for LayerConstants {
    fn default() -> LayerConstants {
        LayerConstants {
            transform: Affine::IDENTITY,
            scissor: None,
            opacity: 1.0,
            backdrop: None,
            mask: None,
        }
    }
}

/// What a backend must upload of a layer's instances.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dirty {
    #[default]
    Clean,
    /// Instances overwritten in place.
    Range { start: u32, end: u32 },
    /// The list was rebuilt.
    All,
}

pub struct Layer {
    serial: u64,
    pub constants: LayerConstants,
    instances: Vec<Instance>,
    /// The other buffer: a rebuild writes into it and the two are swapped, so rebuilding a
    /// layer of steady size allocates nothing.
    spare: Vec<Instance>,
    /// Where child layers are drawn: the index of each marker.
    breaks: Vec<(u32, LayerId)>,
    dirty: Dirty,
}

impl Layer {
    pub(crate) fn new(serial: u64) -> Layer {
        Layer {
            serial,
            constants: LayerConstants::default(),
            instances: Vec::new(),
            spare: Vec::new(),
            breaks: Vec::new(),
            dirty: Dirty::All,
        }
    }

    /// Unique among all layers ever made: a backend tells a new layer from an old one that
    /// had the same id.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// The markers: (index in the instances, child layer), in order.
    pub fn breaks(&self) -> &[(u32, LayerId)] {
        &self.breaks
    }

    pub fn dirty(&self) -> Dirty {
        self.dirty
    }

    /// What must be uploaded; the layer is clean afterwards. For backends.
    pub fn take_dirty(&mut self) -> Dirty {
        std::mem::take(&mut self.dirty)
    }

    /// Makes the backend upload everything again (a new device).
    pub fn invalidate(&mut self) {
        self.dirty = Dirty::All;
    }

    fn find_breaks(&mut self) {
        self.breaks.clear();
        let markers =
            self.instances.iter().enumerate().filter_map(|(i, x)| Some((i as u32, x.child_layer()?)));
        self.breaks.extend(markers);
    }

    /// Overwrites instances in place: the same count as before, so nothing else moves and
    /// only this range is uploaded.
    pub fn patch(&mut self, at: usize, new: &[Instance]) {
        let old = &mut self.instances[at..at + new.len()];
        let markers = old.iter().chain(new).any(|i| i.child_layer().is_some());
        old.copy_from_slice(new);
        if markers {
            self.find_breaks();
        }
        let (start, end) = (at as u32, (at + new.len()) as u32);
        self.dirty = match self.dirty {
            Dirty::Clean => Dirty::Range { start, end },
            Dirty::Range { start: s, end: e } => Dirty::Range { start: s.min(start), end: e.max(end) },
            Dirty::All => Dirty::All,
        };
    }

    /// Replaces the whole list: for a surface widget that owns its layer.
    pub fn set(&mut self, instances: &[Instance]) {
        self.instances.clear();
        self.instances.extend_from_slice(instances);
        self.find_breaks();
        self.dirty = Dirty::All;
    }

    /// Starts building the next list from the current one.
    pub fn rebuild(&mut self) -> Rebuild<'_> {
        self.spare.clear();
        Rebuild { layer: self }
    }
}

/// The next display list of a layer, built by copying what did not change from the current
/// one and appending what did. Nothing changes until [`Rebuild::finish`].
pub struct Rebuild<'a> {
    layer: &'a mut Layer,
}

impl Rebuild<'_> {
    /// The length so far: where the next instances will start.
    pub fn len(&self) -> usize {
        self.layer.spare.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layer.spare.is_empty()
    }

    /// Copies a range of the current list (a clean subtree) and returns where it starts now.
    pub fn copy(&mut self, old: Range<usize>) -> usize {
        let Layer { instances, spare, .. } = &mut *self.layer;
        let at = spare.len();
        spare.extend_from_slice(&instances[old]);
        at
    }

    /// Appends new instances and returns where they start.
    pub fn push(&mut self, new: &[Instance]) -> usize {
        let at = self.layer.spare.len();
        self.layer.spare.extend_from_slice(new);
        at
    }

    /// Appends the marker of a child layer and returns its index.
    pub fn child(&mut self, layer: LayerId) -> usize {
        self.push(&[Instance::marker(layer)])
    }

    /// Makes what was built the layer's list.
    pub fn finish(self) {
        let layer = self.layer;
        std::mem::swap(&mut layer.instances, &mut layer.spare);
        layer.find_breaks();
        layer.dirty = Dirty::All;
    }
}
