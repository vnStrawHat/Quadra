// Reading and installing the theme (specs/15).
module kit

import quadra.core
import quadra.prim
import quadra.text

// theme reads the global theme during render (tracked: the view re-renders on change).
pub fn theme(cx &core.Cx) &Theme {
	return cx.global[Theme]() or { &Theme{
		...dark()
	} }
}

// theme_of reads the theme at layout time; components resolve tokens there, so a theme
// switch only needs a relayout, never a re-render.
pub fn theme_of(app &core.App) &Theme {
	cx := core.Cx{
		app: unsafe { app }
	}
	return cx.peek_global[Theme]() or { &Theme{
		...dark()
	} }
}

// set_theme replaces the theme and relays out every window.
pub fn set_theme(mut app core.App, t Theme) {
	mut cx := core.Cx{
		app: app
	}
	cx.set_global(t)
	app.text_style = text.TextStyle{
		family:      t.font_family
		size:        t.text_md * t.text_scale
		color:       t.colors.foreground
		line_height: t.line_height
	}
	for mut w in app.windows {
		w.clear_color = t.colors.background
		w.focus_color = t.p.border_focused
		w.scroll_color = t.p.scroll_thumb
		w.thumb_style = thumb_style(t)
		w.invalidate()
	}
}

// follow_system picks light, dark or high contrast from the OS and keeps following it.
// Themes with `system_accent` take the accent from the OS too.
pub fn follow_system(mut app core.App, light_theme Theme, dark_theme Theme) {
	apply := fn [light_theme, dark_theme] (mut app core.App) {
		look := app.plat.appearance()
		mut t := if look.high_contrast {
			high_contrast()
		} else if look.dark {
			dark_theme
		} else {
			light_theme
		}
		if !look.high_contrast && t.system_accent {
			t.p.accent = look.accent
			t.p.accent_hover = look.accent.lerp(prim.white, 0.12)
			t.p.accent_press = look.accent.lerp(prim.black, 0.15)
			t.p.accent_tint = look.accent.alpha(0.1)
			t.p.accent_text = look.accent
			t.p.border_focused = look.accent
			t = t.finish()
		}
		t.text_scale = look.text_scale
		set_theme(mut app, t)
	}
	apply(mut app)
	app.appearance_hooks << apply
}
