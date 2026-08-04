//! Two-level Infomap (map equation) for undirected weighted networks.
//!
//! Greedy node moves minimize the map equation description length using
//! **incremental** ΔL updates (no full recomputation per candidate). Suitable
//! for Infostop stay-location graphs.
//!
//! # Limits versus reference Infomap
//!
//! This is a simplified two-level undirected optimizer: no multilevel
//! aggregation and no teleportation. Partitions can diverge from mapequation /
//! upstream Infomap on hard graphs. No external Infomap dependency.

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
        if let Some((_, w)) = self.adj[a].iter_mut().find(|(n, _)| *n == b) {
            *w += weight;
            if let Some((_, w_rev)) = self.adj[b].iter_mut().find(|(n, _)| *n == a) {
                *w_rev += weight;
            }
        } else {
            self.adj[a].push((b, weight));
            self.adj[b].push((a, weight));
        }
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

#[derive(Debug, Clone, Copy, Default)]
struct ModuleStats {
    /// Raw exit weight (sum of edge weights from inside to outside).
    exit: f64,
    /// Sum of node strengths in the module.
    strength: f64,
    n_nodes: usize,
}

/// Incremental map-equation bookkeeping for a partition.
#[derive(Debug, Clone)]
struct MapState {
    partition: Vec<usize>,
    modules: HashMap<usize, ModuleStats>,
    two_m: f64,
    /// Σ_u plogp(strength[u] / two_m); partition-invariant.
    node_plogp_sum: f64,
    q_exit: f64,
    description_length: f64,
}

impl MapState {
    fn from_partition(network: &Network, partition: &[usize]) -> Self {
        let two_m = 2.0 * network.total_weight;
        let mut modules: HashMap<usize, ModuleStats> = HashMap::new();
        let mut node_plogp_sum = 0.0;

        for (node, &m) in partition.iter().enumerate() {
            let s = network.strength[node];
            if two_m > 0.0 {
                node_plogp_sum += plogp(s / two_m);
            }
            let stats = modules.entry(m).or_default();
            stats.strength += s;
            stats.n_nodes += 1;
            for &(v, w) in &network.adj[node] {
                if partition[v] != m {
                    stats.exit += w;
                }
            }
        }

        let mut state = Self {
            partition: partition.to_vec(),
            modules,
            two_m,
            node_plogp_sum,
            q_exit: 0.0,
            description_length: 0.0,
        };
        state.recompute_l();
        state
    }

    fn recompute_l(&mut self) {
        if self.two_m <= 0.0 {
            self.q_exit = 0.0;
            self.description_length = 0.0;
            return;
        }

        let mut q_exit = 0.0;
        let mut sum_plogp_q = 0.0;
        let mut term2 = 0.0;

        for stats in self.modules.values() {
            if stats.n_nodes == 0 {
                continue;
            }
            let q = stats.exit / self.two_m;
            let p = stats.strength / self.two_m;
            q_exit += q;
            sum_plogp_q += plogp(q);
            // Within-module: exit code + node codes (node codes summed globally below).
            term2 += plogp(p + q) - plogp(q);
        }

        // Subtract all node plogp once (matches full map_equation).
        term2 -= self.node_plogp_sum;
        let term1 = plogp(q_exit) - sum_plogp_q;
        self.q_exit = q_exit;
        self.description_length = term1 + term2;
    }

    /// Weight from `node` into modules A and B and everywhere else.
    fn link_weights(
        &self,
        network: &Network,
        node: usize,
        module_a: usize,
        module_b: usize,
    ) -> (f64, f64, f64) {
        let mut to_a = 0.0;
        let mut to_b = 0.0;
        let mut to_other = 0.0;
        for &(v, w) in &network.adj[node] {
            let m = self.partition[v];
            if m == module_a {
                to_a += w;
            } else if m == module_b {
                to_b += w;
            } else {
                to_other += w;
            }
        }
        (to_a, to_b, to_other)
    }

    /// ΔL if `node` moves from its current module to `dest` (may be new/empty).
    fn delta_move(&self, network: &Network, node: usize, dest: usize) -> f64 {
        let src = self.partition[node];
        if src == dest {
            return 0.0;
        }

        let (w_src, w_dest, w_other) = self.link_weights(network, node, src, dest);
        let s_u = network.strength[node];

        let src_stats = self.modules.get(&src).copied().unwrap_or_default();
        let dest_stats = self.modules.get(&dest).copied().unwrap_or_default();

        // Exit updates (see module docs in optimize path).
        let delta_exit_src = -w_dest - w_other + w_src;
        let delta_exit_dest = w_src + w_other - w_dest;

        let src_exit2 = src_stats.exit + delta_exit_src;
        let dest_exit2 = dest_stats.exit + delta_exit_dest;
        let src_str2 = src_stats.strength - s_u;
        let dest_str2 = dest_stats.strength + s_u;
        let src_n2 = src_stats.n_nodes - 1;
        let dest_n2 = dest_stats.n_nodes + 1;

        let q_exit2 = self.q_exit + (delta_exit_src + delta_exit_dest) / self.two_m;

        module_pair_delta_l(
            self.two_m,
            self.q_exit,
            q_exit2,
            src_stats,
            ModuleStats {
                exit: src_exit2,
                strength: src_str2,
                n_nodes: src_n2,
            },
            dest_stats,
            ModuleStats {
                exit: dest_exit2,
                strength: dest_str2,
                n_nodes: dest_n2,
            },
        )
    }

    fn apply_move(&mut self, network: &Network, node: usize, dest: usize) {
        let src = self.partition[node];
        if src == dest {
            return;
        }

        let (w_src, w_dest, w_other) = self.link_weights(network, node, src, dest);
        let s_u = network.strength[node];
        let delta_exit_src = -w_dest - w_other + w_src;
        let delta_exit_dest = w_src + w_other - w_dest;

        {
            let src_stats = self.modules.entry(src).or_default();
            src_stats.exit += delta_exit_src;
            src_stats.strength -= s_u;
            src_stats.n_nodes -= 1;
        }
        if self.modules.get(&src).is_some_and(|s| s.n_nodes == 0) {
            self.modules.remove(&src);
        }

        {
            let dest_stats = self.modules.entry(dest).or_default();
            dest_stats.exit += delta_exit_dest;
            dest_stats.strength += s_u;
            dest_stats.n_nodes += 1;
        }

        self.partition[node] = dest;
        self.q_exit += (delta_exit_src + delta_exit_dest) / self.two_m;
        self.recompute_l();
    }

    fn next_free_module(&self) -> usize {
        self.modules.keys().copied().max().unwrap_or(0) + 1
    }
}

/// Change in L from updating modules src/dest and q_exit (node plogp constant cancels).
fn module_pair_delta_l(
    two_m: f64,
    q_exit: f64,
    q_exit2: f64,
    src_before: ModuleStats,
    src_after: ModuleStats,
    dest_before: ModuleStats,
    dest_after: ModuleStats,
) -> f64 {
    fn module_terms(two_m: f64, stats: ModuleStats) -> f64 {
        if stats.n_nodes == 0 || two_m <= 0.0 {
            return 0.0;
        }
        let q = stats.exit / two_m;
        let p = stats.strength / two_m;
        // Contribution inside Σ: -plogp(q) + plogp(p+q) for term2, and -plogp(q) for term1
        // => -2 plogp(q) + plogp(p+q)
        -2.0 * plogp(q) + plogp(p + q)
    }

    let before = plogp(q_exit)
        + module_terms(two_m, src_before)
        + module_terms(two_m, dest_before);
    let after = plogp(q_exit2)
        + module_terms(two_m, src_after)
        + module_terms(two_m, dest_after);
    after - before
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
    let mut state = MapState::from_partition(network, partition);
    let mut improved = true;
    let mut rounds = 0usize;
    const MAX_ROUNDS: usize = 100;

    while improved && rounds < MAX_ROUNDS {
        improved = false;
        rounds += 1;

        let mut order: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut order);

        for &node in &order {
            let current = state.partition[node];
            let mut candidates: Vec<usize> = network.adj[node]
                .iter()
                .map(|&(nbr, _)| state.partition[nbr])
                .collect();
            candidates.push(current);
            candidates.sort_unstable();
            candidates.dedup();

            let mut best_module = current;
            let mut best_delta = 0.0;

            for &cand in &candidates {
                if cand == current {
                    continue;
                }
                let delta = state.delta_move(network, node, cand);
                if delta < best_delta - 1e-15 {
                    best_delta = delta;
                    best_module = cand;
                }
            }

            if best_module != current {
                state.apply_move(network, node, best_module);
                improved = true;
            }
        }

        // Isolation: move to a fresh empty module when that lowers L.
        for &node in &order {
            let alone = state.next_free_module();
            let delta = state.delta_move(network, node, alone);
            if delta < -1e-15 {
                state.apply_move(network, node, alone);
                improved = true;
            }
        }
    }

    partition.copy_from_slice(&state.partition);
}

/// Map equation L(M) for an undirected weighted network.
fn map_equation(network: &Network, partition: &[usize]) -> f64 {
    MapState::from_partition(network, partition).description_length
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

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn add_edge_merges_duplicate_pairs() {
        let mut net = Network::new(2);
        net.add_edge(0, 1, 1.0);
        net.add_edge(0, 1, 2.0);

        assert_eq!(net.adj[0].len(), 1);
        assert_eq!(net.adj[1].len(), 1);
        assert!(close(net.adj[0][0].1, 3.0, 1e-12));
        assert!(close(net.adj[1][0].1, 3.0, 1e-12));
        assert_eq!(net.adj[0][0].0, 1);
        assert_eq!(net.adj[1][0].0, 0);
        assert!(close(net.strength[0], 3.0, 1e-12));
        assert!(close(net.strength[1], 3.0, 1e-12));
        assert!(close(net.total_weight, 3.0, 1e-12));
    }

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

    #[test]
    fn incremental_delta_matches_full_map_equation() {
        let mut net = Network::new(8);
        let edges = [
            (0, 1, 1.0),
            (1, 2, 1.2),
            (2, 0, 0.8),
            (3, 4, 1.0),
            (4, 5, 1.0),
            (5, 3, 1.0),
            (2, 3, 0.1),
            (6, 7, 2.0),
            (0, 6, 0.05),
        ];
        for &(a, b, w) in &edges {
            net.add_edge(a, b, w);
        }

        let mut partition: Vec<usize> = (0..net.n).collect();
        let mut state = MapState::from_partition(&net, &partition);
        assert!(close(
            state.description_length,
            map_equation(&net, &partition),
            1e-12
        ));

        let mut rng = Rng64::new(7);
        for _ in 0..40 {
            let node = rng.gen_range(net.n);
            let dest = if rng.gen_range(2) == 0 {
                state.partition[rng.gen_range(net.n)]
            } else {
                state.next_free_module()
            };
            let src = state.partition[node];
            if src == dest {
                continue;
            }

            let delta = state.delta_move(&net, node, dest);
            let l_before = map_equation(&net, &state.partition);
            state.apply_move(&net, node, dest);
            partition.copy_from_slice(&state.partition);
            let l_after = map_equation(&net, &partition);
            assert!(
                close(delta, l_after - l_before, 1e-9),
                "delta {delta} vs actual {} (L {l_before} -> {l_after})",
                l_after - l_before
            );
            assert!(close(state.description_length, l_after, 1e-9));
        }
    }
}
