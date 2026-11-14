//! The images of an app (specs/11, "Images"): the decoder, what is known of each file, and
//! the pictures decoded from them, so that views of one file at one size share its pixels
//! and its tiles, and a view that comes back finds them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use quadra_platform::{ImageDecoder, ImageInfo};
use quadra_tree::Image;

/// What pictures nobody shows may take before the oldest are forgotten.
const BUDGET: usize = 32 << 20;

/// What a file is known by: its path, or where in the program it is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SourceKey {
    Path(PathBuf),
    Static(usize, usize),
}

struct Entry {
    image: Rc<Image>,
    used: u64,
}

/// An app has one, provided as context beside its timers and its tasks; the test harness
/// provides one that reads flat test images.
pub struct Images {
    pub(crate) decoder: Arc<dyn ImageDecoder>,
    infos: RefCell<HashMap<SourceKey, ImageInfo>>,
    pictures: RefCell<HashMap<(SourceKey, u32, u32), Entry>>,
    clock: Cell<u64>,
}

impl Images {
    pub fn new(decoder: Arc<dyn ImageDecoder>) -> Rc<Images> {
        let (infos, pictures) = (RefCell::default(), RefCell::default());
        Rc::new(Images { decoder, infos, pictures, clock: Cell::new(0) })
    }

    /// Forgets what is known of a file: it changed on disk. Views that show it keep what
    /// they show; the next one reads it again.
    pub fn forget(&self, path: &Path) {
        let key = SourceKey::Path(path.to_path_buf());
        self.infos.borrow_mut().remove(&key);
        let dropped: Vec<Entry> = {
            let mut pictures = self.pictures.borrow_mut();
            let keys: Vec<_> = pictures.keys().filter(|k| k.0 == key).cloned().collect();
            keys.iter().filter_map(|k| pictures.remove(k)).collect()
        };
        drop(dropped); // outside the borrow
    }

    /// How many decoded pictures are held, and their bytes.
    pub fn held(&self) -> (usize, usize) {
        let pictures = self.pictures.borrow();
        (pictures.len(), pictures.values().map(|e| e.image.bytes()).sum())
    }

    fn tick(&self) -> u64 {
        self.clock.replace(self.clock.get() + 1)
    }

    pub(crate) fn remember(&self, key: &SourceKey, info: ImageInfo) {
        self.infos.borrow_mut().insert(key.clone(), info);
    }

    /// The size of a file read before, and the picture of it used last, of whatever size.
    pub(crate) fn known(&self, key: &SourceKey) -> Option<(ImageInfo, Option<Rc<Image>>)> {
        let info = *self.infos.borrow().get(key)?;
        let pictures = self.pictures.borrow();
        let latest = pictures.iter().filter(|(k, _)| k.0 == *key).max_by_key(|(_, e)| e.used);
        Some((info, latest.map(|(_, e)| e.image.clone())))
    }

    /// The picture of a file at a size; with `pixels`, only while it has them: one that let
    /// go of them is painted from the atlas, and is no use to a view whose atlas lost it.
    pub(crate) fn get(&self, key: &SourceKey, size: (u32, u32), pixels: bool) -> Option<Rc<Image>> {
        let used = self.tick();
        let mut pictures = self.pictures.borrow_mut();
        let entry =
            pictures.get_mut(&(key.clone(), size.0, size.1)).filter(|e| !pixels || e.image.has_pixels())?;
        entry.used = used;
        Some(entry.image.clone())
    }

    /// Keeps a picture, and gives the one to show: another view may have brought the same
    /// while this one was decoded, and then both show that one.
    pub(crate) fn put(&self, key: &SourceKey, size: (u32, u32), image: Rc<Image>) -> Rc<Image> {
        if let Some(first) = self.get(key, size, true) {
            return first;
        }
        let used = self.tick();
        let dropped: Vec<Entry> = {
            let mut pictures = self.pictures.borrow_mut();
            pictures.insert((key.clone(), size.0, size.1), Entry { image: image.clone(), used });
            // What only the cache holds counts against the budget, the oldest first out.
            let mut idle: Vec<_> = pictures
                .iter()
                .filter(|(_, e)| Rc::strong_count(&e.image) == 1)
                .map(|(k, e)| (e.used, e.image.bytes(), k.clone()))
                .collect();
            idle.sort_unstable_by_key(|(used, ..)| std::cmp::Reverse(*used));
            let mut total = 0;
            let over = idle.into_iter().filter(|(_, bytes, _)| {
                total += bytes;
                total > BUDGET
            });
            over.filter_map(|(.., k)| pictures.remove(&k)).collect()
        };
        drop(dropped);
        image
    }
}
