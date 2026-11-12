//! Diagrams: flowcharts and sequence diagrams drawn from the text that describes them.

use quadra::{Div, div};
use quadra_md::markdown;
use quadra_md::mermaid::diagram;

use crate::common::{demo, page};

const FLOW: &str = "graph TD
A[Start] --> B{Is it?}
B -->|Yes| C[OK]
C --> D[Rethink]
D --> B
B ---->|No| E[End]
subgraph work [Work]
C
D
end
F((Hub)) --> A & C & E";

const SHAPES: &str = "flowchart LR
a[Box] --> b(Round) --> c([Stadium]) -.-> d[[Routine]]
a ==> e[(Store)] --o f{Choice} --x g{{Step}}
f -- maybe --> h>Flag] <--> i((End))
g --> g";

const SEQUENCE: &str = "sequenceDiagram
autonumber
actor U as User
participant A as App
participant S as Server
U->>A: Save
A->>S: PUT /note
Note right of S: checks the version
alt saved
S-->>A: 200
A-->>U: Saved
else someone was first
S-->>A: 409
loop until it fits
A->>A: merge
A-)S: PUT /note
end
end
Note over U,A: nothing is lost";

const NOTE: &str = "A build, as its pipeline runs it:

```mermaid
graph LR
src[Sources] --> build(Build) --> test{Pass?}
test -->|yes| ship([Release])
test -.->|no| src
```

A kind that is not drawn shows its source:

```mermaid
pie title Pets
\"Dogs\" : 3
```
";

pub fn diagram_page() -> Div {
    page(
        "Diagram",
        "Flowcharts and sequence diagrams from their text, laid out here and drawn with the theme.",
        [
            demo("Flowchart", diagram(FLOW), "diagram(source)"),
            demo("Shapes and links", diagram(SHAPES), "diagram(source)"),
            demo("Sequence", diagram(SEQUENCE), "diagram(source)"),
            demo("In a document", div().width(560.0).child(markdown(NOTE)), "markdown(source)"),
        ],
    )
}
