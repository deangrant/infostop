use crate::distance::MetricKind;
use crate::error::{Error, Result};

/// Hyperparameters for the Infostop algorithm (defaults match the Python package).
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Max distance from the running stay median to join the current stay.
    pub r1: f64,
    /// Max distance between stay medians to form a network edge.
    pub r2: f64,
    /// If true, isolated stays receive their own labels; otherwise `-1`.
    pub label_singleton: bool,
    /// Minimum stay duration (same time units as timestamps).
    pub min_staying_time: f64,
    /// Maximum gap between consecutive samples within one stay.
    pub max_time_between: f64,
    /// Minimum number of points in a stay (`> 1`, matching Python).
    pub min_size: usize,
    /// Optional grid rounding before unique-ing stay coordinates; `0.0` disables.
    pub min_spatial_resolution: f64,
    /// Distance function for spatial comparisons.
    pub distance_metric: MetricKind,
    /// Weight edges by inverse distance.
    pub weighted: bool,
    /// Exponent used when `weighted` is true: `count * distance^(-exponent)`.
    pub weight_exponent: f64,
    /// Random seed for Infomap optimization.
    pub seed: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            r1: 10.0,
            r2: 10.0,
            label_singleton: true,
            min_staying_time: 300.0,
            max_time_between: 86400.0,
            min_size: 2,
            min_spatial_resolution: 0.0,
            distance_metric: MetricKind::Haversine,
            weighted: false,
            weight_exponent: 1.0,
            seed: 42,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.r1 <= 0.0 {
            return Err(Error::InvalidInput("`r1` must be > 0".into()));
        }
        if self.r2 <= 0.0 {
            return Err(Error::InvalidInput("`r2` must be > 0".into()));
        }
        if self.min_staying_time <= 0.0 {
            return Err(Error::InvalidInput(
                "`min_staying_time` must be > 0".into(),
            ));
        }
        if self.max_time_between <= 0.0 {
            return Err(Error::InvalidInput(
                "`max_time_between` must be > 0".into(),
            ));
        }
        if self.max_time_between <= self.min_staying_time {
            return Err(Error::InvalidInput(
                "`max_time_between` must be > `min_staying_time`".into(),
            ));
        }
        if self.min_size <= 1 {
            return Err(Error::InvalidInput("`min_size` must be > 1".into()));
        }
        if !(0.0..=1.0).contains(&self.min_spatial_resolution) {
            return Err(Error::InvalidInput(
                "`min_spatial_resolution` must be within [0, 1]".into(),
            ));
        }
        Ok(())
    }
}

/// Fluent builder for [`Config`].
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn r1(mut self, r1: f64) -> Self {
        self.config.r1 = r1;
        self
    }

    pub fn r2(mut self, r2: f64) -> Self {
        self.config.r2 = r2;
        self
    }

    pub fn label_singleton(mut self, label_singleton: bool) -> Self {
        self.config.label_singleton = label_singleton;
        self
    }

    pub fn min_staying_time(mut self, min_staying_time: f64) -> Self {
        self.config.min_staying_time = min_staying_time;
        self
    }

    pub fn max_time_between(mut self, max_time_between: f64) -> Self {
        self.config.max_time_between = max_time_between;
        self
    }

    pub fn min_size(mut self, min_size: usize) -> Self {
        self.config.min_size = min_size;
        self
    }

    pub fn min_spatial_resolution(
        mut self,
        min_spatial_resolution: f64,
    ) -> Self {
        self.config.min_spatial_resolution = min_spatial_resolution;
        self
    }

    pub fn distance_metric(mut self, distance_metric: MetricKind) -> Self {
        self.config.distance_metric = distance_metric;
        self
    }

    pub fn weighted(mut self, weighted: bool) -> Self {
        self.config.weighted = weighted;
        self
    }

    pub fn weight_exponent(mut self, weight_exponent: f64) -> Self {
        self.config.weight_exponent = weight_exponent;
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.config.seed = seed;
        self
    }

    pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }

    /// Build without validating (caller must validate). Prefer [`Self::build`].
    pub fn build_unchecked(self) -> Config {
        self.config
    }
}
