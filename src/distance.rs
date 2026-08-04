use crate::types::Point;

/// Mean Earth radius in meters (same constant as the Python/C++ Infostop implementation).
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Which distance function to use for spatial comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricKind {
    #[default]
    Haversine,
    Euclidean,
}

/// Capability: distance between two points.
pub trait DistanceMetric {
    fn distance(&self, a: Point, b: Point) -> f64;
}

/// Great-circle distance in meters (inputs are degrees of latitude/longitude).
#[derive(Debug, Clone, Copy, Default)]
pub struct Haversine;

impl DistanceMetric for Haversine {
    fn distance(&self, a: Point, b: Point) -> f64 {
        let d_lat = (b.x - a.x).to_radians();
        let d_lon = (b.y - a.y).to_radians();
        let lat1 = a.x.to_radians();
        let lat2 = b.x.to_radians();
        let h = (d_lat / 2.0).sin().powi(2)
            + (d_lon / 2.0).sin().powi(2) * lat1.cos() * lat2.cos();
        2.0 * EARTH_RADIUS_M * h.sqrt().asin()
    }
}

/// Planar Euclidean distance (inputs are arbitrary x/y in the same units as `r1`/`r2`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Euclidean;

impl DistanceMetric for Euclidean {
    fn distance(&self, a: Point, b: Point) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Resolve a [`MetricKind`] to a concrete metric object.
pub fn metric_for(kind: MetricKind) -> Box<dyn DistanceMetric> {
    match kind {
        MetricKind::Haversine => Box::new(Haversine),
        MetricKind::Euclidean => Box::new(Euclidean),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn haversine_zero_for_same_point() {
        let p = Point::lat_lon(55.0, 12.0);
        assert!(close(Haversine.distance(p, p), 0.0, 1e-9));
    }

    #[test]
    fn euclidean_unit_square() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!(close(Euclidean.distance(a, b), 5.0, 1e-12));
    }
}
