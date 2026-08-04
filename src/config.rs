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
    /// Check that hyperparameters are finite and internally consistent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] when a field is non-finite, non-positive
    /// where required, or when `max_time_between <= min_staying_time`.
    pub fn validate(&self) -> Result<()> {
        if !self.r1.is_finite() || self.r1 <= 0.0 {
            return Err(Error::InvalidInput(
                "`r1` must be finite and > 0".into(),
            ));
        }
        if !self.r2.is_finite() || self.r2 <= 0.0 {
            return Err(Error::InvalidInput(
                "`r2` must be finite and > 0".into(),
            ));
        }
        if !self.min_staying_time.is_finite() || self.min_staying_time <= 0.0 {
            return Err(Error::InvalidInput(
                "`min_staying_time` must be finite and > 0".into(),
            ));
        }
        if !self.max_time_between.is_finite() || self.max_time_between <= 0.0 {
            return Err(Error::InvalidInput(
                "`max_time_between` must be finite and > 0".into(),
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
        if !self.min_spatial_resolution.is_finite()
            || self.min_spatial_resolution < 0.0
        {
            return Err(Error::InvalidInput(
                "`min_spatial_resolution` must be finite and >= 0".into(),
            ));
        }
        if !self.weight_exponent.is_finite() {
            return Err(Error::InvalidInput(
                "`weight_exponent` must be finite".into(),
            ));
        }
        Ok(())
    }
}

/// Fluent builder for [`Config`].
#[derive(Debug, Clone, Default)]
#[must_use]
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

    /// Validate hyperparameters and produce a [`Config`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] when [`Config::validate`] fails.
    pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }

    /// Build without validating (caller must validate). Prefer [`Self::build`].
    pub fn build_unchecked(self) -> Config {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn rejects_non_positive_r1() {
        let cfg = Config {
            r1: 0.0,
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_invalid_time_ordering() {
        let cfg = Config {
            min_staying_time: 100.0,
            max_time_between: 50.0,
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_non_finite_hyperparameters() {
        type FieldSetter = fn(&mut Config, f64);
        let cases: &[(&str, FieldSetter)] = &[
            ("r1", |c, v| c.r1 = v),
            ("r2", |c, v| c.r2 = v),
            ("min_staying_time", |c, v| c.min_staying_time = v),
            ("max_time_between", |c, v| c.max_time_between = v),
            ("min_spatial_resolution", |c, v| {
                c.min_spatial_resolution = v
            }),
            ("weight_exponent", |c, v| c.weight_exponent = v),
        ];

        for (name, set) in cases {
            for bad in [f64::NAN, f64::INFINITY] {
                let mut cfg = Config::default();
                set(&mut cfg, bad);
                assert!(
                    cfg.validate().is_err(),
                    "{name} must reject non-finite value {bad}"
                );
            }
        }
    }

    #[test]
    fn accepts_general_min_spatial_resolution() {
        for res in [0.0, 1e-5, 5.0] {
            let cfg = Config {
                min_spatial_resolution: res,
                ..Config::default()
            };
            assert!(
                cfg.validate().is_ok(),
                "expected min_spatial_resolution={res} to be valid"
            );
        }
    }

    #[test]
    fn rejects_negative_min_spatial_resolution() {
        let cfg = Config {
            min_spatial_resolution: -1.0,
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }
}
