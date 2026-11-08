// Pixel tests: a realistic flowchart and sequence diagram drawn on the GPU (WARP), saved
// to .tmp/out for review.
module mermaid

import os
import quadra.core
import quadra.prim
import quadra.kit
import quadra.testing
import quadra.ui

const flow_src = 'flowchart TD
    start([Request arrives]) --> auth{Authenticated?}
    auth -->|No| login[Show login page]
    login --> creds[/Enter credentials/]
    creds --> auth
    auth -->|Yes| cache{In cache?}
    subgraph backend [Backend]
        cache -->|Hit| render[Render page]
        cache -->|Miss| db[(Database)]
        db --> render
    end
    render --> log[[Write access log]]
    render --> send((Send))
    send -.-> metrics>Metrics]
    log ==> done([Done])
    send --> done'

const seq_src = 'sequenceDiagram
    autonumber
    actor U as User
    participant W as Web app
    participant A as Auth service
    participant D as Database
    U->>W: Open dashboard
    W->>A: Validate session
    alt session valid
        A-->>W: OK
    else expired
        A-->>W: 401
        W->>U: Redirect to login
    end
    loop every 30 s
        W->>D: Query metrics
        D-->>W: Rows
    end
    Note over W,D: Results are cached for 30 s
    W->>W: Render charts
    W-)U: Push update
    Note right of U: Sees live data'

pub struct Shot {
	src string
}

@[markused]
pub fn (mut v Shot) render(mut cx core.ViewCx[Shot]) core.Element {
	return ui.div().size_full().p(16).child(diagram(v.src))
}

fn shot(src string, name string, w int, h int) !prim.Size {
	mut hh := testing.new_pixel_harness()!
	kit.install(mut hh.app)!
	hh.open(Shot{src}, width: w, height: h)!
	dir := os.join_path(os.dir(@FILE), '..', '..', '..', '..', '.tmp', 'out')
	os.mkdir_all(dir)!
	hh.save_png(os.join_path(dir, name))!
	d := diagram(src)
	assert d.error() == none
	return d.size()
}

fn test_flowchart_pixels() {
	shot(flow_src, 'mermaid_flow.png', 760, 900)!
}

fn test_sequence_pixels() {
	shot(seq_src, 'mermaid_seq.png', 900, 820)!
}

fn test_left_right_pixels() {
	src := 'graph LR
    A[Edit] --> B{Valid?}
    B -->|yes| C[Save]
    B -->|no| D[Show errors]
    D --> A
    subgraph store [Storage]
        C --> E[(Disk)]
        C --> F[(Cloud)]
    end
    E & F --> G([Synced])'
	shot(src, 'mermaid_flow_lr.png', 900, 420)!
}
