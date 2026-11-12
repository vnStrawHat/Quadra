//! Markdown: a document rendered, with links that say where they lead.

use quadra::{Div, column, div, signal};
use quadra_md::markdown;

use crate::common::{demo, hint, page};

const SAMPLE: &str = "## Release 0.4

Rolling restarts now wait for **readiness probes** before moving on. See the
[upgrade notes](https://example.com/upgrade) and run `kubectl rollout status`.

- Faster log search: *regex* filters run off the UI thread
- Pod table keeps the selection across refreshes
  - also when the pod is renamed
1. Back up your config
2. Upgrade

```v
fn main() {
	println('hello') // greet
}
```

> Breaking: the `--legacy` flag is gone.

| Setting | Default | Notes |
|---|---|---|
| `timeout` | 30 s | per probe |
| `retries` | 3 | then the pod restarts |
";

pub fn markdown_page() -> Div {
    let clicked = signal(String::new());
    let doc = markdown(SAMPLE).on_link(move |target| clicked.set(target.to_string()));
    let said = hint(move || match clicked.get() {
        target if target.is_empty() => "Ctrl-click a link".to_string(),
        target => format!("Clicked: {target}"),
    });
    page(
        "Markdown",
        "Release notes, READMEs and help text: headings, lists, code, quotes, tables and links.",
        [demo(
            "Release notes",
            column().gap(12.0).child(div().width(560.0).child(doc)).child(said),
            "markdown(source).on_link(|target| ..)",
        )],
    )
}
