use std::collections::HashMap;

use crate::cluster::{build_edges, CommunityDetector, InfomapDetector};
use crate::config::{Config, ConfigBuilder};
use crate::distance::{metric_for, DistanceMetric, MetricKind};
use crate::error::{Error, Result};
use crate::neighbors::{query_neighbors, BruteForceNeighbors, NeighborQuery};
use crate::stay::get_stationary_events;
use crate::types::{Point, StopLabel, TimedPoint, NON_STOP};

/// Infer stop-location labels from mobility traces (Infostop algorithm).
///
/// # Example
///
/// ```
/// use infostop::Infostop;
///
/// let trace = vec![
///     [0.0_f64, 0.0],
///     [0.00001, 0.0],
///     [0.00002, 0.0],
///     [0.01, 0.0],
///     [0.01001, 0.0],
///     [0.01002, 0.0],
/// ];
/// let mut model = Infostop::builder()
///     .r1(50.0)
///     .r2(50.0)
///     .distance_metric(infostop::MetricKind::Euclidean)
///     .min_size(2)
///     .build()
///     .unwrap();
/// let labels = model.fit_predict(&trace).unwrap();
/// assert_eq!(labels.len(), trace.len());
/// ```
#[derive(Debug)]
pub struct Infostop<D = InfomapDetector, N = BruteForceNeighbors>
where
    D: CommunityDetector,
    N: NeighborQuery,
{
    config: Config,
    detector: D,
    neighbors: N,
    /// Unique stay coordinates used during the last fit.
    unique_stays: Vec<Point>,
    /// Labels for unique stays from the last fit.
    unique_labels: Vec<StopLabel>,
    fitted: bool,
}

impl Infostop<InfomapDetector, BruteForceNeighbors> {
    /// Create a model with default hyperparameters (matching the Python package).
    pub fn new() -> Self {
        Self::from_config(Config::default()).expect("default config is valid")
    }

    /// Start a fluent configuration builder.
    pub fn builder() -> InfostopBuilder {
        InfostopBuilder {
            config: ConfigBuilder::new(),
        }
    }

    pub fn from_config(config: Config) -> Result<Self> {
        config.validate()?;
        let detector = InfomapDetector {
            seed: config.seed,
            trials: 3,
        };
        Ok(Self {
            config,
            detector,
            neighbors: BruteForceNeighbors,
            unique_stays: Vec::new(),
            unique_labels: Vec::new(),
            fitted: false,
        })
    }
}

impl Default for Infostop<InfomapDetector, BruteForceNeighbors> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D, N> Infostop<D, N>
where
    D: CommunityDetector,
    N: NeighborQuery,
{
    /// Inject custom detector / neighbor query (composition root / tests).
    pub fn with_parts(
        config: Config,
        detector: D,
        neighbors: N,
    ) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            detector,
            neighbors,
            unique_stays: Vec::new(),
            unique_labels: Vec::new(),
            fitted: false,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn is_fitted(&self) -> bool {
        self.fitted
    }

    /// Fit on a single trajectory and return a label per point.
    pub fn fit_predict<T>(&mut self, trace: &[T]) -> Result<Vec<StopLabel>>
    where
        T: Into<TimedPoint> + Copy,
    {
        let mut labels = self.fit_predict_many(&[trace])?;
        Ok(labels.pop().unwrap_or_default())
    }

    /// Fit on multiple trajectories; stop locations are shared across traces.
    ///
    /// Each trajectory may be a slice of `[f64; 2]`, `[f64; 3]`, [`TimedPoint`], etc.
    pub fn fit_predict_many<T, U>(
        &mut self,
        traces: &[T],
    ) -> Result<Vec<Vec<StopLabel>>>
    where
        T: AsRef<[U]>,
        U: Into<TimedPoint> + Copy,
    {
        if traces.is_empty() {
            return Err(Error::InvalidInput(
                "expected at least one trajectory".into(),
            ));
        }

        let converted: Vec<Vec<TimedPoint>> = traces
            .iter()
            .map(|t| t.as_ref().iter().map(|p| (*p).into()).collect())
            .collect();
        let refs: Vec<&[TimedPoint]> =
            converted.iter().map(|v| v.as_slice()).collect();
        self.fit_predict_slices(&refs)
    }

    fn fit_predict_slices(
        &mut self,
        traces: &[&[TimedPoint]],
    ) -> Result<Vec<Vec<StopLabel>>> {
        if traces.is_empty() {
            return Err(Error::InvalidInput(
                "expected at least one trajectory".into(),
            ));
        }

        let metric = metric_for(self.config.distance_metric);
        self.validate_traces(traces, metric.as_ref())?;

        let mut all_medians: Vec<Point> = Vec::new();
        let mut event_maps: Vec<Vec<i32>> = Vec::new();
        let mut stay_counts_per_trace: Vec<usize> = Vec::new();

        for trace in traces {
            let events = get_stationary_events(
                trace,
                self.config.r1,
                self.config.min_size,
                self.config.min_staying_time,
                self.config.max_time_between,
                metric.as_ref(),
            );
            stay_counts_per_trace.push(events.medians.len());
            all_medians.extend(events.medians);
            event_maps.push(events.event_map);
        }

        if all_medians.is_empty() {
            return Err(Error::NoStopsFound);
        }

        // Optional spatial resolution rounding.
        if self.config.min_spatial_resolution > 0.0 {
            let res = self.config.min_spatial_resolution;
            for p in &mut all_medians {
                p.x = (p.x / res).round() * res;
                p.y = (p.y / res).round() * res;
            }
        }

        let (unique_coords, inverse, counts) = unique_points(&all_medians);
        let (nbrs, dists) = query_neighbors(
            &unique_coords,
            self.config.r2,
            metric.as_ref(),
            self.config.weighted,
            &self.neighbors,
        );

        let (edges, singletons) = build_edges(
            &nbrs,
            dists.as_deref(),
            &counts,
            self.config.weight_exponent,
        );

        let unique_labels = self.detector.cluster(
            &edges,
            unique_coords.len(),
            self.config.label_singleton,
            &singletons,
        );

        if unique_labels.iter().all(|&l| l == NON_STOP) {
            return Err(Error::NoStopsFound);
        }

        // Reverse spatial unique: label per stay event (in all_medians order).
        let stay_labels: Vec<StopLabel> =
            inverse.iter().map(|&idx| unique_labels[idx]).collect();

        // Reverse temporal downsampling per trace.
        let mut offset = 0usize;
        let mut output = Vec::with_capacity(traces.len());
        for (t_idx, event_map) in event_maps.into_iter().enumerate() {
            let n_stays = stay_counts_per_trace[t_idx];
            let slice = &stay_labels[offset..offset + n_stays];
            offset += n_stays;

            // Append NON_STOP sentinel so event_map == -1 indexes the last slot.
            let mut lookup = slice.to_vec();
            lookup.push(NON_STOP);

            let labels: Vec<StopLabel> = event_map
                .into_iter()
                .map(|e| if e < 0 { NON_STOP } else { lookup[e as usize] })
                .collect();
            output.push(labels);
        }

        self.unique_stays = unique_coords;
        self.unique_labels = unique_labels;
        self.fitted = true;
        Ok(output)
    }

    /// Median coordinate of each stop label from the last fit.
    pub fn label_medians(&self) -> Result<HashMap<StopLabel, Point>> {
        if !self.fitted {
            return Err(Error::NotFitted);
        }
        let mut buckets: HashMap<StopLabel, Vec<Point>> = HashMap::new();
        for (p, &lab) in self.unique_stays.iter().zip(self.unique_labels.iter())
        {
            if lab == NON_STOP {
                continue;
            }
            buckets.entry(lab).or_default().push(*p);
        }
        let mut out = HashMap::new();
        for (lab, pts) in buckets {
            out.insert(lab, median_point(&pts));
        }
        Ok(out)
    }

    /// Unique stay medians from the last fit (for plotting).
    pub fn stationary_points(&self) -> Result<&[Point]> {
        if !self.fitted {
            return Err(Error::NotFitted);
        }
        Ok(&self.unique_stays)
    }

    /// Labels corresponding to [`Self::stationary_points`].
    pub fn stationary_labels(&self) -> Result<&[StopLabel]> {
        if !self.fitted {
            return Err(Error::NotFitted);
        }
        Ok(&self.unique_labels)
    }

    fn validate_traces(
        &self,
        traces: &[&[TimedPoint]],
        _metric: &dyn DistanceMetric,
    ) -> Result<()> {
        for (u, pts) in traces.iter().enumerate() {
            let prefix = if traces.len() == 1 {
                String::new()
            } else {
                format!("trace {u}: ")
            };
            if pts.is_empty() {
                return Err(Error::InvalidInput(format!(
                    "{prefix}trajectory is empty"
                )));
            }
            for p in *pts {
                if !p.point.x.is_finite() || !p.point.y.is_finite() {
                    return Err(Error::InvalidInput(format!(
                        "{prefix}coordinates must be finite"
                    )));
                }
                if let Some(t) = p.time {
                    if !t.is_finite() {
                        return Err(Error::InvalidInput(format!(
                            "{prefix}timestamps must be finite"
                        )));
                    }
                }
            }

            let timed = pts.iter().filter(|p| p.time.is_some()).count();
            if timed != 0 && timed != pts.len() {
                return Err(Error::InvalidInput(format!(
                    "{prefix}timestamps must be present on all points or on none"
                )));
            }

            let times: Vec<f64> = pts.iter().filter_map(|p| p.time).collect();
            if times.len() > 1 {
                for w in times.windows(2) {
                    if w[0] > w[1] {
                        return Err(Error::InvalidInput(format!(
                            "{prefix}timestamps must be ordered"
                        )));
                    }
                }
            }

            if self.config.distance_metric == MetricKind::Haversine {
                for p in *pts {
                    if !(-90.0..=90.0).contains(&p.point.x) {
                        return Err(Error::InvalidInput(format!(
                            "{prefix}latitude must be between -90 and 90"
                        )));
                    }
                    if !(-180.0..=180.0).contains(&p.point.y) {
                        return Err(Error::InvalidInput(format!(
                            "{prefix}longitude must be between -180 and 180"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Builder that constructs a default-wired [`Infostop`].
#[derive(Debug)]
pub struct InfostopBuilder {
    config: ConfigBuilder,
}

impl InfostopBuilder {
    pub fn r1(mut self, r1: f64) -> Self {
        self.config = self.config.r1(r1);
        self
    }
    pub fn r2(mut self, r2: f64) -> Self {
        self.config = self.config.r2(r2);
        self
    }
    pub fn label_singleton(mut self, label_singleton: bool) -> Self {
        self.config = self.config.label_singleton(label_singleton);
        self
    }
    pub fn min_staying_time(mut self, min_staying_time: f64) -> Self {
        self.config = self.config.min_staying_time(min_staying_time);
        self
    }
    pub fn max_time_between(mut self, max_time_between: f64) -> Self {
        self.config = self.config.max_time_between(max_time_between);
        self
    }
    pub fn min_size(mut self, min_size: usize) -> Self {
        self.config = self.config.min_size(min_size);
        self
    }
    pub fn min_spatial_resolution(
        mut self,
        min_spatial_resolution: f64,
    ) -> Self {
        self.config =
            self.config.min_spatial_resolution(min_spatial_resolution);
        self
    }
    pub fn distance_metric(mut self, distance_metric: MetricKind) -> Self {
        self.config = self.config.distance_metric(distance_metric);
        self
    }
    pub fn weighted(mut self, weighted: bool) -> Self {
        self.config = self.config.weighted(weighted);
        self
    }
    pub fn weight_exponent(mut self, weight_exponent: f64) -> Self {
        self.config = self.config.weight_exponent(weight_exponent);
        self
    }
    pub fn seed(mut self, seed: u64) -> Self {
        self.config = self.config.seed(seed);
        self
    }

    pub fn build(self) -> Result<Infostop> {
        let config = self.config.build()?;
        Infostop::from_config(config)
    }
}

/// Deduplicate stay medians with exact float equality (numpy `unique`-style).
///
/// Equality uses `f64` bit patterns after optional `min_spatial_resolution`
/// grid rounding. Near-identical coordinates that differ by floating-point
/// drift are **not** merged unless they round to the same grid cell first.
/// First-seen order of unique points is preserved.
fn unique_points(points: &[Point]) -> (Vec<Point>, Vec<usize>, Vec<usize>) {
    let mut unique: Vec<Point> = Vec::new();
    let mut inverse = Vec::with_capacity(points.len());
    let mut counts: Vec<usize> = Vec::new();
    let mut index: HashMap<(u64, u64), usize> =
        HashMap::with_capacity(points.len());

    for p in points {
        let key = (p.x.to_bits(), p.y.to_bits());
        if let Some(&idx) = index.get(&key) {
            inverse.push(idx);
            counts[idx] += 1;
        } else {
            let idx = unique.len();
            index.insert(key, idx);
            inverse.push(idx);
            counts.push(1);
            unique.push(*p);
        }
    }
    (unique, inverse, counts)
}

fn median_point(points: &[Point]) -> Point {
    let mut xs: Vec<f64> = points.iter().map(|p| p.x).collect();
    let mut ys: Vec<f64> = points.iter().map(|p| p.y).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Point::new(median_slice(&xs), median_slice(&ys))
}

fn median_slice(sorted: &[f64]) -> f64 {
    let i0 = (sorted.len() - 1) / 2;
    let i1 = sorted.len() / 2;
    0.5 * (sorted[i0] + sorted[i1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_two_stops() -> Vec<[f64; 2]> {
        let mut trace = Vec::new();
        for i in 0..8 {
            trace.push([0.0, (i as f64) * 0.01]);
        }
        // trip
        trace.push([50.0, 0.0]);
        trace.push([60.0, 0.0]);
        for i in 0..8 {
            trace.push([100.0, (i as f64) * 0.01]);
        }
        trace
    }

    #[test]
    fn fit_predict_labels_two_destinations() {
        let mut model = Infostop::builder()
            .r1(1.0)
            .r2(5.0)
            .distance_metric(MetricKind::Euclidean)
            .min_size(2)
            .build()
            .unwrap();
        let trace = synthetic_two_stops();
        let labels = model.fit_predict(&trace).unwrap();
        assert_eq!(labels.len(), trace.len());
        let stop_a = labels[0];
        let stop_b = labels[labels.len() - 1];
        assert!(stop_a >= 0);
        assert!(stop_b >= 0);
        assert_ne!(stop_a, stop_b);
        assert_eq!(labels[8], NON_STOP);
        let medians = model.label_medians().unwrap();
        assert!(medians.len() >= 2);
    }

    #[test]
    fn multi_trace_shares_destinations() {
        let mut model = Infostop::builder()
            .r1(1.0)
            .r2(5.0)
            .distance_metric(MetricKind::Euclidean)
            .min_size(2)
            .build()
            .unwrap();

        let t1 = synthetic_two_stops();
        let t2: Vec<[f64; 2]> =
            (0..8).map(|i| [0.0, i as f64 * 0.01]).collect();

        let labels = model
            .fit_predict_many(&[t1.as_slice(), t2.as_slice()])
            .unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0][0], labels[1][0]);
    }

    #[test]
    fn haversine_accepts_poles_and_antimeridian() {
        let mut model = Infostop::builder()
            .r1(100.0)
            .r2(100.0)
            .min_size(2)
            .distance_metric(MetricKind::Haversine)
            .build()
            .unwrap();

        for point in [
            [90.0, 0.0],
            [-90.0, 0.0],
            [0.0, 180.0],
            [0.0, -180.0],
        ] {
            let trace = [point, point];
            assert!(
                model.fit_predict(&trace).is_ok(),
                "expected inclusive bound {point:?} to validate"
            );
        }
    }

    #[test]
    fn haversine_rejects_out_of_range_coordinates() {
        let mut model = Infostop::builder()
            .r1(100.0)
            .r2(100.0)
            .min_size(2)
            .distance_metric(MetricKind::Haversine)
            .build()
            .unwrap();

        let bad_lat = model.fit_predict(&[[91.0, 0.0], [91.0, 0.0]]);
        assert!(matches!(bad_lat, Err(Error::InvalidInput(_))));

        let bad_lon = model.fit_predict(&[[0.0, 181.0], [0.0, 181.0]]);
        assert!(matches!(bad_lon, Err(Error::InvalidInput(_))));
    }

    #[test]
    fn rejects_mixed_timestamps_on_one_trajectory() {
        let mut model = Infostop::builder()
            .r1(100.0)
            .r2(100.0)
            .min_size(2)
            .distance_metric(MetricKind::Euclidean)
            .build()
            .unwrap();

        let trace = [
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::new(0.1, 0.0),
            TimedPoint::with_time(0.2, 0.0, 60.0),
        ];
        let err = model.fit_predict(&trace);
        assert!(matches!(err, Err(Error::InvalidInput(_))));
    }

    #[test]
    fn accepts_fully_timed_trajectory() {
        let mut model = Infostop::builder()
            .r1(100.0)
            .r2(100.0)
            .min_size(2)
            .min_staying_time(50.0)
            .max_time_between(10_000.0)
            .distance_metric(MetricKind::Euclidean)
            .build()
            .unwrap();

        let trace = [
            TimedPoint::with_time(0.0, 0.0, 0.0),
            TimedPoint::with_time(0.1, 0.0, 60.0),
            TimedPoint::with_time(0.2, 0.0, 120.0),
        ];
        assert!(model.fit_predict(&trace).is_ok());
    }

    fn isolated_far_stays() -> Vec<[f64; 2]> {
        let mut trace = Vec::new();
        for i in 0..4 {
            trace.push([0.0, i as f64 * 0.01]);
        }
        for i in 0..4 {
            trace.push([1000.0, i as f64 * 0.01]);
        }
        trace
    }

    #[test]
    fn all_singletons_without_labeling_are_no_stops_found() {
        let mut model = Infostop::builder()
            .r1(1.0)
            .r2(0.5)
            .min_size(2)
            .label_singleton(false)
            .distance_metric(MetricKind::Euclidean)
            .build()
            .unwrap();

        let err = model.fit_predict(&isolated_far_stays());
        assert_eq!(err, Err(Error::NoStopsFound));
        assert!(matches!(
            model.label_medians(),
            Err(Error::NotFitted)
        ));
    }

    #[test]
    fn all_singletons_with_labeling_succeed() {
        let mut model = Infostop::builder()
            .r1(1.0)
            .r2(0.5)
            .min_size(2)
            .label_singleton(true)
            .distance_metric(MetricKind::Euclidean)
            .build()
            .unwrap();

        let labels = model.fit_predict(&isolated_far_stays()).unwrap();
        assert!(labels.iter().any(|&l| l >= 0));
        assert!(!model.label_medians().unwrap().is_empty());
    }

    #[test]
    fn unique_points_collapses_exact_duplicates() {
        let pts = [
            Point::new(1.0, 2.0),
            Point::new(3.0, 4.0),
            Point::new(1.0, 2.0),
            Point::new(3.0, 4.0),
            Point::new(1.0, 2.0),
        ];
        let (unique, inverse, counts) = unique_points(&pts);
        assert_eq!(unique.len(), 2);
        assert_eq!(unique[0], Point::new(1.0, 2.0));
        assert_eq!(unique[1], Point::new(3.0, 4.0));
        assert_eq!(inverse, vec![0, 1, 0, 1, 0]);
        assert_eq!(counts, vec![3, 2]);
    }

    #[test]
    fn unique_points_keeps_bit_distinct_floats() {
        let a = 1.0_f64;
        let b = 1.0 + f64::EPSILON;
        assert_ne!(a.to_bits(), b.to_bits());
        let pts = [Point::new(a, 0.0), Point::new(b, 0.0)];
        let (unique, inverse, counts) = unique_points(&pts);
        assert_eq!(unique.len(), 2);
        assert_eq!(inverse, vec![0, 1]);
        assert_eq!(counts, vec![1, 1]);
    }

    #[test]
    fn unique_points_merges_after_grid_rounding() {
        let res = 0.1_f64;
        let mut pts = [
            Point::new(0.11, 0.0),
            Point::new(0.14, 0.0),
        ];
        for p in &mut pts {
            p.x = (p.x / res).round() * res;
            p.y = (p.y / res).round() * res;
        }
        let (unique, inverse, counts) = unique_points(&pts);
        assert_eq!(unique.len(), 1);
        assert_eq!(inverse, vec![0, 0]);
        assert_eq!(counts, vec![2]);
        assert!((unique[0].x - 0.1).abs() < 1e-12);
    }
}
