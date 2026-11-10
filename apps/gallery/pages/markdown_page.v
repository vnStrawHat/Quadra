// Markdown view page (kit/md).
module pages

import quadra.core
import quadra.kit
import quadra.kit.md
import quadra.ui

const sample_md = "## Release 0.4

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
"

pub struct MarkdownPage {
mut:
	clicked string
}

@[markused]
pub fn (mut p MarkdownPage) render(mut cx core.ViewCx[MarkdownPage]) core.Element {
	t := kit.theme(cx)
	doc := md.view(sample_md).on_link(cx.listener(fn (mut p MarkdownPage, url &string, mut _ core.ViewCx[MarkdownPage]) {
		p.clicked = *url
	}))
	said := if p.clicked == '' { 'Ctrl-click a link' } else { 'Clicked: ${p.clicked}' }
	return page('Markdown',
		'Release notes, READMEs and help text: headings, lists, code, quotes, tables and links.', [
		demo('Release notes', col([ui.div().w(560).child(doc),
			hint(t, said)]), 'md.view(src).on_link(l)'),
	])
}
