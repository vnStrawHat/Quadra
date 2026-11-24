//! The cards layout of settings (specs/design, "Settings, cards"): a column of 250 px with
//! icons, a header for the page, the groups as outlined cards.

use quadra_layout::{Align, Justify, Length};
use quadra_reactive::{Memo, Signal, memo, signal};
use quadra_tree::{Role, State};
use quadra_view::{AnyView, Div, by_text, column, div, dynamic, row, scroll, text};

use quadra_headless::settings::{Model, opens, shown};

use super::{Setting, SettingsGroup, SettingsPage, acts, search};
use crate::icon::icon;
use crate::{icons, size, token};

/// A row of the column, 28 px: filled when chosen, its label a step heavier.
fn line(title: &str, on: impl Fn() -> bool + 'static) -> Div {
    let line = row()
        .align_items(Align::Center)
        .gap(token::SPACE_MD)
        .min_height(by_text(28.0))
        .padding_x(token::SPACE_MD)
        .shrink(0.0);
    let line = line.radius(token::RADIUS_MD).role(Role::TreeItem).label(title.to_string()).selected(on);
    let line = line.hover(|s| s.bg(token::PRESS.alpha(0.8)));
    line.when(State::SELECTED, |s| s.bg(token::PRESS).font_weight(500))
}

/// The column of the pages. A page lists its titled groups under it when more than one of
/// its groups keeps a row: a click on its row opens them, its chevron alone opens and closes.
pub(super) fn nav(pages: &[SettingsPage], model: &Model, query: Option<Signal<String>>) -> Div {
    let side = column().width(251.0).shrink(0.0).bg(token::PANEL).role(Role::Tree);
    let mut side = side.border_right(1.0).border_color(token::LINE);
    if let Some(query) = query {
        side = side
            .child(div().padding_top(token::SPACE_LG).padding_x(token::SPACE_LG).child(search(query, 226.0)));
    }
    let mut list = column().gap(token::SPACE_MD).padding(token::SPACE_LG);
    for (i, page) in pages.iter().enumerate() {
        let (open, kept, searching) = (model.open[i], model.groups[i].clone(), model.searching);
        let (on, picked, page_kept) = (model.shown, model.picked, model.pages[i]);
        let count = memo(move || kept.iter().filter(|kept| kept.get()).count());
        let titled: Vec<Memo<bool>> = (page.groups.iter().zip(model.groups[i].iter()))
            .map(|(group, kept)| (!group.title.is_empty(), *kept))
            .map(|(named, kept)| memo(move || named && count.get() > 1 && kept.get()))
            .collect();
        let lists = titled.clone();
        let lists = memo(move || lists.iter().any(|shows| shows.get()));

        // A chosen group is the chosen row: its page's row is not, unless it is alone there.
        let chosen =
            move || on.get() == i && (picked.get().is_none_or(|(at, _)| at != i) || count.get() == 1);
        let mut head = line(&page.title, chosen);
        if let Some(body) = page.icon {
            head = head.child(icon(body).size(16.0));
        }
        let body = move || icon(if open.get() { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT });
        let chevron = row().align_items(Align::Center).justify(Justify::Center).size(20.0, 20.0);
        let chevron = chevron.shrink(0.0).radius(token::RADIUS_MD).hover(|s| s.bg(token::PRESS));
        let chevron = chevron.display(move || shown(lists.get())).on_click(move |e| {
            open.update(|open| *open = !*open);
            e.stop(); // the chevron opens and closes; it does not choose the page
        });
        let chevron = chevron.child(dynamic(move || body().size(16.0)));
        let name = div().grow(1.0).min_width(0.0).child(text(page.title.clone()));
        let pick = model.clone();
        let head = acts(head, move || {
            open.set(true);
            pick.go(i, None, 0.0);
        });
        let head = opens(head, open).child(name).child(chevron);

        let mut under =
            column().gap(token::SPACE_XS).padding_left(token::SPACE_MD).padding_y(token::SPACE_XXS);
        under = under.border_left(1.0).border_color(token::LINE_STRONG);
        for (g, group) in page.groups.iter().enumerate() {
            let (shows, pick) = (titled[g], model.clone());
            let chosen = move || on.get() == i && picked.get() == Some((i, g));
            let row = line(&group.title, chosen).display(move || shown(shows.get()));
            under = under.child(acts(row, move || pick.go(i, Some(g), 0.0)).child(text(group.title.clone())));
        }
        let under = column()
            .padding_left(token::SPACE_XL)
            .display(move || shown(open.get() && lists.get()))
            .child(under);
        let item = column().display(move || shown(!searching.get() || page_kept.get()));
        list = list.child(item.child(head).child(under));
    }
    side.child(scroll(signal(0.0), list))
}

/// A setting: its title over a line about it, at most 60 % of the row; the control at the
/// right.
fn setting(row_: Setting, kept: Memo<bool>) -> Div {
    let line = row().align_items(Align::Center).justify(Justify::Between).gap(token::SPACE_LG);
    let line = line.display(move || shown(kept.get()));
    if row_.title.is_empty() && row_.about.is_empty() {
        return line.child(div().grow(1.0).min_width(0.0).child(row_.control));
    }
    let title = text(row_.title.clone()).line_height(20.0 / 14.0);
    let mut words = column().grow(1.0).min_width(0.0).max_width(Length::Pct(60.0)).child(title);
    if !row_.about.is_empty() {
        words = words.child(text(row_.about).color(token::TEXT_2));
    }
    let control = super::control(&row_.title, row_.control, row_.reset);
    line.role(Role::Group).label(row_.title).child(words).child(control)
}

/// A group: its title and a line about it, muted, over an outlined card of its rows; shown
/// while `kept` holds, each row while `row(r)` does (what a search keeps).
pub(super) fn group_card(group: SettingsGroup, kept: Memo<bool>, row: impl Fn(usize) -> Memo<bool>) -> Div {
    let mut whole = column().gap(token::SPACE_XL).display(move || shown(kept.get()));
    if !group.title.is_empty() {
        let title = text(group.title).size(size::TEXT_LG.size).color(token::TEXT_2).line_height(1.25);
        let mut head = column().gap(token::SPACE_XS).child(title);
        if !group.about.is_empty() {
            head = head.child(text(group.about).color(token::TEXT_2));
        }
        whole = whole.child(head.role(Role::Heading));
    }
    let card = column().gap(token::SPACE_XL).padding(token::SPACE_XL).radius(token::RADIUS_MD);
    let card = card.border(1.0).border_color(token::LINE_STRONG);
    let rows = group.rows.into_iter().enumerate().map(|(r, line)| setting(line, row(r)));
    whole.child(card.children(rows.collect::<Vec<_>>()))
}

/// The page: a header with its title, what the app put at its right and a line about the
/// page; under it, in what scrolls, each group a title over an outlined card of its rows.
pub(super) fn page(pages: Vec<SettingsPage>, model: &Model, header: Vec<AnyView>) -> Div {
    let on = model.shown;
    let told: Vec<bool> = pages.iter().map(|page| !page.about.is_empty()).collect();
    let (mut titles, mut abouts, mut all) = (div(), div(), column());
    for (i, page) in pages.into_iter().enumerate() {
        let here = move || shown(on.get() == i);
        let title = div().role(Role::Heading).text_size(size::TEXT_LG.size).line_height(1.625);
        titles = titles.child(title.display(here).child(text(page.title)));
        if !page.about.is_empty() {
            abouts = abouts.child(div().display(here).child(text(page.about).color(token::TEXT_2)));
        }
        let mut one = column().display(here);
        for (g, group) in page.groups.into_iter().enumerate() {
            let (kept, rows) = (model.groups[i][g], &model.rows[i][g]);
            let whole = group_card(group, kept, |r| rows[r]).padding(token::SPACE_XL);
            one = one.child(whole.node_ref(&model.heads[i][g]));
        }
        all = all.child(one);
    }
    let top = row().align_items(Align::Center).justify(Justify::Between).gap(token::SPACE_MD);
    let top = top.child(titles).child(row().align_items(Align::Center).gap(token::SPACE_XS).children(header));
    let head = column()
        .gap(token::SPACE_LG)
        .padding(token::SPACE_XL)
        .shrink(0.0)
        .border_bottom(1.0)
        .border_color(token::LINE);
    let abouts = abouts.display(move || shown(told.get(on.get()) == Some(&true)));
    let head = head.child(top).child(abouts);
    column().grow(1.0).min_width(0.0).child(head).child(scroll(model.scroll, model.scrolled(all)))
}
