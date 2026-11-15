//! The rail (specs/19, "Rail"): 248 px of tabs on the panel colour, a heading with the
//! count, a row a tab, and "New terminal". A click shows a tab, the middle button closes it,
//! two clicks rename it, the right button opens its menu, and rows drag to reorder.

use quadra::{
    Align, AnyView, Display, MouseButton, NodeRef, PointerKind, Role, Signal, View, by_text, column, div,
    dynamic, each, row, signal, text,
};
use quadra_design::{MenuItem, context_menu, fonts, icon, icon_button, icons, input, menu, token};

use crate::marks;
use crate::model::{Model, Tab};
use crate::shells;

/// What a tab's row carries while it is dragged.
pub(crate) struct Dragged(pub u64);

pub(crate) fn rail(model: &Model) -> impl View + use<> {
    let (tabs, vertical) = (model.tabs(), model.vertical());
    let heading = row()
        .align_items(Align::Center)
        .gap(6.0)
        .padding_left(14.0)
        .padding_right(6.0)
        .height(by_text(36.0))
        .text_size(11.0)
        .text_color(token::TEXT_3)
        .child(div().child(text("TABS")))
        .child(div().grow(1.0).font_family(fonts::MONO).child(text(move || tabs.with(Vec::len).to_string())))
        .child(icon_button(icons::LAYOUT_GRID, "Horizontal tabs").on_click(move || vertical.set(false)));
    let shown = model.clone();
    let rows =
        each(move || tabs.get(), |tab| tab.id, move |tab: Signal<Tab>| tab_row(&shown, tab.get_untracked()));
    let (open, opener, making) = (signal(false), NodeRef::new(), model.clone());
    let new = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(8.0)
        .height(by_text(32.0))
        .radius(6.0)
        .text_color(token::TEXT_2)
        .hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT))
        .role(Role::Button)
        .label("New terminal")
        .on_click(move |_| open.update(|open| *open = !*open))
        .child(icon(icons::PLUS).size(14.0))
        .child(div().grow(1.0).child(text("New terminal")))
        .child(
            div().font_family(fonts::MONO).text_size(11.0).text_color(token::TEXT_3).child(text("Ctrl+T")),
        );
    column()
        .width(248.0)
        .shrink(0.0)
        .bg(token::PANEL)
        .border_right(1.0)
        .border_color(token::LINE)
        .role(Role::TabList)
        .label("Tabs")
        .child(heading)
        .child(rows.container(column().gap(2.0).padding_x(8.0)))
        .child(div().padding_x(8.0).padding_y(4.0).node_ref(&opener).child(new))
        .child(menu(open, &opener, move || new_menu(&making)))
}

/// The `+` menu: the default shell, each shell found, a split of the space with the focus.
pub(crate) fn new_menu(model: &Model) -> Vec<MenuItem> {
    let (local, splits) = (model.clone(), model.clone());
    let mut items = vec![MenuItem::new("Local shell").keys("Ctrl+T").on_select(move || local.new_tab())];
    for shell in shells::found() {
        let (makes, name) = (model.clone(), shell.name);
        items.push(
            MenuItem::new(name).on_select(move || makes.new_tab_with(makes.launch(Some(shell.clone())))),
        );
    }
    let connects = model.connecting();
    items.push(MenuItem::new("Quick connect…").keys("Ctrl+Shift+K").on_select(move || connects.set(true)));
    items.push(MenuItem::separator());
    items.push(MenuItem::new("Split current space").keys("Ctrl+D").on_select(move || splits.split()));
    items
}

/// The menu of a tab: Rename, Duplicate, Close, Close others.
pub(crate) fn tab_menu(model: &Model, id: u64) -> Vec<MenuItem> {
    let (renames, copies, closes, keeps) = (model.renaming(), model.clone(), model.clone(), model.clone());
    vec![
        MenuItem::new("Rename").on_select(move || renames.set(Some(id))),
        MenuItem::new("Duplicate").on_select(move || copies.duplicate(id)),
        MenuItem::separator(),
        MenuItem::new("Close").on_select(move || _ = closes.close_tab(id)),
        MenuItem::new("Close others").on_select(move || keeps.close_others(id)),
    ]
}

/// The title of a tab, or the field that renames it: Enter keeps the name (an empty one
/// gives the program's title back), Escape leaves it as it was.
pub(crate) fn title(model: &Model, tab: Tab) -> impl View + use<> {
    let renaming = model.renaming();
    dynamic(move || {
        if renaming.get() != Some(tab.id) {
            return AnyView::new(text(move || tab.title()).ellipsis());
        }
        let value = signal(tab.name.get_untracked().unwrap_or_else(|| tab.title()));
        let field = input(value).on_submit(move |name: &str| {
            let name = name.trim();
            tab.name.set((!name.is_empty()).then(|| name.to_string()));
            renaming.set(None);
        });
        let field = div()
            .on_key_down(move |e| {
                if e.key == quadra::Key::Escape {
                    renaming.set(None);
                }
            })
            .on_mount(|tree, id| _ = tree.focus_first(id))
            .child(field);
        AnyView::new(field)
    })
}

/// A tab's row: the accent bar and fill while it shows, its icon, its title and a line on
/// what it holds.
fn tab_row(model: &Model, tab: Tab) -> impl View + use<> {
    let active = model.active();
    let on = move || active.get() == tab.id;
    let (shows, closes, moves, menu) = (model.clone(), model.clone(), model.clone(), model.clone());
    let bar = div()
        .absolute()
        .left(0.0)
        .top(8.0)
        .bottom(8.0)
        .width(2.0)
        .bg(token::ACCENT)
        .display(move || if on() { Display::Flex } else { Display::None });
    let words = column()
        .grow(1.0)
        .min_width(0.0)
        .child(div().text_color(token::TEXT).child(title(model, tab)))
        .child(
            div()
                .text_size(11.0)
                .font_family(fonts::MONO)
                .text_color(token::TEXT_2)
                .child(text(move || marks::tab_line(tab)).ellipsis()),
        );
    let face = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(10.0)
        .height(by_text(46.0))
        .radius(6.0)
        .bg(move || if on() { token::RAISED } else { token::PANEL })
        .hover(|s| s.bg(token::PANEL_HOVER))
        .drop_target(|s| s.bg(token::ACCENT_TINT))
        .role(Role::Tab)
        .label(move || tab.title())
        .selected(on)
        .on_click(move |e| {
            shows.show(tab.id);
            if e.clicks == 2 {
                shows.renaming().set(Some(tab.id));
            }
        })
        .on_pointer(PointerKind::Click, move |e| {
            if e.button == MouseButton::Middle {
                closes.close_tab(tab.id);
            }
        })
        .draggable(move || Dragged(tab.id))
        .on_drop::<Dragged>(move |dragged, _| {
            let to = moves.tabs().with_untracked(|tabs| tabs.iter().position(|it| it.id == tab.id));
            moves.move_tab(dragged.0, to.unwrap_or(usize::MAX));
        })
        .child(bar)
        .child(marks::tab_icon(tab))
        .child(words)
        .child(marks::tab_mark(tab))
        .child(marks::progress_bar(tab));
    context_menu(face, move || tab_menu(&menu, tab.id))
}
