//! The system's image codecs (the Windows Imaging Component) as Quadra's decoder: every
//! format the machine has a codec for, and no decoder of untrusted files of our own.

use quadra_platform::{Decoded, ImageDecoder, ImageError, ImageInfo};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatJpeg, GUID_WICPixelFormat32bppPBGRA,
    GUID_WICPixelFormat32bppPRGBA, IWICBitmapFrameDecode, IWICBitmapSource, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::core::{GUID, Interface};

/// The decoder. It holds nothing: the codecs' factory is made by the thread that decodes.
pub struct WicImages;

fn corrupt(e: windows::core::Error) -> ImageError {
    ImageError::Corrupt(e.message())
}

/// The first frame of a file, the factory that made it, and whether the file is a JPEG.
/// What is returned reads from `file` while it lives.
fn open(file: &[u8]) -> Result<(IWICImagingFactory, IWICBitmapFrameDecode, bool), ImageError> {
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
        let jpeg = decoder.GetContainerFormat().is_ok_and(|format| format == GUID_ContainerFormatJpeg);
        let frame = decoder.GetFrame(0).map_err(corrupt)?;
        Ok((factory, frame, jpeg))
    }
}

fn size(frame: &IWICBitmapFrameDecode) -> Result<ImageInfo, ImageError> {
    let (mut width, mut height) = (0, 0);
    // SAFETY: both pointers are to locals that live through the call.
    unsafe { frame.GetSize(&mut width, &mut height) }.map_err(corrupt)?;
    Ok(ImageInfo { width, height })
}

impl ImageDecoder for WicImages {
    fn info(&self, file: &[u8]) -> Result<ImageInfo, ImageError> {
        size(&open(file)?.1)
    }

    fn decode(&self, file: &[u8], width: u32, height: u32) -> Result<Decoded, ImageError> {
        let (factory, frame, jpeg) = open(file)?;
        let natural = size(&frame)?;
        let scaled = |source: &IWICBitmapSource| -> Result<IWICBitmapSource, ImageError> {
            if (natural.width, natural.height) == (width, height) {
                return Ok(source.clone());
            }
            // SAFETY: plain COM calls on objects of this thread.
            unsafe {
                let scaler = factory.CreateBitmapScaler().map_err(corrupt)?;
                scaler.Initialize(source, width, height, WICBitmapInterpolationModeFant).map_err(corrupt)?;
                scaler.cast().map_err(corrupt)
            }
        };
        let converted = |source: &IWICBitmapSource, to: &GUID| -> Result<IWICBitmapSource, ImageError> {
            // SAFETY: as above; the formats are ones every codec converts to.
            unsafe {
                let converter = factory.CreateFormatConverter().map_err(corrupt)?;
                let (dither, palette) = (WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom);
                converter.Initialize(source, to, dither, None, 0.0, palette).map_err(corrupt)?;
                converter.cast().map_err(corrupt)
            }
        };
        let mut source: IWICBitmapSource = frame.cast().map_err(corrupt)?;
        // A JPEG is scaled as it is: its codec then decodes at a fraction of the size, which
        // is most of the time a photograph takes. Anything else may have alpha, and is
        // premultiplied before it is scaled, or what is transparent would shade its edges.
        // The scaler gives the order of channels it likes, so the order asked for comes last.
        if !jpeg {
            source = converted(&source, &GUID_WICPixelFormat32bppPBGRA)?;
        }
        let source = converted(&scaled(&source)?, &GUID_WICPixelFormat32bppPRGBA)?;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        // SAFETY: the buffer holds `height` rows of `width * 4` bytes, the stride given.
        unsafe { source.CopyPixels(std::ptr::null(), width * 4, &mut pixels) }.map_err(corrupt)?;
        Ok(Decoded { width, height, pixels })
    }
}
