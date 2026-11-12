//! The image view (specs/05; specs/11, "Images"): a file's picture, read and decoded on the
//! background pool at the size it is shown at, and painted when it arrives.

mod cache;

use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Poll, Waker};

use quadra_async::{background, spawn};
use quadra_layout::Length;
use quadra_platform::{ImageError, ImageInfo, MAX_IMAGE_FILE_BYTES, admit};
use quadra_prim::Size;
use quadra_reactive::use_context;
use quadra_tree::{Fit, Image, NodeId, Picture, Role, Style};

pub use cache::Images;
use cache::SourceKey;

use crate::view::{MountCx, View};

/// Where an image's file is.
#[derive(Clone)]
pub enum ImageSource {
    /// A file on disk.
    Path(PathBuf),
    /// A file built into the program (`include_bytes!`).
    Static(&'static [u8]),
    /// A file in memory. Its pictures are not shared with other views of the same bytes.
    Bytes(Arc<[u8]>),
}

impl From<PathBuf> for ImageSource {
    fn from(path: PathBuf) -> ImageSource {
        ImageSource::Path(path)
    }
}

impl From<&str> for ImageSource {
    fn from(path: &str) -> ImageSource {
        ImageSource::Path(path.into())
    }
}

impl From<&'static [u8]> for ImageSource {
    fn from(file: &'static [u8]) -> ImageSource {
        ImageSource::Static(file)
    }
}

impl From<Vec<u8>> for ImageSource {
    fn from(file: Vec<u8>) -> ImageSource {
        ImageSource::Bytes(file.into())
    }
}

impl ImageSource {
    fn key(&self) -> Option<SourceKey> {
        match self {
            ImageSource::Path(path) => Some(SourceKey::Path(path.clone())),
            ImageSource::Static(file) => Some(SourceKey::Static(file.as_ptr() as usize, file.len())),
            ImageSource::Bytes(_) => None,
        }
    }
}

type OnLoad = Box<dyn FnOnce(Result<Size, ImageError>)>;

pub struct ImageView {
    source: ImageSource,
    fit: Fit,
    radius: f32,
    layout: quadra_layout::Style,
    label: String,
    on_load: Option<OnLoad>,
}

/// An image: as large as its file says, no wider than there is room, until a size is given.
/// Nothing shows while it loads, or when it fails.
pub fn image(source: impl Into<ImageSource>) -> ImageView {
    let (source, layout) = (source.into(), quadra_layout::Style { shrink: 0.0, ..Default::default() });
    ImageView { source, fit: Fit::Contain, radius: 0.0, layout, label: String::new(), on_load: None }
}

impl ImageView {
    /// How it fills a box of another shape than its own.
    pub fn fit(mut self, fit: Fit) -> Self {
        self.fit = fit;
        self
    }

    /// Rounds its corners.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// A width in pixels; the height follows the image's shape unless it is given too.
    pub fn width(mut self, px: f32) -> Self {
        self.layout.width = Length::Px(px);
        self
    }

    pub fn height(mut self, px: f32) -> Self {
        self.layout.height = Length::Px(px);
        self
    }

    /// Takes the room its parent has along the main axis.
    pub fn grow(mut self) -> Self {
        self.layout.grow = 1.0;
        self.layout.shrink = 1.0;
        self
    }

    /// What the image shows, in words, for a reader.
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = text.into();
        self
    }

    /// Told once how loading went: the image's size, or why there is none.
    pub fn on_load(mut self, f: impl FnOnce(Result<Size, ImageError>) + 'static) -> Self {
        self.on_load = Some(Box::new(f));
        self
    }
}

/// The size a picture is shown at, for the task that brings its pixels.
#[derive(Default)]
struct Shown {
    size: Cell<(u32, u32)>,
    waker: RefCell<Option<Waker>>,
}

impl Shown {
    async fn other_than(&self, have: (u32, u32)) -> (u32, u32) {
        poll_fn(|cx| {
            let size = self.size.get();
            if size != have && size.0 > 0 && size.1 > 0 {
                return Poll::Ready(size);
            }
            self.waker.replace(Some(cx.waker().clone()));
            Poll::Pending
        })
        .await
    }
}

/// Reads a file, checked against the limits by its length before and by its header after.
async fn read(images: &Images, source: &ImageSource) -> Result<(Arc<[u8]>, ImageInfo), ImageError> {
    let (source, decoder) = (source.clone(), images.decoder.clone());
    let work = background(move || {
        let file: Arc<[u8]> = match source {
            ImageSource::Path(path) => {
                let length = std::fs::metadata(&path).map_err(|e| ImageError::Read(e.to_string()))?.len();
                if length > MAX_IMAGE_FILE_BYTES as u64 {
                    return Err(ImageError::TooLarge(format!("a file of {length} bytes")));
                }
                std::fs::read(&path).map_err(|e| ImageError::Read(e.to_string()))?.into()
            }
            ImageSource::Static(file) => file.into(),
            ImageSource::Bytes(file) => file,
        };
        let info = decoder.info(&file)?;
        admit(info, file.len())?;
        Ok((file, info))
    });
    work.await.unwrap_or_else(|panic| Err(ImageError::Corrupt(panic.0)))
}

impl View for ImageView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let shown = Rc::new(Shown::default());
        let tell = shown.clone();
        let mut picture = Picture::new(self.fit, self.radius, move |width, height| {
            if tell.size.replace((width, height)) != (width, height)
                && let Some(waker) = tell.waker.take()
            {
                waker.wake();
            }
        });
        // Without an app or a harness around there is nobody to decode: the box stays empty.
        let images = use_context::<Rc<Images>>();
        let key = self.source.key();
        // What is known of the file already shows in the first frame: a row that scrolls
        // back into a list does not blink.
        let known = images.as_ref().zip(key.as_ref()).and_then(|(images, key)| images.known(key));
        if let Some((info, image)) = &known {
            picture.set_natural(Size::new(info.width as f32, info.height as f32));
            if let Some(image) = image {
                picture.set_image(image.clone());
            }
        }
        let id = cx.tree.insert_widget(Style { layout: self.layout, ..Style::default() }, Box::new(picture));
        let label = self.label;
        cx.tree.update_semantics(id, |says| (says.role, says.name) = (Role::Image, label));
        let Some(images) = images else { return id };

        let (handle, source, on_load) = (cx.handle.clone(), self.source, self.on_load);
        let mut info = known.map(|(info, _)| info);
        spawn(async move {
            let mut file: Option<Arc<[u8]>> = None;
            if info.is_none() {
                match read(&images, &source).await {
                    Ok((bytes, read)) => {
                        (file, info) = (Some(bytes), Some(read));
                        if let Some(key) = &key {
                            images.remember(key, read);
                        }
                        let natural = Size::new(read.width as f32, read.height as f32);
                        handle.with(move |tree| {
                            if tree.widget_mut::<Picture>(id).is_some_and(|p| p.set_natural(natural)) {
                                tree.mark_layout(id);
                            }
                        });
                    }
                    Err(error) => return on_load.map_or((), |f| f(Err(error))),
                }
            }
            if let (Some(f), Some(info)) = (on_load, info) {
                f(Ok(Size::new(info.width as f32, info.height as f32)));
            }
            let mut have = (0, 0);
            loop {
                // Each size it is shown at: pixels of that size, from the cache or the file.
                let want = shown.other_than(have).await;
                have = want;
                let mut image = key.as_ref().and_then(|key| images.get(key, want));
                if image.is_none() {
                    if file.is_none() {
                        file = read(&images, &source).await.ok().map(|(bytes, _)| bytes);
                    }
                    let Some(file) = file.clone() else { continue };
                    let decoder = images.decoder.clone();
                    let decoded = background(move || decoder.decode(&file, want.0, want.1)).await;
                    image = decoded.ok().and_then(Result::ok).map(Image::new);
                    if let (Some(key), Some(decoded)) = (&key, image.take()) {
                        image = Some(images.put(key, want, decoded));
                    }
                }
                let Some(image) = image else { continue };
                handle.with(move |tree| {
                    if let Some(picture) = tree.widget_mut::<Picture>(id) {
                        picture.set_image(image);
                        tree.mark_paint(id);
                    }
                });
            }
        })
        .detach();
        id
    }
}
