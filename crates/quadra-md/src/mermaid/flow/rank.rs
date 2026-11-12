//! The layers of a flowchart: a cycle is broken by turning round the links a walk finds
//! going back, a node's rank is the longest way to it, a link that spans ranks is cut into
//! nodes one a rank, and each rank starts in the order a walk from the sources meets it.
//! Nothing here calls itself: a chart is as deep as its source says.

use std::collections::VecDeque;

use super::Flow;

#[derive(Clone, Default)]
pub struct LNode {
    /// A node of the flowchart (the first are those, in their order); not one a long link
    /// passes through.
    pub real: bool,
    pub rank: usize,
    pub order: usize,
    /// Its size across the ranks and along them, and its middle across them.
    pub w: f32,
    pub h: f32,
    pub pos: f32,
    pub up: Vec<usize>,
    pub down: Vec<usize>,
    /// The subgraphs it is in, the outermost first.
    pub path: Vec<usize>,
}

#[derive(Default)]
pub struct Layers {
    pub n: Vec<LNode>,
    /// The nodes of each rank, in order.
    pub rows: Vec<Vec<usize>>,
    /// For each link, its nodes from the end that is higher to the other.
    pub chains: Vec<Vec<usize>>,
    /// For each link, whether it was turned round.
    pub rev: Vec<bool>,
}

/// The links a walk in depth finds going back to a node it is still in.
fn back_edges(f: &Flow) -> Vec<bool> {
    let mut out = vec![Vec::new(); f.nodes.len()];
    for (k, e) in f.edges.iter().enumerate() {
        out[e.from].push(k);
    }
    // 0: not met, 1: the walk is in it, 2: done.
    let (mut state, mut rev) = (vec![0u8; f.nodes.len()], vec![false; f.edges.len()]);
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for start in 0..f.nodes.len() {
        if state[start] == 0 {
            state[start] = 1;
            stack.push((start, 0));
        }
        while let Some(top) = stack.last_mut() {
            let (v, next) = *top;
            top.1 += 1;
            let Some(k) = out[v].get(next) else {
                state[v] = 2;
                stack.pop();
                continue;
            };
            let to = f.edges[*k].to;
            if state[to] == 1 {
                rev[*k] = true;
            } else if state[to] == 0 {
                state[to] = 1;
                stack.push((to, 0));
            }
        }
    }
    rev
}

/// The ends of a link as it is laid out: the higher first.
fn ends(f: &Flow, rev: &[bool], k: usize) -> (usize, usize) {
    let e = &f.edges[k];
    if rev[k] { (e.to, e.from) } else { (e.from, e.to) }
}

/// The rank of each node. A link spans twice what it asks for, so that the rank in its
/// middle can hold its label; a source is then moved down beside what it leads to.
fn ranks(f: &Flow, rev: &[bool]) -> Vec<usize> {
    let count = f.nodes.len();
    let (mut waits, mut rank) = (vec![0usize; count], vec![0usize; count]);
    let (mut succ, mut fed) = (vec![Vec::new(); count], vec![false; count]);
    for k in (0..f.edges.len()).filter(|k| f.edges[*k].from != f.edges[*k].to) {
        let (a, b) = ends(f, rev, k);
        succ[a].push(k);
        (waits[b], fed[b]) = (waits[b] + 1, true);
    }
    let mut queue: VecDeque<usize> = (0..count).filter(|v| waits[*v] == 0).collect();
    let mut sorted = Vec::with_capacity(count);
    while let Some(v) = queue.pop_front() {
        sorted.push(v);
        for k in &succ[v] {
            let to = ends(f, rev, *k).1;
            rank[to] = rank[to].max(rank[v] + 2 * f.edges[*k].span);
            waits[to] -= 1;
            if waits[to] == 0 {
                queue.push_back(to);
            }
        }
    }
    for v in sorted.into_iter().rev().filter(|v| !fed[*v]) {
        let next = succ[v].iter().map(|k| rank[ends(f, rev, *k).1].saturating_sub(2 * f.edges[*k].span));
        rank[v] = next.min().unwrap_or(rank[v]);
    }
    let low = rank.iter().copied().min().unwrap_or(0);
    rank.into_iter().map(|r| r - low).collect()
}

/// How many nodes the long links of a chart are cut into: what its layout costs.
pub fn dummies(f: &Flow) -> usize {
    let rev = back_edges(f);
    let rank = ranks(f, &rev);
    let between = |k: usize| {
        let (a, b) = ends(f, &rev, k);
        rank[b].saturating_sub(rank[a] + 1)
    };
    (0..f.edges.len()).filter(|k| f.edges[*k].from != f.edges[*k].to).map(between).sum()
}

/// The subgraphs a subgraph is in, the outermost first, and itself.
fn path(f: &Flow, sub: Option<usize>) -> Vec<usize> {
    let mut path: Vec<usize> = std::iter::successors(sub, |sub| f.subs[*sub].parent).collect();
    path.reverse();
    path
}

/// How many subgraphs two nodes are in together.
pub fn common(a: &[usize], b: &[usize]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| a == b).count()
}

/// The layered chart; the sizes of its nodes are for the caller to set.
pub fn layers(f: &Flow) -> Layers {
    let rev = back_edges(f);
    let rank = ranks(f, &rev);
    let mut g = Layers { chains: vec![Vec::new(); f.edges.len()], ..Layers::default() };
    for (v, node) in f.nodes.iter().enumerate() {
        g.n.push(LNode { real: true, rank: rank[v], path: path(f, node.sub), ..LNode::default() });
    }
    for k in (0..f.edges.len()).filter(|k| f.edges[*k].from != f.edges[*k].to) {
        let (a, b) = ends(f, &rev, k);
        // A node of a link is in the subgraphs both its ends are in.
        let shared = g.n[a].path[..common(&g.n[a].path, &g.n[b].path)].to_vec();
        let mut chain = vec![a];
        for r in rank[a] + 1..rank[b] {
            chain.push(g.n.len());
            g.n.push(LNode { rank: r, path: shared.clone(), ..LNode::default() });
        }
        chain.push(b);
        for pair in chain.windows(2) {
            g.n[pair[0]].down.push(pair[1]);
            g.n[pair[1]].up.push(pair[0]);
        }
        g.chains[k] = chain;
    }
    g.rows = vec![Vec::new(); rank.iter().copied().max().unwrap_or(0) + 1];
    g.rev = rev;
    // The first order: a walk in depth from the nodes nothing leads to, in the order of the
    // source, then from whatever is left.
    let mut seen = vec![false; g.n.len()];
    let sources = (0..f.nodes.len()).filter(|v| g.n[*v].up.is_empty());
    for start in sources.chain(0..g.n.len()).collect::<Vec<_>>() {
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            if std::mem::replace(&mut seen[v], true) {
                continue;
            }
            g.n[v].order = g.rows[g.n[v].rank].len();
            g.rows[g.n[v].rank].push(v);
            stack.extend(g.n[v].down.iter().rev());
        }
    }
    g
}
