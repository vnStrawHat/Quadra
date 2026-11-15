//! The look of the tab strip (specs/behaviour/16, "Tab strip"): the open views of a pane, one
//! of them showing; a cross closes a tab, and a list at the end of the strip names them all.
//! What the tabs do is `quadra_headless::work::Strip`.

use std::rc::Rc;

use quadra_headless::work::{self, Dock, Strip, Tab};
use quadra_layout::{Align, Display, Justify};
use quadra_platform::{Cursor, Key};
use quadra_reactive::{Signal, effect, memo, signal};
use quadra_tree::{NodeId, Paint, Role};
use quadra_view::{
    AnyView, Div, MountCx, NodeRef, View, by_text, div, dynamic, each, row, scroll_strip, text, text_input,
};

use crate::icon::icon;
use crate::menu::{MenuItem, context_menu, menu_icon_button};
use crate::popup::surface;
use crate::{icons, token};

/// How wide a tab's title may be: past that it ends with an ellipsis.
const TITLE_MAX: f32 = 200.0;

type TabMenu = Rc<dyn Fn(&str) -> Vec<MenuItem>>;
type Rename = Rc<dyn Fn(&str, &str)>;
/// A tab's menu, and the id of the tab being renamed with what renames it.
type More = (Option<TabMenu>, Option<(Signal<Option<String>>, Rename)>);

pub struct TabStrip {
    strip: Strip,
    actions: Vec<AnyView>,
    menu: Option<TabMenu>,
    rename: Option<Rename>,
}

/// A strip of 32 px with a tab for each of `tabs`, the one whose id `active` holds joined to
/// what is under the strip. A press writes `active`, a drag the order of `tabs`.
pub fn tab_strip(tabs: Signal<Vec<Tab>>, active: Signal<String>) -> TabStrip {
    TabStrip { strip: work::strip(tabs, active), actions: Vec::new(), menu: None, rename: None }
}

impl TabStrip {
    /// Tabs can be closed, by the cross that shows under the pointer or a click with the
    /// middle button: `f` is told which tab, and takes it out of the tabs if it agrees.
    pub fn on_close(mut self, f: impl Fn(&str) + 'static) -> Self {
        self.strip = self.strip.on_close(f);
        self
    }

    /// Makes the strip that of a group of a docking workbench: a tab is torn off it (dragged
    /// 12 px up or down) and dropped on another strip or on a pane, and the strip takes the
    /// tabs dropped on it, where they are dropped.
    pub fn dock(mut self, dock: Dock, group: impl Into<String>) -> Self {
        self.strip = self.strip.dock(dock, group, chip);
        self
    }

    /// Something at the end of the strip, before the list of the tabs: split, more.
    pub fn action(mut self, view: impl View + 'static) -> Self {
        self.actions.push(AnyView::new(view));
        self
    }

    /// A tab's own menu, opened by the other button on it: `f` is given the tab's id.
    pub fn menu(mut self, f: impl Fn(&str) -> Vec<MenuItem> + 'static) -> Self {
        self.menu = Some(Rc::new(f));
        self
    }

    /// Tabs can be renamed: a double click on a title puts it in a field, where Enter or
    /// leaving gives `f` the id and the new title, and Escape gives it up.
    pub fn on_rename(mut self, f: impl Fn(&str, &str) + 'static) -> Self {
        self.rename = Some(Rc::new(f));
        self
    }
}

/// The title of a tab, cut with an ellipsis past its width; a double click on it, when
/// tabs are renamed, writes its id to `editing`.
fn title(
    tab: Signal<Tab>,
    tone: impl Fn() -> Paint + 'static,
    renames: Option<(Signal<Option<String>>, String)>,
) -> Div {
    let words = text(move || tab.with(|tab| tab.title.clone())).ellipsis();
    let title = div().max_width(TITLE_MAX).text_color(tone).child(words);
    match renames {
        Some((editing, id)) => title.on_click(move |e| {
            if e.clicks == 2 {
                editing.set(Some(id.clone()));
            }
        }),
        None => title,
    }
}

/// The field a title is renamed in, with the focus: Enter or leaving it renames, Escape
/// gives up; either way the title shows again.
fn field(tab: Signal<Tab>, editing: Signal<Option<String>>, rename: Rename, id: String) -> Div {
    let value = signal(tab.with_untracked(|tab| tab.title.clone()));
    let mine = id.clone();
    let done = Rc::new(move |keep: bool| {
        if editing.with_untracked(|on| on.as_deref() == Some(mine.as_str())) {
            editing.set(None);
            if keep {
                rename(&mine, value.get_untracked().trim());
            }
        }
    });
    let (submit, escape, leave) = (done.clone(), done.clone(), done);
    let input = text_input(value).size(13.0).on_submit(move |_| submit(true));
    let field = div().width(TITLE_MAX.min(160.0)).border(1.0).border_color(token::BORDER_FOCUSED);
    let field = field.radius(token::RADIUS_SM).padding_x(2.0).child(input);
    let field = field.on_key_down(move |e| {
        if e.key == Key::Escape {
            e.stop();
            escape(false);
        }
    });
    let field = field.on_focus(move |has| {
        if !has {
            leave(true);
        }
    });
    field.on_mount(|tree, id| {
        tree.after_layout(move |tree| {
            tree.focus_first(id);
        })
    })
}

/// What follows the pointer while a tab is dragged: its icon and its title on a chip.
fn chip(tab: &Tab) -> Div {
    let chip = row().align_items(Align::Center).gap(4.0).height(by_text(28.0)).padding_x(8.0);
    let chip = match tab.icon {
        Some(body) => chip.child(icon(body).color(token::TEXT_2)),
        None => chip,
    };
    let chip = chip.child(text(tab.title.clone()).color(token::TEXT));
    surface(1).bg(token::CANVAS).border_color(token::LINE_STRONG).radius(token::RADIUS_MD).child(chip)
}

/// One tab: 32 px high on the plane of tabs, or on the canvas with no line under it while
/// it is the active one; the seams are on the side away from the active tab.
fn face(strip: &Strip, tab: Signal<Tab>, more: More) -> impl View + use<> {
    let (tabs, active, id) = (strip.tabs, strip.active, tab.with_untracked(|tab| tab.id.clone()));
    let (me, mine) = (id.clone(), id.clone());
    let on = memo(move || active.with(|active| *active == me));
    // Where the tab and the active one are in the strip.
    let place = memo(move || {
        let (i, at) = (tabs.with(|all| all.iter().position(|tab| tab.id == mine)), active.get());
        (i.unwrap_or(0), tabs.with(|all| all.iter().position(|tab| tab.id == at)))
    });
    let dot = div().size(6.0, 6.0).radius(3.0).bg(token::ACCENT);
    let dot = dot.display(move || if tab.with(|tab| tab.dirty) { Display::Flex } else { Display::None });
    let mark = row().size(12.0, 12.0).align_items(Align::Center).justify(Justify::Center).child(dot);
    let content = row().align_items(Align::Center).gap(4.0).padding_x(4.0).grow(1.0).child(mark);
    let content = match tab.with_untracked(|tab| tab.icon) {
        Some(body) => content.child(icon(body).color(token::TEXT_2)),
        None => content,
    };
    let tone = move || -> Paint { if on.get() { token::TAB_ACTIVE_FG } else { token::TAB_FG }.into() };
    let (menu, renames) = more;
    let content = match renames {
        Some((editing, rename)) => {
            let me = id.clone();
            content.child(dynamic(move || match editing.with(|on| on.as_deref() == Some(me.as_str())) {
                true => field(tab, editing, rename.clone(), me.clone()),
                false => title(tab, tone, Some((editing, me.clone()))),
            }))
        }
        None => content.child(title(tab, tone, None)),
    };
    let content = match &strip.close {
        // The cross has no colour of its own: it takes that of the tab's text, which is the
        // tab's own fill until the pointer is over the tab.
        Some(_) => {
            let cross =
                row().size(18.0, 18.0).shrink(0.0).align_items(Align::Center).justify(Justify::Center);
            let cross = cross.radius(token::RADIUS_MD).hover(|s| s.bg(token::PANEL_HOVER));
            let cross = cross.role(Role::Button).label("Close").child(icon(icons::X));
            let cross = strip.closes(cross, &id);
            let room = row().size(14.0, 14.0).align_items(Align::Center).justify(Justify::Center);
            content.child(room.child(cross))
        }
        None => content.padding_right(8.0),
    };
    let fill = move || -> Paint { if on.get() { token::CANVAS } else { token::TAB_BG }.into() };
    let face = row().shrink(0.0).cursor(Cursor::Pointer).border_color(token::LINE_STRONG);
    let face = face.bg(fill).text_color(fill).hover(|s| s.text_color(token::TEXT));
    let face = face.border_bottom(move || if on.get() { 0.0 } else { 1.0 });
    // The active tab has the accent's line along its top.
    let face = face.child(crate::display::chosen_mark(quadra_tree::Side::Above, move || on.get()));
    let seam = |there: bool| if there { 1.0 } else { 0.0 };
    let face = face.border_left(move || {
        let (i, at) = place.get();
        seam(i > 0 && at.is_none_or(|at| i <= at))
    });
    let face = face.border_right(move || {
        let (i, at) = place.get();
        seam(at.is_some_and(|at| i >= at))
    });

    // The tab that becomes the active one is scrolled to in the strip: one chosen from the
    // menu of tabs, one just opened past the end.
    let shown = NodeRef::new();
    let reveal = shown.clone();
    let face = face.child(content).node_ref(&shown).on_mount(move |_, _| {
        effect(move || {
            if on.get() {
                reveal.scroll_into_view();
            }
        });
    });
    let face = strip.tab(face, tab);
    match menu {
        Some(items) => AnyView::new(context_menu(face, move || items(&id))),
        None => AnyView::new(face),
    }
}

impl View for TabStrip {
    fn mount(mut self, cx: &mut MountCx) -> NodeId {
        let (strip, actions) = (self.strip, std::mem::take(&mut self.actions));
        let (tabs, active, look) = (strip.tabs, strip.active, strip.clone());
        let (menu, editing) = (self.menu, signal(None::<String>));
        let renames = self.rename.map(|rename| (editing, rename));
        let faces = each(
            move || tabs.get(),
            |tab| tab.id.clone(),
            move |tab| face(&look, tab, (menu.clone(), renames.clone())),
        );
        let faces = faces.container(strip.lane(row().shrink(0.0)));
        // The line of the bar runs under the tabs that are not active, the room that is left
        // and what stands at the end.
        let rest = div().grow(1.0).border_bottom(1.0).border_color(token::LINE_STRONG);
        let all = row().role(Role::TabList).child(faces).child(rest);
        // Every tab by name, the active one checked.
        let list = move || {
            let named = |tab: &Tab| {
                let id = tab.id.clone();
                let item = MenuItem::new(tab.title.clone()).checked(active.with_untracked(|on| *on == id));
                let item = item.on_select(move || active.set(id.clone()));
                tab.icon.map_or(item.clone(), |body| item.icon(body))
            };
            tabs.with_untracked(|all| all.iter().map(named).collect())
        };
        let end = row().align_items(Align::Center).gap(4.0).padding_x(6.0).shrink(0.0);
        let end = end.border_bottom(1.0).border_left(1.0).border_color(token::LINE_STRONG);
        let end = end.children(actions).child(menu_icon_button(icons::ELLIPSIS, "Tabs", list));
        let bar = row().height(by_text(32.0)).shrink(0.0).bg(token::TAB_BG);
        let bar = bar.child(scroll_strip(signal(0.0), all)).child(end).mount(cx);
        strip.mounted(cx, bar);
        bar
    }
}
