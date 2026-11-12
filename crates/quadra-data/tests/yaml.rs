//! The YAML reader (specs/13, "Data"): the subset manifests are written in.

use quadra_data::{Value, parse_yaml};

const MANIFEST: &str = "# A deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  labels: {app: web, tier: \"front end\", app.kubernetes.io/name: web}
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: web
          image: registry.local/web:1.4.2
          args: [--port, 8080, --verbose]
          env:
            - {name: DEBUG, value: \"false\"}
          command: &cmd
            - /bin/sh
          script: |
            echo starting # still part of the script

            exec web
      restartPolicy: Always
      tolerations:
      - key: a
        effect: NoSchedule
---
kind: Ignored
";

fn text(value: Option<&Value>) -> &str {
    match value {
        Some(Value::Text(text) | Value::Number(text)) => text,
        other => panic!("{other:?} is not a text or a number"),
    }
}

fn keys(value: &Value) -> Vec<&str> {
    let Value::Map(pairs) = value else { panic!("{value:?} is not a map") };
    pairs.iter().map(|(key, _)| key.as_str()).collect()
}

#[test]
fn a_manifest_is_read() {
    let v = parse_yaml(MANIFEST).unwrap();
    assert_eq!(keys(&v), ["apiVersion", "kind", "metadata", "spec"]);
    assert_eq!(text(v.at(".metadata.labels.tier")), "front end");
    // A key that is not a plain word is in brackets in a path.
    assert_eq!(text(v.at(".metadata.labels[\"app.kubernetes.io/name\"]")), "web");
    assert_eq!(v.at(".spec.replicas"), Some(&Value::Number("3".to_string())));
    let container = v.at(".spec.template.spec.containers[0]").unwrap();
    assert_eq!(keys(container), ["name", "image", "args", "env", "command", "script"]);
    assert_eq!(text(container.get("image")), "registry.local/web:1.4.2");
    let Some(Value::List(args)) = container.get("args") else { panic!("args are a list") };
    assert_eq!(
        args[..],
        [Value::Text("--port".into()), Value::Number("8080".into()), Value::Text("--verbose".into())]
    );
    // In quotes: a text, not a truth.
    assert_eq!(container.at(".env[0].value"), Some(&Value::Text("false".to_string())));
    assert_eq!(text(container.at(".command[0]")), "/bin/sh", "the anchor is skipped");
    assert_eq!(text(container.get("script")), "echo starting # still part of the script\n\nexec web");
    assert_eq!(text(v.at(".spec.template.spec.restartPolicy")), "Always");
    // A sequence at the indentation of its key; and the second document is not read.
    assert_eq!(text(v.at(".spec.template.spec.tolerations[0].effect")), "NoSchedule");
    assert_eq!(text(v.get("kind")), "Deployment");
    assert_eq!((v.at(".spec.nothing"), v.at(".spec.template.spec.containers[7]")), (None, None));
}

#[test]
fn scalars_have_their_kinds_and_what_is_wrong_its_line() {
    let v =
        parse_yaml("a: ~\nb: true\nc: -1.5e3\nd: 'it''s'\ne: 0x1F\nf: 1.2.3\ng: >\n  one\n  two\n").unwrap();
    let Value::Map(pairs) = &v else { panic!("a map") };
    let kinds: Vec<&Value> = pairs.iter().map(|(_, value)| value).collect();
    assert_eq!(*kinds[0], Value::Null);
    assert_eq!(*kinds[1], Value::Bool(true));
    assert_eq!(*kinds[2], Value::Number("-1.5e3".to_string()));
    assert_eq!(*kinds[3], Value::Text("it's".to_string()));
    assert_eq!(*kinds[4], Value::Number("0x1F".to_string()));
    assert_eq!(*kinds[5], Value::Text("1.2.3".to_string()));
    assert_eq!(*kinds[6], Value::Text("one two".to_string()));
    let wrong = parse_yaml("---\na: 1\n    b: 2\n").unwrap_err();
    assert_eq!((wrong.line, wrong.to_string()), (3, "line 3: unexpected indentation".to_string()));
    assert_eq!(parse_yaml("# nothing\n"), Ok(Value::Null));
    assert_eq!(parse_yaml("- 1\n- [a, {b: c}]\n-\n  - x\n").unwrap().summary(), "[3]");
}

#[test]
fn what_no_one_would_write_is_read_to_an_end() {
    // Colons and brackets with nothing between, a quote that is not closed, a backslash at
    // the end, letters of several bytes: each is read to something, and none loops or panics.
    for odd in [
        "a: [: x]",
        "a: {: : :}",
        "a: [\"é\\",
        "a: {\"k\\é\": 'v",
        "[[[[",
        "a: [é, 'ü', {ö: ä}]",
        "- - - -",
        "? x",
        "a:\n  - b:\n  - c",
        "'k' : v\n\"q\": w",
        "k: |\n\n\n",
        "é: ü: ö",
    ] {
        let _ = parse_yaml(odd);
    }
    assert_eq!(parse_yaml("a: [é, 'ü']").unwrap().at(".a[1]"), Some(&Value::Text("ü".to_string())));
    // Nested deeper than anything real, in lines and on one line.
    let deep: String = (0..300).map(|i| format!("{}a:\n", " ".repeat(i))).collect();
    assert_eq!(parse_yaml(&deep).unwrap_err().message, "nested too deep");
    assert!(parse_yaml(&format!("a: {}", "[".repeat(5000))).is_ok());
}
