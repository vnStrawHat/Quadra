//! YAML written and shown (specs/13, "Data"): `to_yaml` reads back as the same value, plain
//! where it can be; `yaml_view` folds its lines, leaves paths out and copies the YAML.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_data::{Value, parse_json, parse_yaml, to_yaml, yaml_view};
use quadra_design::{dark, token};
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;

const POD: &str = r#"{"apiVersion": "v1", "kind": "Pod",
 "metadata": {"name": "web-5d8b", "managedFields": [{"manager": "kubectl"}],
   "labels": {"app.kubernetes.io/name": "web", "tier": "front end"}},
 "spec": {"containers": [{"name": "web", "image": "nginx:1.27", "ports": [{"containerPort": 80}],
   "args": ["--port", "80", "yes", "", "a: b", "x #y", "two\nlines", "null"]},
   {"resources": {}, "name": "side"}], "volumes": [], "nodeSelector": {}},
 "status": {"phase": "Running", "ready": true, "restarts": 0, "ip": null}}"#;

#[test]
fn yaml_is_written_plain_where_it_can_be_and_reads_back_the_same() {
    let pod = parse_json(POD).unwrap();
    let yaml = to_yaml(&pod);
    assert_eq!(parse_yaml(&yaml).unwrap(), pod, "{yaml}");
    for line in [
        "apiVersion: v1\n",
        "  labels:\n    app.kubernetes.io/name: web\n    tier: front end\n",
        "    - name: web\n      image: nginx:1.27\n      ports:\n        - containerPort: 80\n",
        "        - \"--port\"\n        - \"80\"\n        - \"yes\"\n        - \"\"\n        - \"a: b\"\n",
        "        - \"x #y\"\n        - \"two\\nlines\"\n        - \"null\"\n",
        "    - resources: {}\n      name: side\n",
        "  volumes: []\n  nodeSelector: {}\n",
        "  ready: true\n  restarts: 0\n  ip: null\n",
    ] {
        assert!(yaml.contains(line), "{line:?} is not in\n{yaml}");
    }
    let lists = Value::List(vec![Value::List(vec![Value::Number("1".into())]), Value::Map(vec![])]);
    assert_eq!(to_yaml(&lists), "- - 1\n- {}\n");
    assert_eq!(parse_yaml(&to_yaml(&lists)).unwrap(), lists);
    // Left out, with or without the first dot; what holds things, by levels.
    let lean = pod.without(&["metadata.managedFields", ".spec.containers[1]"]);
    assert!(lean.at(".metadata.managedFields").is_none() && lean.at(".spec.containers[1]").is_none());
    assert_eq!(lean.at(".metadata.name"), pod.at(".metadata.name"));
    let top: HashSet<String> = [".metadata", ".spec", ".status"].map(String::from).into();
    assert_eq!(pod.holders(1), top);
}

/// The names of the lines that show, from the top.
fn lines(h: &Harness) -> Vec<String> {
    let rows = h.query(|n| n.semantics.role == Role::TreeItem);
    let mut all: Vec<(f32, String)> = Vec::new();
    for row in rows {
        let name = h.tree.semantic_tree().nodes.into_iter().find(|n| h.tree.node_of(n.id) == Some(row));
        all.push((h.tree.window_rect(row).y, name.unwrap().semantics.name));
    }
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    all.into_iter().map(|(_, label)| label).collect()
}

#[test]
fn yaml_lines_fold_leave_paths_out_and_copy_as_yaml() {
    let pod = Rc::new(parse_json(POD).unwrap());
    let open = signal(HashSet::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    let shown = pod.clone();
    h.mount(move || {
        let view = yaml_view(shown.clone()).omit(["metadata.managedFields"]).folds(open);
        quadra_design::root().child(view)
    });
    h.frame();
    // The top open, the rest folded to what it holds; the left-out path nowhere.
    let top = ["apiVersion:", "kind:", "metadata:", "name:", "labels:", "spec:", "containers:"];
    let rest = ["volumes:", "nodeSelector:", "status:", "phase:", "ready:", "restarts:", "ip:"];
    assert_eq!(lines(&h), [&top[..], &rest[..]].concat());
    for said in ["v1", "web-5d8b", "{2}", "[2]", "[]", "{}", "Running", "true", "null"] {
        assert!(h.has_text(said), "{said} is not said:\n{}", h.semantics());
    }
    assert!(!h.has_text("{3}") && !h.has_text("managedFields:"), "{}", h.semantics());
    // A key in the colour of properties, a text in that of strings; 24 px lines.
    let theme = dark().theme();
    let key = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "kind:")[0];
    let value = h.get_by_text("Pod").node();
    assert_eq!(h.tree.inherited(key).color, theme.resolve(token::SYNTAX_PROPERTY.into()));
    assert_eq!(h.tree.inherited(value).color, theme.resolve(token::SYNTAX_STRING.into()));
    let row = h.query(|n| n.semantics.role == Role::TreeItem && n.semantics.name == "kind:")[0];
    assert_eq!(h.tree.window_rect(row).height, 24.0);
    // Unfolded all: a map in a list on the dash's line, the rest under it 18 px in.
    open.set(pod.holders(usize::MAX));
    h.frame();
    let all = lines(&h);
    let at = all.iter().position(|l| l == "- name:").expect("a map in a list on the dash's line");
    assert_eq!(&all[at..at + 3], ["- name:", "image:", "ports:"]);
    assert!(all.contains(&"- containerPort:".to_string()) && all.contains(&"-".to_string()));
    // Folded: only the top keys.
    open.set(HashSet::new());
    h.frame();
    assert_eq!(lines(&h), ["apiVersion:", "kind:", "metadata:", "spec:", "status:"]);
    // The key for copying, with the focus in the lines: the YAML, without what is left out.
    h.click(100.0, 12.0);
    h.key(Key::C, Mods::CTRL);
    let copied = h.clipboard.read_text().unwrap();
    assert_eq!(copied, to_yaml(&pod.without(&["metadata.managedFields"])));
}
