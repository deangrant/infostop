//! Infostop: detect stop locations in GPS / mobility trajectory data.
//!
//! This crate ports the Infostop algorithm (sequential stay detection + Infomap
//! clustering of stay medians). Stay detection and edge weighting target
//! reference Infostop behavior. Community detection uses a **simplified
//! two-level** Infomap (greedy incremental search; no multilevel aggregation or
//! teleportation), so partitions need not match upstream Infomap. It is
//! **std-first**: no required third-party crates.
//!
//! # Quick start
//!
//! ```
//! use infostop::{Infostop, MetricKind};
//!
//! let mut model = Infostop::builder()
//!     .r1(5.0)
//!     .r2(10.0)
//!     .distance_metric(MetricKind::Euclidean)
//!     .build()
//!     .unwrap();
//!
//! let trace = [
//!     [0.0, 0.0],
//!     [0.1, 0.0],
//!     [0.2, 0.0],
//!     [50.0, 0.0],
//!     [100.0, 0.0],
//!     [100.1, 0.0],
//!     [100.2, 0.0],
//! ];
//! let labels = model.fit_predict(&trace).unwrap();
//! assert_eq!(labels.len(), trace.len());
//! ```
//!
//! Enable the `plot` feature and call [`plot_map`] to write a Leaflet HTML map.

mod cluster;
mod config;
mod distance;
mod error;
mod infomap;
mod model;
mod neighbors;
mod stay;
mod types;

#[cfg(feature = "plot")]
mod plot;

pub use config::{Config, ConfigBuilder};
pub use distance::{
    DistanceMetric, Euclidean, Haversine, MetricKind, EARTH_RADIUS_M,
};
pub use error::{Error, Result};
pub use model::{Infostop, InfostopBuilder};
pub use types::{Point, StopLabel, TimedPoint, NON_STOP};

pub use cluster::{CommunityDetector, InfomapDetector};
pub use neighbors::{BruteForceNeighbors, NeighborQuery};

#[cfg(feature = "plot")]
pub use plot::plot_map;
