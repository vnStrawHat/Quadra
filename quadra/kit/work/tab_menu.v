// A tab strip's right click and double click, for apps with tab menus and renaming.
module work

import quadra.core
import quadra.kit

// TabPoint is a right click on a tab, for the app's context menu.
pub struct TabPoint {
pub:
	id string
	x  f32
	y  f32
}

pub fn (self_ &TabStrip) on_menu(l core.Listener[TabPoint]) &TabStrip {
	mut s := unsafe { self_ }
	s.menu = l.any
	return s
}

// list_heading titles the `...` list and adds Close for the active tab below it.
pub fn (self_ &TabStrip) list_heading(title string) &TabStrip {
	mut s := unsafe { self_ }
	s.heading = title
	return s
}

// list_items is the `...` menu: every tab, the active one checked.
fn (s &TabStrip) list_items() []kit.MenuItem {
	mut items := []kit.MenuItem{cap: s.tabs.len + 3}
	if s.heading != '' {
		items << kit.MenuItem{
			label:   s.heading
			heading: true
		}
	}
	for tab in s.tabs {
		items << kit.MenuItem{
			label:   tab.title
			glyph:   tab.glyph
			checked: tab.id == s.active
			action:  SelectTab{s.change, tab.id}
		}
	}
	if s.heading != '' && s.close != none && s.active != '' {
		items << kit.separator()
		items << kit.MenuItem{
			label:  'Close'
			action: CloseTab{s.close, s.active}
		}
	}
	return items
}

// CloseTab is the `...` menu's Close row.
struct CloseTab {
	close ?core.AnyListener
	id    string
}

pub fn (c CloseTab) run(mut cx core.Cx) {
	kit.fire(c.close, c.id, mut cx)
}

// after adds elements past the overflow menu (a panel toggle).
pub fn (self_ &TabStrip) leads(m map[string][]core.Element) &TabStrip {
	mut s := unsafe { self_ }
	s.leads = m.clone()
	return s
}

pub fn (self_ &TabStrip) after(els []core.Element) &TabStrip {
	mut s := unsafe { self_ }
	s.after = els
	return s
}

// tab_width limits a tab's content width (ellipsis past max).
pub fn (self_ &TabStrip) tab_width(min f32, max f32) &TabStrip {
	mut s := unsafe { self_ }
	s.min_w, s.max_w = min, max
	return s
}

pub fn (self_ &TabStrip) on_rename(l core.Listener[string]) &TabStrip {
	mut s := unsafe { self_ }
	s.rename = l.any
	return s
}
