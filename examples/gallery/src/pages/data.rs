//! What shows data: the table, the tree, and a thousand cells to move the pointer over.

use quadra::{Align, Div, Justify, column, div, row, signal, text};
use quadra_kit::{Column, TreeNode, fonts, icons, size, table, token, tree};

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
        [demo(
            "1,000,000 rows",
            stack().child(people).child(count),
            "table(columns, || 1_000_000, |row, column| ..)",
        )],
    )
}

pub fn tree_page() -> Div {
    let chosen = signal(None);
    let roots = || {
        let folder = |name: &str| TreeNode::new(name, name).icon(icons::FOLDER);
        let file = |name: &str| TreeNode::new(name, name).icon(icons::FILE);
        let core = folder("core").children([file("app.rs"), file("window.rs")]);
        let quadra =
            folder("quadra").children([core, folder("ui").children([file("div.rs")]), folder("kit")]);
        vec![quadra, folder("docs").children([file("progress.md")])]
    };
    let files = framed(tree(roots, chosen)).size(260.0, 220.0);
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
