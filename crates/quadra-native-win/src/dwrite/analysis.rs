//! Script analysis and font fallback. DirectWrite pulls the text through a source object and
//! pushes its results into a sink; one object here is both.

use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_LINE_BREAKPOINT, DWRITE_READING_DIRECTION,
    DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, DWRITE_SCRIPT_ANALYSIS, IDWriteNumberSubstitution,
    IDWriteTextAnalysisSink, IDWriteTextAnalysisSink_Impl, IDWriteTextAnalysisSource,
    IDWriteTextAnalysisSource_Impl,
};
use windows::core::{HSTRING, Interface, OutRef, Ref, Result, implement};

use super::{DirectWrite, family_name, style_of};

/// A range of UTF-16 units in one script.
#[derive(Clone, Copy)]
pub(super) struct ScriptRun {
    pub start: u32,
    pub len: u32,
    pub analysis: DWRITE_SCRIPT_ANALYSIS,
}

/// A range of UTF-16 units and the family that draws it; `None` is the family asked for.
pub(super) struct FontRun {
    pub start: u32,
    pub len: u32,
    pub family: Option<String>,
}

/// The text DirectWrite reads, and where it writes the script runs. It owns copies: a COM
/// object may not borrow.
#[implement(IDWriteTextAnalysisSource, IDWriteTextAnalysisSink)]
struct Analysis {
    text: Vec<u16>,
    locale: HSTRING,
    runs: Rc<RefCell<Vec<ScriptRun>>>,
}

impl Analysis_Impl {
    fn rest(&self, position: u32) -> u32 {
        (self.text.len() as u32).saturating_sub(position)
    }
}

impl IDWriteTextAnalysisSource_Impl for Analysis_Impl {
    fn GetTextAtPosition(&self, position: u32, text: *mut *mut u16, len: *mut u32) -> Result<()> {
        let rest = self.rest(position);
        // SAFETY: the caller's out pointers are valid; the text is offered only inside the
        // buffer, which lives as long as this object.
        unsafe {
            *text = if rest == 0 {
                std::ptr::null_mut()
            } else {
                self.text.as_ptr().add(position as usize) as *mut u16
            };
            *len = rest;
        }
        Ok(())
    }

    fn GetTextBeforePosition(&self, position: u32, text: *mut *mut u16, len: *mut u32) -> Result<()> {
        let inside = position > 0 && position as usize <= self.text.len();
        // SAFETY: as above.
        unsafe {
            *text = if inside { self.text.as_ptr() as *mut u16 } else { std::ptr::null_mut() };
            *len = if inside { position } else { 0 };
        }
        Ok(())
    }

    fn GetParagraphReadingDirection(&self) -> DWRITE_READING_DIRECTION {
        DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
    }

    fn GetLocaleName(&self, position: u32, len: *mut u32, locale: *mut *mut u16) -> Result<()> {
        // SAFETY: the caller's out pointers are valid; the locale lives as long as this object.
        unsafe {
            *len = self.rest(position);
            *locale = self.locale.as_ptr() as *mut u16;
        }
        Ok(())
    }

    fn GetNumberSubstitution(
        &self,
        position: u32,
        len: *mut u32,
        substitution: OutRef<IDWriteNumberSubstitution>,
    ) -> Result<()> {
        // SAFETY: the caller's out pointer is valid.
        unsafe { *len = self.rest(position) };
        substitution.write(None)
    }
}

impl IDWriteTextAnalysisSink_Impl for Analysis_Impl {
    fn SetScriptAnalysis(&self, start: u32, len: u32, analysis: *const DWRITE_SCRIPT_ANALYSIS) -> Result<()> {
        // SAFETY: the caller's pointer is valid for the call.
        let analysis = unsafe { *analysis };
        self.runs.borrow_mut().push(ScriptRun { start, len, analysis });
        Ok(())
    }

    fn SetLineBreakpoints(&self, _: u32, _: u32, _: *const DWRITE_LINE_BREAKPOINT) -> Result<()> {
        Ok(())
    }

    fn SetBidiLevel(&self, _: u32, _: u32, _: u8, _: u8) -> Result<()> {
        Ok(())
    }

    fn SetNumberSubstitution(&self, _: u32, _: u32, _: Ref<IDWriteNumberSubstitution>) -> Result<()> {
        Ok(())
    }
}

impl DirectWrite {
    fn source(&self, text: &[u16]) -> (IDWriteTextAnalysisSource, Rc<RefCell<Vec<ScriptRun>>>) {
        let runs: Rc<RefCell<Vec<ScriptRun>>> = Rc::default();
        let analysis = Analysis { text: text.to_vec(), locale: self.locale.clone(), runs: runs.clone() };
        (analysis.into(), runs)
    }

    /// The script runs of a text.
    pub(super) fn analyze(&self, text: &[u16]) -> Vec<ScriptRun> {
        let (source, runs) = self.source(text);
        let Ok(sink) = source.cast::<IDWriteTextAnalysisSink>() else { return Vec::new() };
        // SAFETY: the source and the sink are alive for the call and answer inside the text.
        let done = unsafe { self.analyzer.AnalyzeScript(&source, 0, text.len() as u32, &sink) };
        if done.is_err() {
            return Vec::new();
        }
        runs.take()
    }

    /// Covers `text[start..start + len]` with installed fonts, starting from a family: the
    /// ranges the family lacks get the font the system falls back to.
    pub(super) fn map_fonts(
        &self,
        text: &[u16],
        start: u32,
        len: u32,
        key: &quadra_text::FontKey,
    ) -> Vec<FontRun> {
        let whole = || vec![FontRun { start, len, family: None }];
        let (source, _) = self.source(text);
        let base = HSTRING::from(&*key.family);
        let (mut out, mut at, end) = (Vec::<FontRun>::new(), start, start + len);
        while at < end {
            let (mut mapped, mut font, mut scale) = (0u32, None, 1.0f32);
            // SAFETY: the out pointers are valid, and the range is inside the text.
            let done = unsafe {
                self.fallback.MapCharacters(
                    &source,
                    at,
                    end - at,
                    &self.system,
                    &base,
                    DWRITE_FONT_WEIGHT(key.weight as i32),
                    style_of(key.italic),
                    DWRITE_FONT_STRETCH_NORMAL,
                    &mut mapped,
                    &mut font,
                    &mut scale,
                )
            };
            if done.is_err() {
                return whole();
            }
            let mapped = mapped.max(1);
            // SAFETY: the font is alive.
            let family = font.and_then(|f| unsafe { f.GetFontFamily() }.ok()).and_then(|f| family_name(&f));
            let family = family.filter(|name| **name != *key.family);
            match out.last_mut() {
                Some(last) if last.family == family => last.len += mapped,
                _ => out.push(FontRun { start: at, len: mapped, family }),
            }
            at += mapped;
        }
        out
    }
}
