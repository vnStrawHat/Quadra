//! The system's image codecs (the Windows Imaging Component) as Quadra's decoder: every
//! format the machine has a codec for, and no decoder of untrusted files of our own.

mod moving;

use std::time::Duration;

use quadra_platform::{Decoded, ImageDecoder, ImageError, ImageInfo, quarter};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatGif, GUID_ContainerFormatJpeg,
    GUID_WICPixelFormat32bppPBGRA, GUID_WICPixelFormat32bppPRGBA, IWICBitmapDecoder, IWICBitmapFrameDecode,
    IWICBitmapSource, IWICImagingFactory, IWICMetadataQueryReader, WICBitmapDitherTypeNone,
    WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantClear};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::Variant::{VT_UI1, VT_UI2, VT_UI4};
use windows::core::{GUID, Interface, PCWSTR, w};

/// The decoder. It holds nothing: the codecs' factory is made by the thread that decodes.
pub struct WicImages;

fn corrupt(e: windows::core::Error) -> ImageError {
    ImageError::Corrupt(e.message())
}

/// A file opened: the factory, the decoder of its container and its first frame. What is
/// returned reads from `file` while it lives.
struct Opened {
    factory: IWICImagingFactory,
    decoder: IWICBitmapDecoder,
    frame: IWICBitmapFrameDecode,
    format: GUID,
}

fn open(file: &[u8]) -> Result<Opened, ImageError> {
    // SAFETY: COM is started on this thread if it was not (a thread of the pool never ends
    // it, a thread that chose another mode keeps it: either way the factory can be made).
    // The stream reads `file`, which outlives every object made here: they are dropped
    // before the caller returns.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).map_err(corrupt)?;
        let stream = factory.CreateStream().map_err(corrupt)?;
        stream.InitializeFromMemory(file).map_err(corrupt)?;
        let decoder = factory
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
            .map_err(corrupt)?;
        let format = decoder.GetContainerFormat().unwrap_or_default();
        let frame = decoder.GetFrame(0).map_err(corrupt)?;
        Ok(Opened { factory, decoder, frame, format })
    }
}

/// The size of the pixels as they are stored.
fn size(frame: &IWICBitmapFrameDecode) -> Result<ImageInfo, ImageError> {
    let (mut width, mut height) = (0, 0);
    // SAFETY: both pointers are to locals that live through the call.
    unsafe { frame.GetSize(&mut width, &mut height) }.map_err(corrupt)?;
    Ok(ImageInfo { width, height })
}

/// A number of the metadata, by its name, when it has one.
fn number(reader: &IWICMetadataQueryReader, name: PCWSTR) -> Option<u32> {
    // SAFETY: the value is a local that lives through the call and is cleared after it; its
    // union is read as the type its tag says it holds.
    unsafe {
        let mut value = PROPVARIANT::default();
        reader.GetMetadataByName(name, &mut value).ok()?;
        let inner = &value.Anonymous.Anonymous;
        let seen = match inner.vt {
            VT_UI1 => Some(inner.Anonymous.bVal as u32),
            VT_UI2 => Some(inner.Anonymous.uiVal as u32),
            VT_UI4 => Some(inner.Anonymous.ulVal),
            _ => None,
        };
        let _ = PropVariantClear(&mut value);
        seen
    }
}

/// The camera's orientation tag of a frame (EXIF 274, by the name the codecs share for it),
/// 1 when it has none.
fn orientation(frame: &IWICBitmapFrameDecode) -> u16 {
    // SAFETY: a plain COM call on an object of this thread.
    let Ok(reader) = (unsafe { frame.GetMetadataQueryReader() }) else { return 1 };
    number(&reader, w!("System.Photo.Orientation")).map_or(1, |o| o as u16)
}

/// `source` at another size.
fn scaled(
    factory: &IWICImagingFactory,
    source: &IWICBitmapSource,
    width: u32,
    height: u32,
) -> Result<IWICBitmapSource, ImageError> {
    // SAFETY: plain COM calls on objects of this thread.
    unsafe {
        let scaler = factory.CreateBitmapScaler().map_err(corrupt)?;
        scaler.Initialize(source, width, height, WICBitmapInterpolationModeFant).map_err(corrupt)?;
        scaler.cast().map_err(corrupt)
    }
}

/// `source` in another format: ones every codec converts to.
fn converted(
    factory: &IWICImagingFactory,
    source: &IWICBitmapSource,
    to: &GUID,
) -> Result<IWICBitmapSource, ImageError> {
    // SAFETY: as above.
    unsafe {
        let converter = factory.CreateFormatConverter().map_err(corrupt)?;
        let (dither, palette) = (WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom);
        converter.Initialize(source, to, dither, None, 0.0, palette).map_err(corrupt)?;
        converter.cast().map_err(corrupt)
    }
}

/// The pixels of a source of 32 bits a pixel.
fn pixels(source: &IWICBitmapSource, width: u32, height: u32) -> Result<Vec<u8>, ImageError> {
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    // SAFETY: the buffer holds `height` rows of `width * 4` bytes, the stride given.
    unsafe { source.CopyPixels(std::ptr::null(), width * 4, &mut pixels) }.map_err(corrupt)?;
    Ok(pixels)
}

impl ImageDecoder for WicImages {
    /// The size as the image is seen: a photograph turned a quarter by its tag swaps sides.
    fn info(&self, file: &[u8]) -> Result<ImageInfo, ImageError> {
        let opened = open(file)?;
        if opened.format == GUID_ContainerFormatGif
            && let Some(screen) = moving::screen(&opened.decoder)
        {
            return Ok(screen);
        }
        let stored = size(&opened.frame)?;
        Ok(match quarter(orientation(&opened.frame)) {
            true => ImageInfo { width: stored.height, height: stored.width },
            false => stored,
        })
    }

    fn decode(&self, file: &[u8], width: u32, height: u32) -> Result<Decoded, ImageError> {
        let opened = open(file)?;
        // A GIF's first frame may cover a part of its screen only.
        if opened.format == GUID_ContainerFormatGif
            && let Some(mut first) = moving::gif(&opened, width, height, 1)?
            && !first.is_empty()
        {
            return Ok(first.swap_remove(0).0);
        }
        let Opened { factory, frame, format, .. } = opened;
        let turn = orientation(&frame);
        // What is asked for is the size seen: the stored pixels are scaled to it unturned.
        let (width, height) = if quarter(turn) { (height, width) } else { (width, height) };
        let natural = size(&frame)?;
        let mut source: IWICBitmapSource = frame.cast().map_err(corrupt)?;
        // A JPEG is scaled as it is: its codec then decodes at a fraction of the size, which
        // is most of the time a photograph takes. Anything else may have alpha, and is
        // premultiplied before it is scaled, or what is transparent would shade its edges.
        // The scaler gives the order of channels it likes, so the order asked for comes last.
        if format != GUID_ContainerFormatJpeg {
            source = converted(&factory, &source, &GUID_WICPixelFormat32bppPBGRA)?;
        }
        if (natural.width, natural.height) != (width, height) {
            source = scaled(&factory, &source, width, height)?;
        }
        let source = converted(&factory, &source, &GUID_WICPixelFormat32bppPRGBA)?;
        Ok(Decoded { width, height, pixels: pixels(&source, width, height)? }.oriented(turn))
    }

    fn frames(&self, file: &[u8], width: u32, height: u32) -> Result<Vec<(Decoded, Duration)>, ImageError> {
        let opened = open(file)?;
        if opened.format == GUID_ContainerFormatGif
            && let Some(frames) = moving::gif(&opened, width, height, u32::MAX)?
        {
            return Ok(frames);
        }
        Ok(vec![(self.decode(file, width, height)?, Duration::ZERO)])
    }
}
