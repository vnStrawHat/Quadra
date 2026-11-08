// Accessibility: the semantic tree a window shows to assistive technology (screen
// readers, voice control) and the actions they ask for. Windows serves it through UI
// Automation (win/uia_windows.c.v).
module platform

import quadra.prim

// A11yRole mirrors core.Role value for value (a test checks the names match).
pub enum A11yRole {
	none
	button
	link
	checkbox
	radio
	switch
	slider
	textbox
	listbox
	listitem
	menu
	menuitem
	tab
	tablist
	dialog
	heading
	image
	progressbar
	tree
	treeitem
	table
	row
	cell
	group
	tooltip
	alert
}

pub struct A11yNode {
pub:
	id            u64 // stable across frames
	parent        int // index in the same list, -1 = top level
	role          A11yRole
	name          string
	value         string
	automation_id string // the test id
	checked       bool
	disabled      bool
	selected      bool
	expanded      bool
	focused       bool
	focusable     bool
	bounds        prim.Rect // logical px, window client coordinates
}

pub enum A11yAction {
	invoke
	toggle
	select
	expand
	collapse
	focus
}

// A11yEvent asks the app to act on node (an A11yNode id).
pub struct A11yEvent {
pub:
	node   u64
	action A11yAction
}
