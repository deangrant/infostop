use crate::distance::DistanceMetric;
use crate::types::{Point, TimedPoint, NON_STOP};

/// Result of sequential stay detection for one trajectory.
#[derive(Debug, Clone)]
pub struct StayEvents {
    /// Median coordinates of each accepted stay.
    pub medians: Vec<Point>,
    /// For each input point: stay index, or `NON_STOP` if not part of an accepted stay.
    pub event_map: Vec<i32>,
}

/// Detect sequential stationary events (Hariharan & Toyama style), matching Infostop C++.
pub fn get_stationary_events(
    points: &[TimedPoint],
    r1: f64,
    min_size: usize,
    min_staying_time: f64,
    max_time_between: f64,
    metric: &dyn DistanceMetric,
) -> StayEvents {
    let n = points.len();
    if n == 0 {
        return StayEvents {
            medians: Vec::new(),
            event_map: Vec::new(),
        };
    }

    let has_time = points.iter().all(|p| p.time.is_some());
    let mut event_map = vec![NON_STOP; n];
    let mut medians = Vec::new();

    let mut lats: Vec<f64> = Vec::new();
    let mut lons: Vec<f64> = Vec::new();
    insert_ordered(&mut lats, points[0].point.x);
    insert_ordered(&mut lons, points[0].point.y);
    let mut i0 = 0usize;
    let mut stay_idx = 0i32;

    for i in 1..n {
        let ddist = metric.distance(
            points[i].point,
            Point::new(median(&lats), median(&lons)),
        );

        let join = if has_time {
            let t_prev = points[i - 1].time.expect("all points timed");
            let t_curr = points[i].time.expect("all points timed");
            let dtime = t_curr - t_prev;
            ddist <= r1 && dtime <= max_time_between
        } else {
            ddist <= r1
        };

        if join {
            insert_ordered(&mut lats, points[i].point.x);
            insert_ordered(&mut lons, points[i].point.y);
        } else {
            let accept = if has_time {
                let t_start = points[i0].time.expect("all points timed");
                let t_end = points[i - 1].time.expect("all points timed");
                i - i0 >= min_size && (t_end - t_start) >= min_staying_time
            } else {
                i - i0 >= min_size
            };

            if accept {
                medians.push(Point::new(median(&lats), median(&lons)));
                for slot in &mut event_map[i0..i] {
                    *slot = stay_idx;
                }
                stay_idx += 1;
            } else {
                for slot in &mut event_map[i0..i] {
                    *slot = NON_STOP;
                }
            }

            lats.clear();
            lons.clear();
            insert_ordered(&mut lats, points[i].point.x);
            insert_ordered(&mut lons, points[i].point.y);
            i0 = i;
        }
    }

    // Final group
    let accept = if has_time {
        let t_start = points[i0].time.expect("all points timed");
        let t_end = points[n - 1].time.expect("all points timed");
        n - i0 >= min_size && (t_end - t_start) >= min_staying_time
    } else {
        n - i0 >= min_size
    };

    if accept {
        medians.push(Point::new(median(&lats), median(&lons)));
        for slot in &mut event_map[i0..n] {
            *slot = stay_idx;
        }
    } else {
        for slot in &mut event_map[i0..n] {
            *slot = NON_STOP;
        }
    }

    StayEvents { medians, event_map }
}

fn median(sorted: &[f64]) -> f64 {
    // Incoming vector is kept sorted via insert_ordered.
    let i0 = (sorted.len() - 1) / 2;
    let i1 = sorted.len() / 2;
    0.5 * (sorted[i0] + sorted[i1])
}

fn insert_ordered(arr: &mut Vec<f64>, elem: f64) {
    let pos = arr.partition_point(|&x| x <= elem);
    arr.insert(pos, elem);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distance::Euclidean;

    #[test]
    fn detects_two_stays_without_time() {
        // Stay A around (0,0), trip, stay B around (100,0)
        let mut pts = Vec::new();
        for i in 0..5 {
            pts.push(TimedPoint::new(0.0 + f64::from(i) * 0.1, 0.0));
        }
        pts.push(TimedPoint::new(50.0, 0.0)); // trip point
        for i in 0..5 {
            pts.push(TimedPoint::new(100.0 + f64::from(i) * 0.1, 0.0));
        }

        let events =
            get_stationary_events(&pts, 5.0, 2, 300.0, 86400.0, &Euclidean);
        assert_eq!(events.medians.len(), 2);
        assert!(events.event_map[0] >= 0);
        assert_eq!(events.event_map[5], NON_STOP);
        assert!(events.event_map[6] >= 0);
        assert_ne!(events.event_map[0], events.event_map[6]);
    }

    #[test]
    fn rejects_short_timed_stay() {
        let pts = vec![
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, 10.0),
            TimedPoint::with_time(0.2, 0.0, 20.0),
            TimedPoint::with_time(100.0, 0.0, 30.0),
        ];
        let events =
            get_stationary_events(&pts, 5.0, 2, 300.0, 86400.0, &Euclidean);
        assert!(events.medians.is_empty());
        assert!(events.event_map.iter().all(|&l| l == NON_STOP));
    }

    #[test]
    fn time_gap_splits_stay() {
        let pts = vec![
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, 100.0),
            TimedPoint::with_time(0.2, 0.0, 200.0),
            // gap larger than max_time_between
            TimedPoint::with_time(0.3, 0.0, 200.0 + 100_000.0),
            TimedPoint::with_time(0.4, 0.0, 200.0 + 100_100.0),
            TimedPoint::with_time(0.5, 0.0, 200.0 + 100_200.0),
        ];
        let events =
            get_stationary_events(&pts, 5.0, 2, 150.0, 1000.0, &Euclidean);
        // First stay duration 200 >= 150, second stay duration 200 >= 150
        assert_eq!(events.medians.len(), 2);
    }

    #[test]
    fn max_time_between_boundary_joins_when_equal() {
        let limit = 100.0;
        let pts = vec![
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, limit),
            TimedPoint::with_time(0.2, 0.0, 2.0 * limit),
        ];
        let events =
            get_stationary_events(&pts, 5.0, 2, 150.0, limit, &Euclidean);
        assert_eq!(events.medians.len(), 1);
        assert!(events.event_map.iter().all(|&e| e == 0));
    }

    #[test]
    fn max_time_between_boundary_splits_when_just_over() {
        let limit = 100.0;
        // Two long stays; gap between them is just over `limit`.
        let pts = vec![
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, 80.0),
            TimedPoint::with_time(0.2, 0.0, 160.0),
            TimedPoint::with_time(0.3, 0.0, 160.0 + limit + 1.0),
            TimedPoint::with_time(0.4, 0.0, 160.0 + limit + 1.0 + 80.0),
            TimedPoint::with_time(0.5, 0.0, 160.0 + limit + 1.0 + 160.0),
        ];
        let events =
            get_stationary_events(&pts, 5.0, 2, 150.0, limit, &Euclidean);
        assert_eq!(events.medians.len(), 2);
    }

    #[test]
    fn final_group_accepted_when_size_and_duration_met() {
        let pts = vec![
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, 100.0),
            TimedPoint::with_time(0.2, 0.0, 200.0),
        ];
        let events =
            get_stationary_events(&pts, 5.0, 2, 150.0, 1000.0, &Euclidean);
        assert_eq!(events.medians.len(), 1);
        assert!(events.event_map.iter().all(|&e| e == 0));
    }

    #[test]
    fn final_group_rejected_when_duration_too_short() {
        let pts = vec![
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, 50.0),
            TimedPoint::with_time(0.2, 0.0, 100.0),
        ];
        let events =
            get_stationary_events(&pts, 5.0, 2, 150.0, 1000.0, &Euclidean);
        assert!(events.medians.is_empty());
        assert!(events.event_map.iter().all(|&e| e == NON_STOP));
    }

    #[test]
    fn haversine_near_r1_joins_then_splits() {
        use crate::distance::{Haversine, EARTH_RADIUS_M};
        use std::f64::consts::PI;

        // ~1 m per (1/R) radian in longitude at the equator ≈ degrees.
        let meters_to_deg = 180.0 / (PI * EARTH_RADIUS_M);
        let r1 = 50.0;
        let within = 15.0 * meters_to_deg;
        let beyond = 80.0 * meters_to_deg;

        let pts = vec![
            TimedPoint::new(0.0, 0.0),
            TimedPoint::new(0.0, within),
            TimedPoint::new(0.0, within * 2.0),
            TimedPoint::new(0.0, beyond),
            TimedPoint::new(0.0, beyond + within),
            TimedPoint::new(0.0, beyond + within * 2.0),
        ];

        assert!(Haversine.distance(pts[0].point, pts[2].point) < r1);
        assert!(Haversine.distance(pts[0].point, pts[3].point) > r1);

        let events =
            get_stationary_events(&pts, r1, 2, 300.0, 86400.0, &Haversine);
        assert_eq!(events.medians.len(), 2);
        assert_eq!(events.event_map[0], 0);
        assert_eq!(events.event_map[1], 0);
        assert_eq!(events.event_map[2], 0);
        assert_eq!(events.event_map[3], 1);
        assert_eq!(events.event_map[4], 1);
        assert_eq!(events.event_map[5], 1);
    }
}
