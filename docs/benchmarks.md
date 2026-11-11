# Benchmarks

Measurements of the Rust implementation, newest first, each with its date and the commit it
was taken on. The V implementation's numbers, the baseline of specs/14, are in
[benchmarks-v.md](benchmarks-v.md).

Machine: the development machine (the one of benchmarks-v.md). Release profile at cargo's
defaults until phase R1 fixes it.

## 2026-11-11 — reactive propagation (on 7f18b29 plus the uncommitted `quadra-reactive`)

`tools/cargo.sh run --release -p quadra-reactive --example propagation`

| Shape | Cost |
|---|---|
| 600 signals, one binding each; write all, run all | 93 ns per binding, 56 µs per frame |
| The same through a memo per cell | 121 ns per binding, 73 µs per frame |
| One cell of the 600 changes | 92 ns per change, whatever the other 599 |

A binding here is a deferred effect that reads its source and adds to a counter, so this is
the runtime's own cost: the write, the mark, the notify, the check and the call. It is the
shape of the comparison bench's `grid` (600 cells, all changing every frame), the worst case
for fine-grained updates. For scale, the V implementation spends 1.19 ms of CPU per frame on
`grid` in total, so the reactive layer is about a twentieth of that. What the nodes, layout
and paint add is measured when they exist.
