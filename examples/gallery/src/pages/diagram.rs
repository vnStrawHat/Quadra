//! Diagrams: flowcharts drawn from the text that describes them.

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
        "Flowcharts from their text: ranks, links that do not cross where they need not, subgraphs.",
        [
            demo("Flowchart", diagram(FLOW), "diagram(source)"),
            demo("Shapes and links", diagram(SHAPES), "diagram(source)"),
            demo("In a document", div().width(560.0).child(markdown(NOTE)), "markdown(source)"),
        ],
    )
}
