//! What shows data: the table, the tree, and a thousand cells to move the pointer over.

use quadra::{Align, Div, Justify, Signal, column, div, row, signal, text, virtual_list};
use quadra_design::{Column, Size, TreeNode, button, fonts, icons, progress, size, table, token, tree};

use crate::common::{demo, framed, hint, line, page, stack};

const NAMES: [&str; 10] = ["An", "Bình", "Chi", "Dũng", "Giang", "Hà", "Khoa", "Lan", "Minh", "Ngọc"];
const CITIES: [&str; 5] = ["Hà Nội", "Huế", "Đà Nẵng", "Cần Thơ", "Hải Phòng"];

pub fn table_page() -> Div {
    let chosen = signal(Vec::new());
    let columns = [
        Column::new("#").numeric().width(90.0),
        Column::new("Tên").width(280.0),
        Column::new("Thành phố").width(280.0),
        Column::new("Điểm").numeric().width(90.0),
    ];
    let cell = |row: usize, column: usize| match column {
        0 => (row + 1).to_string(),
        1 => format!("{} {}", NAMES[row % NAMES.len()], row / 10),
        2 => CITIES[(row / 7) % CITIES.len()].to_string(),
        _ => (row as u64 * 37 % 101).to_string(),
    };
    let people = framed(table(columns, || 1_000_000, cell).selected(chosen)).height(360.0);
    let count = hint(move || format!("{} row(s) selected", chosen.with(Vec::len)));
    let about = "A million rows with sortable, resizable columns, shift/ctrl selection and arrow keys. \
                 Only visible rows are built.";
    page(
        "Table",
        about,
        [
            demo(
                "1,000,000 rows",
                stack().child(people).child(count),
                "table(columns, || 1_000_000, |row, column| ..)",
            ),
            demo(
                "Wide, its first column pinned",
                framed(wide()).height(240.0),
                "table(..).pinned(1).hidden(columns)",
            ),
            demo(
                "Cells that show views, rows that open",
                volumes(),
                "Column::new(\"Used\").view(|row, says| ..), table(..).on_activate(|row| ..)",
            ),
        ],
    )
}

/// Volumes: a mono name, a bar of the space used, a muted note and a button in a row. A
/// double click or Enter opens one.
fn volumes() -> Div {
    const VOLUMES: [(&str, u32, &str); 4] = [
        ("data-postgres-0", 42, "ssd, 50 Gi"),
        ("data-redis-0", 88, "ssd, 8 Gi"),
        ("logs-loki-0", 97, "hdd, 200 Gi"),
        ("cache-web-1", 12, "ssd, 4 Gi"),
    ];
    let used = Column::new("Used").width(200.0).view(|_, says| {
        let part = move || says.get().parse::<f32>().unwrap_or(0.0) / 100.0;
        let bar = progress(part).width(120.0);
        let value = div().font_family(fonts::MONO).child(text(move || format!("{}%", says.get())));
        row().align_items(Align::Center).gap(8.0).child(bar).child(value)
    });
    let open = Column::new("").width(90.0).unsorted().view(|_, _| button("Open").size(Size::Sm));
    let columns = [Column::new("Name").mono().width(200.0), used, Column::new("Note").muted(), open];
    let cell = |row: usize, column: usize| match column {
        0 => VOLUMES[row].0.to_string(),
        1 => VOLUMES[row].1.to_string(),
        2 => VOLUMES[row].2.to_string(),
        _ => String::new(),
    };
    let opened = signal(None::<usize>);
    let volumes = table(columns, || VOLUMES.len(), cell).on_activate(move |row| opened.set(Some(row)));
    let said = hint(move || match opened.get() {
        Some(row) => format!("Opened {}", VOLUMES[row].0),
        None => "Double click a volume, or press Enter on one".to_string(),
    });
    stack().child(framed(volumes).height(160.0)).child(said)
}

/// Twelve months of figures beside the city they are of: the city stays while the months
/// scroll sideways; the last three are hidden.
fn wide() -> quadra_design::Table {
    const MONTHS: [&str; 12] =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let columns = std::iter::once(Column::new("Thành phố").width(200.0))
        .chain(MONTHS.iter().map(|month| Column::new(*month).numeric().width(96.0)));
    let cell = |row: usize, column: usize| match column {
        0 => CITIES[row % CITIES.len()].to_string(),
        c => (row as u64 * 53 + c as u64 * 17 % 89).to_string(),
    };
    table(columns, || 40, cell).pinned(1).hidden(signal(vec![10, 11, 12]))
}

pub fn tree_page() -> Div {
    let chosen = signal(Some("app.rs".to_string()));
    let open = signal(["quadra", "core", "ui"].map(String::from).into_iter().collect());
    let roots = || {
        let folder = |name: &str| TreeNode::new(name, name).icon(icons::FOLDER);
        let file = |name: &str| TreeNode::new(name, name).icon(icons::FILE);
        // The state of a file under version control: its hue, and a letter that says it.
        let changed = file("window.rs").color(token::HUE_YELLOW).detail("M", token::HUE_YELLOW);
        let added = file("div.rs").color(token::HUE_GREEN).detail("A", token::HUE_GREEN);
        let core = folder("core").children([file("app.rs"), changed]);
        let quadra = folder("quadra").children([core, folder("ui").children([added]), folder("kit")]);
        vec![quadra, folder("docs").children([file("progress.md")])]
    };
    let files = framed(tree(roots, chosen).open(open)).size(260.0, 220.0);
    let said = hint(move || format!("Selected: {}", chosen.get().unwrap_or_default()));
    page(
        "Tree",
        "Nested items such as files. Arrow keys expand, collapse and move.",
        [demo(
            "Explorer",
            line().align_items(Align::Start).child(files).child(said),
            "tree(|| roots, chosen)",
        )],
    )
}

const WORDS: [&str; 17] = [
    "deploy",
    "rollout",
    "pod",
    "node",
    "ready",
    "scaled",
    "restarted",
    "pulled",
    "image",
    "config",
    "map",
    "secret",
    "volume",
    "mounted",
    "probe",
    "failed",
    "passed",
];

/// What the event of a row says: more or fewer words, by its number.
fn feed_text(i: usize) -> String {
    let words = (0..4 + (i * 7919) % 40).map(|k| WORDS[(i + k * 31) % WORDS.len()]);
    words.collect::<Vec<_>>().join(" ")
}

pub fn variable_list_page() -> Div {
    let feed = virtual_list(
        || 100_000,
        52.0,
        signal(0.0),
        |i: Signal<usize>| {
            let number = text(move || format!("#{}", i.get())).color(token::TEXT_3).size(size::TEXT_XS.size);
            let event = column().gap(2.0).padding_x(12.0).padding_y(8.0).border_bottom(1.0);
            event.border_color(token::LINE).child(number).child(text(move || feed_text(i.get())))
        },
    );
    let about = "100,000 rows whose height is their wrapped text. Rows are measured as they appear; \
                 scrolling up never moves what is on screen.";
    page(
        "Variable list",
        about,
        [demo("Event feed", framed(feed).height(360.0), "virtual_list(|| n, 52.0, scroll_y, |i| row)")],
    )
}

pub fn stress_page() -> Div {
    let mut grid = div().grid_fill(96.0).gap(4.0).font_family(fonts::MONO).text_size(size::TEXT_XS.size);
    for i in 0..1000 {
        let cell = row().height(28.0).align_items(Align::Center).justify(Justify::Center).bg(token::PANEL);
        let cell = cell.hover(|s| s.bg(token::ACCENT).text_color(token::TEXT_ON_ACCENT));
        grid = grid.child(cell.child(text(format!("Item {i}"))));
    }
    let about = "1,000 hoverable cells. A hover styles and paints one of them.";
    page("Stress", about, [column().child(grid)])
}
