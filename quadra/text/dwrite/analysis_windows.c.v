// Script analysis. DirectWrite pulls text through an
// IDWriteTextAnalysisSource and pushes results into an IDWriteTextAnalysisSink;
// both COM objects are implemented here in V.
module dwrite

import quadra.platform.win

pub struct ScriptRun {
pub:
	start    u32 // UTF-16 offset
	len      u32
	analysis ScriptAnalysis
}

// TextAnalysis holds the script runs of one paragraph (shaping needs nothing else; line
// breaking and bidi are done in quadra.text).
pub struct TextAnalysis {
pub mut:
	scripts []ScriptRun
}

struct AnalysisSource {
	win.ComObject // must stay first: COM sees the vtable pointer at offset 0
	text   &u16
	len    u32
	locale &u16
}

struct AnalysisSink {
	win.ComObject
mut:
	result TextAnalysis
}

fn source_vtable() &voidptr {
	return win.new_vtable([voidptr(src_text_at), voidptr(src_text_before),
		voidptr(src_reading_direction), voidptr(src_locale), voidptr(src_number_substitution)])
}

fn sink_vtable() &voidptr {
	return win.new_vtable([voidptr(sink_script), voidptr(sink_breakpoints), voidptr(sink_bidi),
		voidptr(sink_number_substitution)])
}

fn src_text_at(this voidptr, pos u32, text &&u16, len &u32) int {
	s := unsafe { &AnalysisSource(this) }
	unsafe {
		if pos >= s.len {
			*text = nil
			*len = 0
		} else {
			*text = s.text + pos
			*len = s.len - pos
		}
	}
	return 0
}

fn src_text_before(this voidptr, pos u32, text &&u16, len &u32) int {
	s := unsafe { &AnalysisSource(this) }
	unsafe {
		if pos == 0 || pos > s.len {
			*text = nil
			*len = 0
		} else {
			*text = s.text
			*len = pos
		}
	}
	return 0
}

fn src_reading_direction(_ voidptr) int {
	return 0 // left to right paragraphs in v1
}

fn src_locale(this voidptr, pos u32, len &u32, locale &&u16) int {
	s := unsafe { &AnalysisSource(this) }
	unsafe {
		*len = if pos < s.len { s.len - pos } else { 0 }
		*locale = s.locale
	}
	return 0
}

fn src_number_substitution(this voidptr, pos u32, len &u32, sub &voidptr) int {
	s := unsafe { &AnalysisSource(this) }
	unsafe {
		*len = if pos < s.len { s.len - pos } else { 0 }
		*sub = nil
	}
	return 0
}

fn sink_script(this voidptr, pos u32, len u32, sa &ScriptAnalysis) int {
	mut s := unsafe { &AnalysisSink(this) }
	s.result.scripts << ScriptRun{pos, len, *sa}
	return 0
}

fn sink_breakpoints(_ voidptr, _ u32, _ u32, _ &u8) int {
	return 0
}

fn sink_bidi(_ voidptr, _ u32, _ u32, _ u8, _ u8) int {
	return 0
}

fn sink_number_substitution(_ voidptr, _ u32, _ u32, _ voidptr) int {
	return 0
}

type FnAnalyze = fn (this voidptr, source voidptr, pos u32, len u32, sink voidptr) int

// analyze itemizes UTF-16 text by script.
pub fn (f &Factory) analyze(text []u16, locale &u16) !TextAnalysis {
	if text.len == 0 {
		return TextAnalysis{}
	}
	src := &AnalysisSource{
		ComObject: win.ComObject{
			vtbl: f.src_vtbl
			iid:  iid_analysis_source
		}
		text:      unsafe { &u16(text.data) }
		len:       u32(text.len)
		locale:    unsafe { locale }
	}
	mut sink := &AnalysisSink{
		ComObject: win.ComObject{
			vtbl: f.sink_vtbl
			iid:  iid_analysis_sink
		}
	}
	run := unsafe { FnAnalyze(win.slot(f.analyzer, analyzer_analyze_script)) }
	win.check(run(f.analyzer, voidptr(src), 0, u32(text.len), voidptr(sink)), 'AnalyzeScript')!
	return sink.result
}
