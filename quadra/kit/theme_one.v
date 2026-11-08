// The kit's themes (specs/15): One Dark Pro and One Light, bundled as theme files
// (themes/, theme_json.v), and high contrast.
module kit

import quadra.prim

const dark_json = $embed_file('themes/one-dark-pro.json')
const light_json = $embed_file('themes/one-light.json')

// dark is One Dark Pro.
pub fn dark() Theme {
	return built_in(dark_json.to_string())
}

// light is One Light.
pub fn light() Theme {
	return built_in(light_json.to_string())
}

// high_contrast follows the spirit of Windows high-contrast themes: pure black and white,
// yellow focus and selection, no subtle greys.
pub fn high_contrast() Theme {
	mut t := dark()
	t.name = 'High contrast'
	t.mode = .high_contrast
	t.p = Palette{
		...t.p
		canvas:         prim.black
		panel:          prim.black
		chrome:         prim.black
		panel_hover:    prim.rgb(0x1A1A1A)
		raised:         prim.black
		raised_hover:   prim.rgb(0x1A1A1A)
		press:          prim.rgb(0x333333)
		overlay_bg:     prim.black
		line:           prim.white
		line_strong:    prim.white
		border_focused: prim.rgb(0xFFFF00)
		text:           prim.white
		text_2:         prim.white
		text_3:         prim.white
		text_disabled:  prim.rgb(0x9A9A9A)
		text_on_accent: prim.black
		accent:         prim.rgb(0xFFFF00)
		accent_hover:   prim.rgb(0xFFFF66)
		accent_press:   prim.rgb(0xCCCC00)
		accent_tint:    prim.rgba(0xFFFF0055)
		accent_text:    prim.rgb(0xFFFF00)
	}
	return t.finish()
}
