//! The rules layout of settings (behaviour/15, "Settings"): groups under small mono headers,
//! a hairline under every row.

use quadra_layout::{Align, Justify};
use quadra_reactive::{Memo, Signal, memo, signal};
use quadra_tree::{Role, State};
use quadra_view::{AnyView, Div, Region, by_text, column, div, dynamic, row, scroll, text};

use quadra_headless::settings::{Model, opens, shown};

use super::{Setting, SettingsPage, acts, search};
use crate::display::{heading, key_hint};
use crate::icon::icon;
use crate::{fonts, icons, size, token};

/// The page's padding at the top: a chosen group's header comes as far under the top.
const TOP: f32 = 24.0;

/// The column of the pages, 226 px: the search field, rows of 28 px whose chevron shows the
/// page's groups under it, the key hint at the bottom over a hairline.
pub(super) fn nav(
    pages: &[SettingsPage],
    model: &Model,
    query: Option<Signal<String>>,
    hint: Option<(String, String)>,
) -> Div {
    let side = column().width(226.0).shrink(0.0).padding(10.0).gap(2.0).role(Role::Tree);
    let mut side = side.border_right(1.0).border_color(token::LINE_STRONG);
    if let Some(query) = query {
        side = side.child(div().padding_bottom(10.0).child(search(query, 205.0)));
    }
    let mut list = column().gap(2.0);
    for (i, page) in pages.iter().enumerate() {
        let (open, kept, searching) = (model.open[i], model.pages[i], model.searching);
        let under = column().gap(2.0).display(move || shown(open.get() && kept.get()));
        let groups = page.groups.iter().enumerate().map(|(g, group)| {
            let (kept, model) = (model.groups[i][g], model.clone());
            let line = row()
                .align_items(Align::Center)
                .min_height(by_text(26.0))
                .padding_left(28.0)
                .padding_right(6.0);
            let line = line.shrink(0.0).radius(token::RADIUS_MD).text_color(token::TEXT_2);
            let line = line.role(Role::TreeItem).label(group.title.clone());
            let line = line.display(move || shown(kept.get())).hover(|s| s.bg(token::PANEL_HOVER));
            acts(line, move || model.go(i, Some(g), TOP)).child(text(group.title.clone()))
        });
        let line = page_row(i, page, model).display(move || shown(!searching.get() || kept.get()));
        list = list.child(line).child(under.children(groups.collect::<Vec<_>>()));
    }
    side = side.child(scroll(signal(0.0), list));
    match hint {
        Some((keys, does)) => {
            let foot = div().padding_top(10.0).border_top(1.0).border_color(token::LINE);
            side.child(foot.child(key_hint(&keys, does)))
        }
        None => side,
    }
}

/// The row of a page: a chevron that opens it, and its title. The page that shows is a step
/// up, in a hairline; the others have the line too, and it does not show.
fn page_row(i: usize, page: &SettingsPage, model: &Model) -> Div {
    let (open, on, model) = (model.open[i], model.shown, model.clone());
    let body = move || icon(if open.get() { icons::CHEVRON_DOWN } else { icons::CHEVRON_RIGHT });
    let chevron = row().align_items(Align::Center).justify(Justify::Center).size(16.0, 16.0).shrink(0.0);
    let chevron = chevron.radius(token::RADIUS_SM).hover(|s| s.bg(token::PRESS)).on_click(move |e| {
        open.update(|open| *open = !*open);
        e.stop(); // the chevron opens; it does not choose the page
    });
    let chevron = chevron.child(dynamic(move || body().size(12.0).color(token::TEXT_2)));
    let line = row().align_items(Align::Center).gap(6.0).min_height(by_text(28.0)).padding_x(5.0).shrink(0.0);
    let line = line.radius(token::RADIUS_MD).border(1.0).border_color(token::LINE.alpha(0.0));
    let line = line.role(Role::TreeItem).label(page.title.clone()).selected(move || on.get() == i);
    let line = line.hover(|s| s.bg(token::PANEL_HOVER));
    let line = line.when(State::SELECTED, |s| s.bg(token::RAISED_HOVER).border_color(token::LINE_STRONG));
    let line = opens(acts(line, move || model.go(i, None, TOP)), open);
    line.child(chevron).child(text(page.title.clone()))
}

/// A setting: its title over a line about it that wraps, the control at the right, 16 px
/// above and below, a hairline under.
fn setting(row_: Setting, kept: Memo<bool>) -> Div {
    let line = row().align_items(Align::Center).gap(16.0).padding_y(16.0);
    let line = line.border_bottom(1.0).border_color(token::LINE).display(move || shown(kept.get()));
    if row_.title.is_empty() && row_.about.is_empty() {
        return line.child(div().grow(1.0).min_width(0.0).child(row_.control));
    }
    let mut words = column().gap(2.0).grow(1.0).min_width(0.0).child(text(row_.title.clone()));
    if !row_.about.is_empty() {
        words = words.child(text(row_.about).size(size::TEXT_SM.size).color(token::TEXT_2));
    }
    let control = super::control(&row_.title, row_.control, row_.reset);
    line.role(Role::Group).label(row_.title).child(words).child(control)
}

/// The pages, in what scrolls: what the app put above them, then of the page that shows its
/// title and its groups, each a mono header over a hairline and its rows.
pub(super) fn page(
    pages: Vec<SettingsPage>,
    model: &Model,
    resets: Vec<Option<Div>>,
    header: Vec<AnyView>,
) -> Region {
    let headed = !header.is_empty();
    let mut all = column().padding_left(40.0).padding_right(32.0).padding_y(TOP);
    if headed {
        all = all.child(row().align_items(Align::Center).gap(4.0).children(header));
    }
    for (i, (page, reset)) in pages.into_iter().zip(resets).enumerate() {
        let (on, kept) = (model.shown, model.groups[i].clone());
        // The first group that shows is nearer the title than the others are to each other.
        let first = memo(move || kept.iter().position(|kept| kept.get()));
        let title = div().padding_top(if headed { 24.0 } else { 0.0 }).child(heading(page.title, 2));
        let title = match reset {
            Some(reset) => title.align_items(Align::Center).justify(Justify::Between).child(reset),
            None => title,
        };
        let mut one = column().display(move || shown(on.get() == i)).child(title);
        for (g, group) in page.groups.into_iter().enumerate() {
            let kept = model.groups[i][g];
            let name = text(group.title).size(size::TEXT_SM.size).color(token::TEXT_2);
            let name = div().font_family(fonts::MONO).node_ref(&model.heads[i][g]).child(name);
            let above = move || if first.get() == Some(g) { 8.0 } else { 32.0 };
            let head = column().padding_top(above).padding_bottom(6.0);
            let mut head = head.border_bottom(1.0).border_color(token::LINE).child(name);
            if !group.about.is_empty() {
                let about = text(group.about).size(size::TEXT_SM.size).color(token::TEXT_3);
                head = head.child(div().padding_top(2.0).child(about));
            }
            let rows = group.rows.into_iter().enumerate().map(|(r, row)| setting(row, model.rows[i][g][r]));
            let all_of = column().display(move || shown(kept.get())).child(head);
            one = one.child(all_of.children(rows.collect::<Vec<_>>()));
        }
        all = all.child(one);
    }
    scroll(model.scroll, model.scrolled(all)).container(div().grow(1.0).min_width(0.0))
}
