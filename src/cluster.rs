use crate::infomap::{run_infomap, Network};
use crate::types::{StopLabel, NON_STOP};

/// Capability: cluster a stay-location network into destination labels.
pub trait CommunityDetector {
    fn cluster(
        &self,
        edges: &[(usize, usize, f64)],
        n_nodes: usize,
        label_singleton: bool,
        singleton_nodes: &[usize],
    ) -> Vec<StopLabel>;
}

/// In-crate Infomap-backed community detector.
#[derive(Debug, Clone)]
pub struct InfomapDetector {
    pub seed: u64,
    pub trials: usize,
}

impl Default for InfomapDetector {
    fn default() -> Self {
        Self {
            seed: 42,
            trials: 3,
        }
    }
}

impl CommunityDetector for InfomapDetector {
    fn cluster(
        &self,
        edges: &[(usize, usize, f64)],
        n_nodes: usize,
        label_singleton: bool,
        singleton_nodes: &[usize],
    ) -> Vec<StopLabel> {
        let mut labels = vec![NON_STOP; n_nodes];

        // Map original node index -> dense Infomap index among non-singletons.
        let mut name_map = vec![None; n_nodes];
        let mut inverse = Vec::new();
        let singleton_set: std::collections::HashSet<usize> =
            singleton_nodes.iter().copied().collect();

        for (n, slot) in name_map.iter_mut().enumerate() {
            if !singleton_set.contains(&n) {
                *slot = Some(inverse.len());
                inverse.push(n);
            }
        }

        if !inverse.is_empty() {
            let mut network = Network::new(inverse.len());
            for &(a, b, w) in edges {
                if let (Some(ia), Some(ib)) = (name_map[a], name_map[b]) {
                    network.add_edge(ia, ib, w);
                }
            }
            let modules = run_infomap(&network, self.seed, self.trials);
            for (infomap_idx, module) in modules.into_iter().enumerate() {
                let orig = inverse[infomap_idx];
                // Module ids are dense and tiny relative to StopLabel (i32).
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_possible_wrap
                )]
                {
                    labels[orig] = module as StopLabel;
                }
            }
        }

        if label_singleton {
            let max_label = labels
                .iter()
                .copied()
                .filter(|&l| l >= 0)
                .max()
                .unwrap_or(-1);
            let mut next = max_label + 1;
            for &n in singleton_nodes {
                labels[n] = next;
                next += 1;
            }
        }

        labels
    }
}

/// Build edges from neighbor lists (matching Python Infostop weighting).
pub fn build_edges(
    neighbors: &[Vec<usize>],
    distances: Option<&[Vec<f64>]>,
    counts: &[usize],
    weight_exponent: f64,
) -> (Vec<(usize, usize, f64)>, Vec<usize>) {
    let mut edges = Vec::new();
    let mut singletons = Vec::new();

    for (node, nbrs) in neighbors.iter().enumerate() {
        if nbrs.len() <= 1 {
            singletons.push(node);
            continue;
        }
        for (k, &neighbor) in nbrs.iter().enumerate() {
            if neighbor <= node {
                continue;
            }
            #[allow(clippy::cast_precision_loss)]
            // edge weight from visit counts
            let mut weight = counts[node].max(counts[neighbor]) as f64;
            if let Some(dists) = distances {
                let d = dists[node][k].max(1e-12);
                weight *= d.powf(-weight_exponent);
            }
            edges.push((node, neighbor, weight));
        }
    }

    (edges, singletons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singleton_labeling() {
        let detector = InfomapDetector::default();
        // Two connected nodes + one singleton
        let edges = vec![(0, 1, 1.0)];
        let labels = detector.cluster(&edges, 3, true, &[2]);
        assert!(labels[0] >= 0);
        assert_eq!(labels[0], labels[1]);
        assert!(labels[2] >= 0);
        assert_ne!(labels[2], labels[0]);
    }

    #[test]
    fn singleton_as_non_stop() {
        let detector = InfomapDetector::default();
        let edges = vec![(0, 1, 1.0)];
        let labels = detector.cluster(&edges, 3, false, &[2]);
        assert_eq!(labels[2], NON_STOP);
    }

    #[test]
    fn build_edges_unweighted_uses_max_count() {
        let neighbors = vec![vec![0, 1], vec![1, 0]];
        let counts = vec![3usize, 5];
        let (edges, singletons) = build_edges(&neighbors, None, &counts, 1.0);
        assert!(singletons.is_empty());
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, 0);
        assert_eq!(edges[0].1, 1);
        assert!((edges[0].2 - 5.0).abs() < 1e-12);
    }

    #[test]
    fn build_edges_weighted_scales_by_inverse_distance() {
        let neighbors = vec![vec![0, 1], vec![1, 0]];
        let distances = vec![vec![0.0, 2.0], vec![2.0, 0.0]];
        let counts = vec![1usize, 1];
        let (edges, _) =
            build_edges(&neighbors, Some(&distances), &counts, 1.0);
        assert_eq!(edges.len(), 1);
        // max(count)=1, distance=2, exponent=1 → 1 * 2^(-1) = 0.5
        assert!((edges[0].2 - 0.5).abs() < 1e-12);

        let (edges2, _) =
            build_edges(&neighbors, Some(&distances), &counts, 2.0);
        assert!((edges2[0].2 - 0.25).abs() < 1e-12);
        assert!(edges2[0].2 < edges[0].2);
    }
}
