//! Live data: the table of rows known by their keys, and the value viewer.

use std::rc::Rc;

use quadra::{Div, signal};
use quadra_data::{Value, parse_json, value_view};
use quadra_kit::{CellKind, Column, button, table};

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
            stack().child(change).child(framed(rows).height(320.0)),
            "table(columns, rows, cell).keys(|row| ..) // what each row is known by",
        )],
    )
}

const VALUE: &str = r#"{"name": "web", "replicas": 3, "ready": true, "selector": null,
 "labels": {"app": "web", "tier": "frontend"},
 "containers": [{"name": "web", "image": "registry.local/web:1.4.2", "ports": [8080, 9090],
   "resources": {"limits": {"cpu": "500m", "memory": "256Mi"}}}]}"#;

pub fn value_viewer_page() -> Div {
    let value = Rc::new(parse_json(VALUE).unwrap_or(Value::Null));
    let about = "A collapsible tree of a JSON or YAML value. Click or press right and left to open and close; ctrl-c or Copy path copies the selected path.";
    page(
        "Value viewer",
        about,
        [demo(
            "Deployment (JSON)",
            framed(value_view(value)).height(340.0),
            "value_view(Rc::new(parse_json(text)?))  // or parse_yaml",
        )],
    )
}
