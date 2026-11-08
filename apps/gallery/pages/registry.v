// The page registry: one page per component, grouped into sidebar sections.
module pages

import quadra.core
import quadra.kit

pub struct PageInfo {
pub:
	id      string
	title   string
	section string
	glyph   string
}

pub fn all() []PageInfo {
	return [
		PageInfo{'start', 'Getting started', 'Overview', kit.icon_house},
		PageInfo{'typography', 'Typography', 'Overview', kit.icon_type},
		PageInfo{'colors', 'Colors', 'Overview', kit.icon_palette},
		PageInfo{'icons', 'Icons', 'Overview', kit.icon_star},
		PageInfo{'button', 'Button', 'Actions', kit.icon_square_check},
		PageInfo{'icon-button', 'Icon button', 'Actions', kit.icon_settings},
		PageInfo{'toggle-button', 'Toggle button', 'Actions', kit.icon_bold},
		PageInfo{'checkbox', 'Checkbox', 'Forms', kit.icon_square_check},
		PageInfo{'switch', 'Switch', 'Forms', kit.icon_toggle_left},
		PageInfo{'radio', 'Radio group', 'Forms', kit.icon_circle_dot},
		PageInfo{'segmented', 'Segmented control', 'Forms', kit.icon_layout_grid},
		PageInfo{'slider', 'Slider', 'Forms', kit.icon_sliders_horizontal},
		PageInfo{'text-input', 'Text input', 'Forms', kit.icon_type},
		PageInfo{'password', 'Password input', 'Forms', kit.icon_lock},
		PageInfo{'number', 'Number input', 'Forms', kit.icon_plus},
		PageInfo{'textarea', 'Textarea', 'Forms', kit.icon_pencil},
		PageInfo{'combobox', 'Combobox', 'Forms', kit.icon_search},
		PageInfo{'select', 'Select', 'Forms', kit.icon_chevron_down},
		PageInfo{'date-picker', 'Date and time', 'Forms', kit.icon_calendar},
		PageInfo{'field', 'Form field', 'Forms', kit.icon_circle_check},
		PageInfo{'status', 'Status', 'Display', kit.icon_circle_dot},
		PageInfo{'badge', 'Badge', 'Display', kit.icon_bookmark},
		PageInfo{'tag', 'Tag', 'Display', kit.icon_tag},
		PageInfo{'kbd', 'Kbd', 'Display', kit.icon_command},
		PageInfo{'rich-text', 'Rich text', 'Display', kit.icon_type},
		PageInfo{'markdown', 'Markdown', 'Display', kit.icon_file},
		PageInfo{'alert', 'Alert', 'Display', kit.icon_info},
		PageInfo{'motion', 'Motion', 'Display', kit.icon_layers},
		PageInfo{'empty-state', 'Empty state', 'Display', kit.icon_search},
		PageInfo{'description-list', 'Description list', 'Display', kit.icon_list},
		PageInfo{'avatar', 'Avatar', 'Display', kit.icon_user},
		PageInfo{'progress', 'Progress bar', 'Display', kit.icon_gauge},
		PageInfo{'spinner', 'Spinner', 'Display', kit.icon_loader_circle},
		PageInfo{'skeleton', 'Skeleton', 'Display', kit.icon_minus},
		PageInfo{'card', 'Card', 'Display', kit.icon_layers},
		PageInfo{'tabs', 'Tabs', 'Layout', kit.icon_panel_left},
		PageInfo{'accordion', 'Accordion', 'Layout', kit.icon_list},
		PageInfo{'split', 'Split pane', 'Layout', kit.icon_grip_vertical},
		PageInfo{'scroll-area', 'Scroll area', 'Layout', kit.icon_arrow_down},
		PageInfo{'grid', 'Grid', 'Layout', kit.icon_layout_grid},
		PageInfo{'activity-bar', 'Activity bar', 'Workbench', kit.icon_layers},
		PageInfo{'tab-strip', 'Tab strip', 'Workbench', kit.icon_file},
		PageInfo{'panel-header', 'Panel header', 'Workbench', kit.icon_minus},
		PageInfo{'panes', 'Pane tree', 'Workbench', kit.icon_layout_grid},
		PageInfo{'dock', 'Dock', 'Workbench', kit.icon_layers},
		PageInfo{'terminal', 'Terminal', 'Workbench', kit.icon_terminal},
		PageInfo{'editor', 'Editor', 'Workbench', kit.icon_pencil},
		PageInfo{'settings', 'Settings', 'Workbench', kit.icon_settings},
		PageInfo{'sidebar', 'Sidebar', 'Navigation', kit.icon_panel_left},
		PageInfo{'toolbar', 'Toolbar', 'Navigation', kit.icon_sliders_horizontal},
		PageInfo{'breadcrumb', 'Breadcrumb', 'Navigation', kit.icon_chevron_right},
		PageInfo{'menubar', 'Menu bar', 'Navigation', kit.icon_menu},
		PageInfo{'statusbar', 'Status bar', 'Navigation', kit.icon_info},
		PageInfo{'palette', 'Command palette', 'Navigation', kit.icon_command},
		PageInfo{'menu', 'Menu', 'Overlays', kit.icon_ellipsis},
		PageInfo{'context-menu', 'Context menu', 'Overlays', kit.icon_list},
		PageInfo{'tooltip', 'Tooltip', 'Overlays', kit.icon_message_square},
		PageInfo{'popover', 'Popover', 'Overlays', kit.icon_layers},
		PageInfo{'hover-card', 'Hover card', 'Overlays', kit.icon_info},
		PageInfo{'dialog', 'Dialog', 'Overlays', kit.icon_monitor},
		PageInfo{'drawer', 'Drawer', 'Overlays', kit.icon_panel_left},
		PageInfo{'confirm', 'Confirm', 'Overlays', kit.icon_circle_alert},
		PageInfo{'toast', 'Toast', 'Overlays', kit.icon_bell},
		PageInfo{'table', 'Table', 'Data', kit.icon_table},
		PageInfo{'tree', 'Tree', 'Data', kit.icon_folder},
		PageInfo{'variable-list', 'Variable list', 'Data', kit.icon_list},
		PageInfo{'live-table', 'Live table', 'Data', kit.icon_refresh_cw},
		PageInfo{'log-view', 'Log view', 'Data', kit.icon_list},
		PageInfo{'value-viewer', 'Value viewer', 'Data', kit.icon_layers},
		PageInfo{'sparkline', 'Sparkline', 'Charts', kit.icon_gauge},
		PageInfo{'line-chart', 'Line chart', 'Charts', kit.icon_gauge},
		PageInfo{'bar-chart', 'Bar chart', 'Charts', kit.icon_sliders_horizontal},
		PageInfo{'stress', 'Stress', 'Performance', kit.icon_gauge},
	]
}

// make creates the view for a page id (pages are created lazily, then cached). Pages without
// state render through Static.
pub fn make(id string, mut cx core.Cx) core.AnyView {
	return match id {
		'button' { cx.new_view(ButtonPage{}).any() }
		'toggle-button' { cx.new_view(ToggleButtonPage{}).any() }
		'checkbox' { cx.new_view(CheckboxPage{}).any() }
		'switch' { cx.new_view(SwitchPage{}).any() }
		'radio' { cx.new_view(RadioPage{}).any() }
		'segmented' { cx.new_view(SegmentedPage{}).any() }
		'slider' { cx.new_view(SliderPage{}).any() }
		'text-input' { cx.new_view(TextInputPage{}).any() }
		'password' { cx.new_view(PasswordPage{}).any() }
		'number' { cx.new_view(NumberPage{}).any() }
		'textarea' { cx.new_view(TextareaPage{}).any() }
		'combobox' { cx.new_view(ComboboxPage{}).any() }
		'select' { cx.new_view(SelectPage{}).any() }
		'date-picker' { cx.new_view(DatePage{}).any() }
		'field' { cx.new_view(FieldPage{}).any() }
		'tabs' { cx.new_view(TabsPage{}).any() }
		'accordion' { cx.new_view(AccordionPage{}).any() }
		'split' { cx.new_view(SplitPage{}).any() }
		'sidebar' { cx.new_view(SidebarPage{}).any() }
		'breadcrumb' { cx.new_view(BreadcrumbPage{}).any() }
		'confirm' { cx.new_view(ConfirmPage{}).any() }
		'table' { cx.new_view(TablePage{}).any() }
		'tree' { cx.new_view(TreePage{}).any() }
		'live-table' { cx.new_view(LiveTablePage{}).any() }
		'log-view' { cx.new_view(LogViewPage{}).any() }
		'value-viewer' { cx.new_view(ValueViewPage{}).any() }
		'activity-bar' { cx.new_view(ActivityPage{}).any() }
		'tab-strip' { cx.new_view(TabStripPage{}).any() }
		'panes' { cx.new_view(PanesPage{}).any() }
		'rich-text' { cx.new_view(RichTextPage{}).any() }
		'markdown' { cx.new_view(MarkdownPage{}).any() }
		'dock' { cx.new_view(DockPage{}).any() }
		'terminal' { cx.new_view(TerminalPage{}).any() }
		'editor' { cx.new_view(EditorPage{}).any() }
		'settings' { cx.new_view(SettingsDemo{}).any() }
		else { cx.new_view(Static{ id: id }).any() }
	}
}

// Static renders the pages that hold no state.
pub struct Static {
	id string
}

@[markused]
pub fn (mut p Static) render(mut cx core.ViewCx[Static]) core.Element {
	t := kit.theme(cx)
	return match p.id {
		'typography' { typography_page(t) }
		'colors' { colors_page(t) }
		'icons' { icons_page(t) }
		'icon-button' { icon_button_page(t) }
		'status' { status_page(t) }
		'badge' { badge_page(t) }
		'tag' { tag_page(t) }
		'kbd' { kbd_page(t) }
		'avatar' { avatar_page(t) }
		'progress' { progress_page(t) }
		'spinner' { spinner_page(t) }
		'skeleton' { skeleton_page(t) }
		'card' { card_page(t) }
		'scroll-area' { scroll_page(t) }
		'grid' { grid_page(t) }
		'toolbar' { toolbar_page(t) }
		'menubar' { menubar_page(t) }
		'statusbar' { statusbar_page(t) }
		'palette' { palette_page(t) }
		'menu' { menu_page(t) }
		'context-menu' { context_menu_page(t) }
		'tooltip' { tooltip_page(t) }
		'popover' { popover_page(t) }
		'hover-card' { hover_card_page(t) }
		'dialog' { dialog_page(t) }
		'drawer' { drawer_page(t) }
		'motion' { motion_page(t) }
		'alert' { alert_page(t) }
		'empty-state' { empty_state_page(t) }
		'description-list' { description_list_page(t) }
		'toast' { toast_page(t) }
		'stress' { stress_page(t) }
		'variable-list' { variable_list_page(t) }
		'panel-header' { panel_header_page(t) }
		'sparkline' { sparkline_page(t) }
		'line-chart' { line_chart_page(t) }
		'bar-chart' { bar_chart_page(t) }
		else { start_page(t) }
	}
}
