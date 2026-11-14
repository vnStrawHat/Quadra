//! The diff: two versions of a file, as one column or side by side.

use quadra::{Div, column, row, signal};
use quadra_code::line_colours;
use quadra_kit::{DiffMode, diff, segmented};

use crate::common::{demo, framed, page, stack};

const OLD: &str = "\
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  namespace: shop
  labels:
    app: web
    tier: frontend
spec:
  replicas: 2
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: web
    spec:
      containers:
        - name: web
          image: registry.local/web:1.4.2
          ports:
            - containerPort: 8080
          env:
            - name: LOG_LEVEL
              value: debug
          resources:
            limits:
              cpu: 500m
              memory: 256Mi
          readinessProbe:
            httpGet:
              path: /healthz
              port: 8080
            periodSeconds: 10
      restartPolicy: Always
      terminationGracePeriodSeconds: 30
";

const NEW: &str = "\
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
  namespace: shop
  labels:
    app: web
    tier: frontend
spec:
  replicas: 4
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: web
    spec:
      containers:
        - name: web
          image: registry.local/web:1.5.0
          ports:
            - containerPort: 8080
            - containerPort: 9090
          resources:
            limits:
              cpu: 750m
              memory: 512Mi
          readinessProbe:
            httpGet:
              path: /healthz
              port: 8080
            periodSeconds: 10
      restartPolicy: Always
      terminationGracePeriodSeconds: 30
";

/// A paragraph of a note, before and after: lines longer than half the page.
const NOTE_OLD: &str = "# Rollout

The new build goes to one region first, and to the others once its error rate has stayed under the line for an hour.
Nobody deploys on Fridays.
";
const NOTE_NEW: &str = "# Rollout

The new build goes to one region first, and to the others once its error rate and its latency have stayed under their lines for an hour.
Nobody deploys on Fridays.
";

pub fn diff_page() -> Div {
    let side = signal(0);
    let mode = move || if side.get() == 0 { DiffMode::Inline } else { DiffMode::SideBySide };
    let about = "What differs between two texts, line by line: one column, or the old text beside the \
                 new. A line that was edited shows where; lines far from any change fold into a row \
                 that opens.";
    let modes = row().child(segmented(["Inline", "Side by side"], side));
    page(
        "Diff",
        about,
        [
            demo(
                "A deployment, before and after",
                stack().child(modes).child(framed(
                    column()
                        .height(260.0)
                        .child(diff(OLD, NEW).mode(mode).colours(line_colours("deploy.yaml"))),
                )),
                "diff(old, new).mode(mode).colours(line_colours(\"deploy.yaml\"))",
            ),
            demo(
                "Side by side, folded",
                framed(column().height(300.0).child(diff(OLD, NEW).mode(DiffMode::SideBySide).context(3))),
                "diff(old, new).mode(DiffMode::SideBySide).context(3)",
            ),
            demo(
                "Long lines wrapped",
                framed(
                    column().height(200.0).child(diff(NOTE_OLD, NOTE_NEW).mode(DiffMode::SideBySide).wrap()),
                ),
                "diff(old, new).mode(DiffMode::SideBySide).wrap()",
            ),
        ],
    )
}
