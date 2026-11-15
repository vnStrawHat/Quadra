//! Fonts loaded from memory (bundled with an app): an in-memory loader feeds a font set
//! builder, whose set becomes a collection that is searched before the installed fonts.

use std::ffi::c_void;

use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_UNICODE_RANGE, IDWriteFactory5, IDWriteFontCollection, IDWriteFontFallback, IDWriteFontFile,
    IDWriteFontSetBuilder1, IDWriteInMemoryFontFileLoader,
};
use windows::core::{Error, HSTRING, Interface, PCWSTR, Result};

use super::{DirectWrite, family_name};

pub(super) struct Loaded {
    factory: IDWriteFactory5,
    loader: IDWriteInMemoryFontFileLoader,
    /// Every file loaded so far.
    builder: IDWriteFontSetBuilder1,
    pub collection: IDWriteFontCollection,
}

impl DirectWrite {
    fn collection_of(&self, builder: &IDWriteFontSetBuilder1) -> Result<IDWriteFontCollection> {
        // SAFETY: plain calls on live objects.
        unsafe { self.factory.CreateFontCollectionFromFontSet(&builder.CreateFontSet()?)?.cast() }
    }

    /// The family name of a font file, from a collection of that file alone.
    fn name_of(&self, file: &IDWriteFontFile) -> Result<String> {
        // SAFETY: plain calls on live objects; family 0 exists when the count is not 0.
        unsafe {
            let alone = self.factory.CreateFontSetBuilder()?;
            alone.AddFontFile(file)?;
            let collection = self.collection_of(&alone)?;
            let name = if collection.GetFontFamilyCount() == 0 {
                None
            } else {
                family_name(&collection.GetFontFamily(0)?)
            };
            name.ok_or_else(|| Error::new(E_INVALIDARG, "the font has no family name"))
        }
    }

    /// Adds TrueType or OpenType fonts from memory and returns the family name of each. The
    /// bytes are copied. The searched collection is rebuilt once per call, so a family's
    /// files loaded together cost one rebuild.
    pub(super) fn load(&mut self, files: &[&[u8]]) -> Result<Vec<String>> {
        // SAFETY: plain calls on live objects; the loader copies the bytes it is given
        // during the call, since no owner object is passed to keep them alive.
        unsafe {
            let (loader, builder) = match &self.loaded {
                Some(loaded) => (loaded.loader.clone(), loaded.builder.clone()),
                None => {
                    let loader = self.factory.CreateInMemoryFontFileLoader()?;
                    self.factory.RegisterFontFileLoader(&loader)?;
                    (loader, self.factory.CreateFontSetBuilder()?)
                }
            };
            let mut names = Vec::with_capacity(files.len());
            for bytes in files {
                if bytes.is_empty() {
                    return Err(Error::new(E_INVALIDARG, "empty font data"));
                }
                let data = bytes.as_ptr().cast::<c_void>();
                let file =
                    loader.CreateInMemoryFontFileReference(&self.factory, data, bytes.len() as u32, None)?;
                names.push(self.name_of(&file)?);
                builder.AddFontFile(&file)?;
            }
            let collection = self.collection_of(&builder)?;
            self.loaded = Some(Loaded { factory: self.factory.clone(), loader, builder, collection });
            Ok(names)
        }
    }
}

impl DirectWrite {
    /// The system's fallback after the families named, tried in order for every character.
    pub(super) fn fallback_with(&self, families: &[String]) -> Result<IDWriteFontFallback> {
        // SAFETY: plain calls on live objects; the names outlive the call that reads them.
        unsafe {
            let system = self.factory2.GetSystemFontFallback()?;
            if families.is_empty() {
                return Ok(system);
            }
            let builder = self.factory2.CreateFontFallbackBuilder()?;
            let names: Vec<HSTRING> = families.iter().map(HSTRING::from).collect();
            let names: Vec<*const u16> = names.iter().map(|it| it.as_ptr()).collect();
            let all = [DWRITE_UNICODE_RANGE { first: 0, last: 0x10FFFF }];
            builder.AddMapping(&all, &names, &self.system, PCWSTR::null(), PCWSTR::null(), 1.0)?;
            builder.AddMappings(&system)?;
            builder.CreateFontFallback()
        }
    }
}

impl Drop for Loaded {
    fn drop(&mut self) {
        // The factory is shared by the process: a loader registered with it is taken back.
        // SAFETY: the loader was registered by `load`; the factory is the one it came from.
        let _ = unsafe { self.factory.UnregisterFontFileLoader(&self.loader) };
    }
}
