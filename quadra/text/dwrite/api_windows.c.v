// DirectWrite types, constants and vtable slots. dwrite*.h are C++ only, so
// layouts are transcribed from the SDK headers; slots come from tools/com_slots.sh.
module dwrite

import quadra.platform.win

pub struct FontMetrics {
pub:
	design_units_per_em     u16
	ascent                  u16
	descent                 u16
	line_gap                i16
	cap_height              u16
	x_height                u16
	underline_position      i16
	underline_thickness     u16
	strikethrough_position  i16
	strikethrough_thickness u16
}

pub struct ScriptAnalysis {
pub:
	script u16
	shapes u32 // DWRITE_SCRIPT_SHAPES; 1 = no visual (control characters)
}

pub struct GlyphOffset {
pub:
	advance_offset  f32
	ascender_offset f32
}

struct GlyphRun {
	face        voidptr
	em_size     f32
	count       u32
	indices     &u16
	advances    &f32
	offsets     &GlyphOffset
	is_sideways int
	bidi_level  u32
}

pub const font_weight_regular = 400
pub const font_weight_semibold = 600
pub const font_weight_bold = 700
pub const font_style_normal = 0
pub const font_style_italic = 2
const font_stretch_normal = 5

pub const texture_aliased_1x1 = 0
pub const texture_cleartype_3x1 = 1
const rendering_natural_symmetric = 5
const measuring_natural = 0
const grid_fit_default = 0
const antialias_cleartype = 0
const antialias_grayscale = 1
const e_not_sufficient_buffer = int(u32(0x8007007A))

const iid_factory2 = win.guid('0439fc60-ca44-4994-8dee-3a9af7b732ec')
const iid_analysis_source = win.guid('688e1a58-5094-47c8-adc8-fbcea60ae92b')
const iid_analysis_sink = win.guid('5810cd44-0ca0-4701-b3fa-bec5182ae4f6')

// IDWriteFactory2
const factory_get_system_font_collection = 3
const factory_create_text_analyzer = 21
const factory2_create_glyph_run_analysis = 30
// IDWriteFontCollection, IDWriteFontFamily, IDWriteFont, IDWriteFontFace
const collection_get_font_family = 4
const collection_find_family_name = 5
const family_get_first_matching_font = 7
const font_create_font_face = 13
const face_get_metrics = 8
// IDWriteTextAnalyzer
const analyzer_analyze_script = 3
const analyzer_get_glyphs = 7
const analyzer_get_glyph_placements = 8
// IDWriteGlyphRunAnalysis
const run_analysis_get_alpha_texture_bounds = 3
const run_analysis_create_alpha_texture = 4
