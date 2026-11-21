//! Live data: the table of rows known by their keys, the log view and the value viewer.

use std::rc::Rc;

use quadra::{Div, column, signal};
use quadra_data::{Log, Value, log_view, parse_json, value_view, yaml_view};
use quadra_design::{CellKind, Column, button, table};

use crate::common::{demo, framed, page, stack};

#[derive(Clone)]
struct Service {
    name: String,
    status: &'static str,
    latency: usize,
    age: usize,
}

const STATES: [&str; 5] = ["Running", "Running", "Pending", "Running", "Failed"];

/// Changes a few services, the same ones at the same step: what a live source would send.
fn update(services: &mut [Service], step: usize) {
    for k in 0..4 {
        let i = (step * 7 + k * 11) % services.len();
        services[i].latency = 10 + (step * 37 + i * 13) % 180;
        services[i].status = STATES[(step + i) % STATES.len()];
    }
    services.iter_mut().for_each(|service| service.age += 60);
}

pub fn live_table_page() -> Div {
    let service = |i: usize| {
        let (name, latency) = (format!("service-{i:02}"), 20 + i * 7 % 90);
        Service { name, status: "Running", latency, age: i * 3700 }
    };
    let (services, step) = (signal((0..40).map(service).collect::<Vec<_>>()), signal(0));
    let columns = [
        Column::new("Service").width(200.0),
        Column::new("Status").width(130.0).kind(CellKind::Status),
        Column::new("Latency (ms)").width(120.0).numeric(),
        Column::new("Age").width(80.0).kind(CellKind::Age),
    ];
    let cell = move |row: usize, column: usize| {
        services.with(|all| match (all.get(row), column) {
            (Some(service), 0) => service.name.clone(),
            (Some(service), 1) => service.status.to_string(),
            (Some(service), 2) => service.latency.to_string(),
            (Some(service), _) => service.age.to_string(),
            (None, _) => String::new(),
        })
    };
    let key = move |row: usize| services.with(|all| all[row].name.clone());
    let rows = table(columns, move || services.with(Vec::len), cell).keys(key);
    let change = button("Update").test_id("live-update").on_click(move || {
        step.update(|step| *step += 1);
        services.update(|all| update(all, step.get_untracked()));
    });
    let about = "Rows identified by key: an update keeps the selection, the keyboard cursor and the \
                 sort, and changed cells flash once. Select a row, sort by latency, then press Update.";
    page(
        "Live table",
        about,
        [demo(
            "Services",
            stack().child(change).child(framed(column().height(320.0).child(rows))),
            "table(columns, rows, cell).keys(|row| ..) // what each row is known by",
        )],
    )
}

/// Lines as a server writes them: requests, with a warning and an error now and then.
fn requests(from: usize, n: usize) -> Vec<(i64, String)> {
    let line = |k: usize| match k % 9 {
        4 => format!("\u{1b}[33mWARN \u{1b}[0m cache miss for key user:{k}"),
        7 => format!("\u{1b}[31mERROR\u{1b}[0m upstream timed out after 3000ms (attempt {})", k % 3 + 1),
        _ => format!("\u{1b}[32mINFO \u{1b}[0m GET /api/v1/items/{} 200 {}ms", k * 7 % 500, k * 13 % 90),
    };
    (from..from + n).map(|k| (1_793_966_400_000 + k as i64 * 250, line(k))).collect()
}

/// Two containers' logs, merged in time order: a level in words, `level=`, or a JSON field.
fn panel_log() -> Log {
    let log = Log::new();
    let at = 1_793_966_400_000;
    log.append_from(
        0,
        [
            (at, "INFO listening on :8080"),
            (at + 1200, "level=warn msg=\"slow response\" ms=812"),
            (at + 2400, r#"{"level":"error","msg":"upstream refused"}"#),
            (at + 3600, "GET /healthz 200"),
        ],
    );
    log.append_from(1, [
        (at + 600, "DEBUG a pool of 8 connections"),
        (at + 3000, "WARN a checkpoint took 4.2 s, longer than the 2 s it is given: the writes of the last minute wait"),
    ]);
    log
}

pub fn log_view_page() -> Div {
    let (log, sent) = (Log::new(), signal(200));
    log.append(requests(0, 200));
    let more = log.clone();
    let add = button("Add 1 000 lines").test_id("log-more").on_click(move || {
        more.append(requests(sent.get_untracked(), 1000));
        sent.update(|sent| *sent += 1000);
    });
    let about = "An append-only log with ANSI colors, timestamps and follow mode: scroll up to stop \
                 following, back to the bottom (or Follow) to resume. The filter hides or dims other \
                 lines; .* makes it a regular expression; Warn lists only warnings and errors.";
    page(
        "Log view",
        about,
        [
            demo(
                "Requests",
                stack().child(add).child(framed(column().height(340.0).child(log_view(&log)))),
                "log.append(lines)  // and log_view(&log) follows",
            ),
            demo(
                "A panel of the app's own: two containers, levels, long lines wrapped",
                column().height(260.0).child(
                    log_view(&panel_log())
                        .bar(false)
                        .text(15.0, 24.0)
                        .ground()
                        .levels()
                        .wrap(true)
                        .header(|| "api, db · current run · 6 lines".to_string()),
                ),
                "log_view(&log).bar(false).ground().levels().wrap(on).header(..)  // log.append_from(source, lines)",
            ),
        ],
    )
}

const VALUE: &str = r#"{"name": "web", "replicas": 3, "ready": true, "selector": null,
 "labels": {"app": "web", "tier": "frontend"},
 "containers": [{"name": "web", "image": "registry.local/web:1.4.2", "ports": [8080, 9090],
   "resources": {"limits": {"cpu": "500m", "memory": "256Mi"}}}]}"#;

pub fn value_viewer_page() -> Div {
    let value = Rc::new(parse_json(VALUE).unwrap_or(Value::Null));
    let about = "A collapsible tree of a JSON or YAML value. Click or press right and left to open and close; ctrl-c or Copy path copies the selected path. The same value as YAML lines that fold copies as YAML.";
    page(
        "Value viewer",
        about,
        [
            demo(
                "Deployment (JSON)",
                framed(column().height(340.0).child(value_view(value.clone()))),
                "value_view(Rc::new(parse_json(text)?))  // or parse_yaml",
            ),
            demo(
                "As YAML lines that fold",
                column().height(300.0).child(yaml_view(value).open(1).omit(["selector"])),
                "yaml_view(value).open(1).omit([\"metadata.managedFields\"])  // to_yaml(&value) to copy",
            ),
        ],
    )
}
