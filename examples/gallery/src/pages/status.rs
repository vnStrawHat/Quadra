//! The status of a workload: its marks and labels, alone, in a table, smaller and in a row.

use quadra::{Align, Div, column, div, row, text};
use quadra_design::{Status, size, status_dots, status_label, status_label_sized, status_mark, token};

use crate::common::{demo, line, mono, page, stack};

const STATES: [Status; 7] = [
    Status::Running,
    Status::Pending,
    Status::Failed,
    Status::Succeeded,
    Status::Terminating,
    Status::Unknown,
    Status::None,
];

pub fn status_page() -> Div {
    let pods = [
        ("api-7d9f8c6b5-x2k4p", "Running", "0", "3d"),
        ("worker-5c6d7f8-9qwer", "ContainerCreating", "0", "14m"),
        ("migrate-28417-hk2zd", "CrashLoopBackOff", "5", "2m"),
        ("cron-backup-28417-1", "Completed", "0", "1h"),
        ("web-6b7c8d9e0-old01", "Terminating", "1", "6d"),
    ];
    let head = row().align_items(Align::Center).height(28.0).padding_x(token::SPACE_LG).bg(token::PANEL);
    let head = head.border_bottom(1.0).border_color(token::LINE).text_size(size::TEXT_SM.size);
    let head = head.text_color(token::TEXT_2).child(div().width(260.0).child(text("Name")));
    let head = head.child(div().width(200.0).child(text("Status")));
    let head = head.child(div().width(80.0).child(text("Restarts"))).child(text("Age"));
    let mut table = column().border(1.0).border_color(token::LINE).child(head);
    for (i, (name, state, restarts, age)) in pods.into_iter().enumerate() {
        let line = row().align_items(Align::Center).height(28.0).padding_x(token::SPACE_LG);
        let line = line.border_bottom(1.0).border_color(token::LINE).hover(|s| s.bg(token::PANEL_HOVER));
        let line = line.child(div().width(260.0).child(mono(name).text_color(token::TEXT)));
        let line = line.child(div().width(200.0).child(status_label(STATES[i], state)));
        table = table.child(line.child(div().width(80.0).child(mono(restarts))).child(mono(age)));
    }
    let about = "Workload state as a color and a shape, so it reads in grayscale and for color-blind users.";
    page(
        "Status",
        about,
        [
            demo(
                "States",
                line().children(STATES.map(|state| status_label(state, ""))),
                "status_label(Status::Failed, \"CrashLoopBackOff\")",
            ),
            demo("In a table", table, ""),
            demo("Marks only", line().children(STATES.map(status_mark)), "status_mark(Status::Running)"),
            demo(
                "Smaller, and in a row",
                stack()
                    .child(status_dots(
                        [
                            Status::Succeeded,
                            Status::Succeeded,
                            Status::Failed,
                            Status::Succeeded,
                            Status::Succeeded,
                            Status::Succeeded,
                            Status::Failed,
                            Status::Succeeded,
                            Status::Succeeded,
                            Status::Running,
                        ],
                        8.0,
                    ))
                    .child(status_dots([Status::Running, Status::Running, Status::Pending], 10.0))
                    .child(status_label_sized(Status::Failed, "CrashLoopBackOff", 10.0)),
                "status_dots(last_runs, 8.0), status_label_sized(status, words, 10.0)",
            ),
        ],
    )
}
