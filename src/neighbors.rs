use crate::distance::DistanceMetric;
use crate::types::Point;

/// Capability: find neighbors within a radius for each point.
pub trait NeighborQuery {
    fn neighbors_within(
        &self,
        points: &[Point],
        radius: f64,
        metric: &dyn DistanceMetric,
    ) -> Vec<Vec<usize>>;
}

/// Exact radius search among all pairs (suitable after stay downsampling).
#[derive(Debug, Clone, Copy, Default)]
pub struct BruteForceNeighbors;

impl NeighborQuery for BruteForceNeighbors {
    fn neighbors_within(
        &self,
        points: &[Point],
        radius: f64,
        metric: &dyn DistanceMetric,
    ) -> Vec<Vec<usize>> {
        let n = points.len();
        let mut out = vec![Vec::new(); n];
        for i in 0..n {
            out[i].push(i);
            for j in (i + 1)..n {
                if metric.distance(points[i], points[j]) <= radius {
                    out[i].push(j);
                    out[j].push(i);
                }
            }
        }
        out
    }
}

/// Neighbor lists plus optional pairwise distances (for weighted edges).
pub fn query_neighbors(
    points: &[Point],
    radius: f64,
    metric: &dyn DistanceMetric,
    weighted: bool,
    query: &dyn NeighborQuery,
) -> (Vec<Vec<usize>>, Option<Vec<Vec<f64>>>) {
    let neighbors = query.neighbors_within(points, radius, metric);
    if !weighted {
        return (neighbors, None);
    }

    let distances: Vec<Vec<f64>> = neighbors
        .iter()
        .enumerate()
        .map(|(i, nbrs)| {
            nbrs.iter()
                .map(|&j| {
                    if i == j {
                        0.0
                    } else {
                        metric.distance(points[i], points[j])
                    }
                })
                .collect()
        })
        .collect();
    (neighbors, Some(distances))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distance::Euclidean;

    #[test]
    fn finds_close_pair() {
        let pts = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(100.0, 0.0),
        ];
        let nbrs = BruteForceNeighbors.neighbors_within(&pts, 2.0, &Euclidean);
        assert_eq!(nbrs[0], vec![0, 1]);
        assert_eq!(nbrs[2], vec![2]);
    }
}
