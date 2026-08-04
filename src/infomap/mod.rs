//! Two-level Infomap (map equation) for undirected weighted networks.
//!
//! Implements greedy node moves that minimize the map equation description length,
//! suitable for Infostop's stay-location graphs. No external Infomap dependency.

use std::collections::HashMap;

/// Undirected weighted adjacency list.
#[derive(Debug, Clone)]
pub struct Network {
    n: usize,
    /// For each node: (neighbor, weight). Undirected edges stored on both sides.
    adj: Vec<Vec<(usize, f64)>>,
    /// Node strength (weighted degree).
    strength: Vec<f64>,
    /// Sum of all undirected edge weights (each edge counted once).
    total_weight: f64,
}

impl Network {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            adj: vec![Vec::new(); n],
            strength: vec![0.0; n],
            total_weight: 0.0,
        }
    }

    pub fn add_edge(&mut self, a: usize, b: usize, weight: f64) {
        if a == b || weight <= 0.0 {
            return;
        }
        self.adj[a].push((b, weight));
        self.adj[b].push((a, weight));
        self.strength[a] += weight;
        self.strength[b] += weight;
        self.total_weight += weight;
    }
}

/// Simple xorshift64 RNG for deterministic Infomap trials.
#[derive(Debug, Clone)]
struct Rng64 {
    state: u64,
}

impl Rng64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed | 1, // avoid zero state
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn gen_range(&mut self, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        (self.next_u64() as usize) % len
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.gen_range(i + 1);
            items.swap(i, j);
        }
    }
}

/// Run two-level Infomap; returns module id per node (0-based, dense among used modules).
pub fn run_infomap(network: &Network, seed: u64, trials: usize) -> Vec<usize> {
    if network.n == 0 {
        return Vec::new();
    }
    if network.total_weight <= 0.0 {
        // No edges: each node its own module.
        return (0..network.n).collect();
    }

    let mut best_partition = (0..network.n).collect::<Vec<_>>();
    let mut best_l = f64::INFINITY;
    let mut rng = Rng64::new(seed);

    for _ in 0..trials.max(1) {
        let mut partition = (0..network.n).collect::<Vec<_>>();
        optimize_partition(network, &mut partition, &mut rng);
        let l = map_equation(network, &partition);
        if l < best_l {
            best_l = l;
            best_partition = partition;
        }
    }

    relabel_dense(&best_partition)
}

fn optimize_partition(
    network: &Network,
    partition: &mut [usize],
    rng: &mut Rng64,
) {
    let n = network.n;
    let mut improved = true;
    let mut rounds = 0usize;
    const MAX_ROUNDS: usize = 100;

    while improved && rounds < MAX_ROUNDS {
        improved = false;
        rounds += 1;

        let mut order: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut order);

        for &node in &order {
            let current = partition[node];
            let mut candidates: Vec<usize> = network.adj[node]
                .iter()
                .map(|&(nbr, _)| partition[nbr])
                .collect();
            candidates.push(current);
            candidates.sort_unstable();
            candidates.dedup();

            let mut best_module = current;
            let mut best_delta = 0.0;
            let base = map_equation(network, partition);

            for &cand in &candidates {
                if cand == current {
                    continue;
                }
                partition[node] = cand;
                let l = map_equation(network, partition);
                let delta = l - base;
                if delta < best_delta - 1e-15 {
                    best_delta = delta;
                    best_module = cand;
                }
            }
            partition[node] = best_module;
            if best_module != current {
                improved = true;
            }
        }

        // Create new empty modules for nodes that benefit from isolation.
        for &node in &order {
            let current = partition[node];
            let alone = next_free_module(partition);
            let base = map_equation(network, partition);
            partition[node] = alone;
            let l = map_equation(network, partition);
            if l < base - 1e-15 {
                improved = true;
            } else {
                partition[node] = current;
            }
        }
    }
}

fn next_free_module(partition: &[usize]) -> usize {
    partition.iter().copied().max().unwrap_or(0) + 1
}

/// Map equation L(M) for an undirected weighted network.
fn map_equation(network: &Network, partition: &[usize]) -> f64 {
    let two_m = 2.0 * network.total_weight;
    if two_m <= 0.0 {
        return 0.0;
    }

    // Module -> nodes
    let mut modules: HashMap<usize, Vec<usize>> = HashMap::new();
    for (node, &m) in partition.iter().enumerate() {
        modules.entry(m).or_default().push(node);
    }

    let mut q_exit = 0.0;
    let mut term2 = 0.0;

    for nodes in modules.values() {
        let mut p_module = 0.0;
        let mut exit = 0.0;
        for &u in nodes {
            p_module += network.strength[u] / two_m;
            for &(v, w) in &network.adj[u] {
                if partition[v] != partition[u] {
                    exit += w;
                }
            }
        }
        // Each undirected cut edge counted twice in the loop above.
        let q = exit / two_m;
        q_exit += q;

        // Within-module entropy contribution: exit code + node visit codes.
        let mut plogp_sum = plogp(q);
        for &u in nodes {
            plogp_sum += plogp(network.strength[u] / two_m);
        }
        let p_circle = p_module + q;
        term2 += plogp(p_circle) - plogp_sum;
    }

    // L = q↷ H(Q) + Σ (p↷^i H(P^i))
    // H(Q) = -Σ (q_i / q↷) log(q_i / q↷) = ( -Σ plogp(q_i) + plogp(q↷) ) / q↷ ... use standard form:
    // q↷ H(Q) = plogp(q↷) - Σ plogp(q_i)
    let mut sum_plogp_q = 0.0;
    for nodes in modules.values() {
        let mut exit = 0.0;
        for &u in nodes {
            for &(v, w) in &network.adj[u] {
                if partition[v] != partition[u] {
                    exit += w;
                }
            }
        }
        let q = exit / two_m;
        sum_plogp_q += plogp(q);
    }

    let term1 = plogp(q_exit) - sum_plogp_q;
    term1 + term2
}

fn plogp(p: f64) -> f64 {
    if p > 0.0 {
        p * p.ln()
    } else {
        0.0
    }
}

fn relabel_dense(partition: &[usize]) -> Vec<usize> {
    let mut map = HashMap::new();
    let mut next = 0usize;
    let mut out = Vec::with_capacity(partition.len());
    for &m in partition {
        let id = *map.entry(m).or_insert_with(|| {
            let id = next;
            next += 1;
            id
        });
        out.push(id);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_two_triangles() {
        let mut net = Network::new(6);
        // Triangle A
        net.add_edge(0, 1, 1.0);
        net.add_edge(1, 2, 1.0);
        net.add_edge(2, 0, 1.0);
        // Triangle B
        net.add_edge(3, 4, 1.0);
        net.add_edge(4, 5, 1.0);
        net.add_edge(5, 3, 1.0);
        // Weak bridge
        net.add_edge(2, 3, 0.05);

        let labels = run_infomap(&net, 42, 5);
        assert_eq!(labels.len(), 6);
        // Nodes 0,1,2 should share a module different from 3,4,5
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[1], labels[2]);
        assert_eq!(labels[3], labels[4]);
        assert_eq!(labels[4], labels[5]);
        assert_ne!(labels[0], labels[3]);
    }
}
