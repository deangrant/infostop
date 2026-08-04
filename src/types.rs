/// Geographic or planar coordinate (latitude/x, longitude/y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn lat_lon(lat: f64, lon: f64) -> Self {
        Self { x: lat, y: lon }
    }
}

impl From<[f64; 2]> for Point {
    fn from(value: [f64; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
}

impl From<(f64, f64)> for Point {
    fn from(value: (f64, f64)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

/// Location sample with an optional timestamp (same units as time hyperparameters).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedPoint {
    pub point: Point,
    pub time: Option<f64>,
}

impl TimedPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            point: Point::new(x, y),
            time: None,
        }
    }

    pub fn with_time(x: f64, y: f64, time: f64) -> Self {
        Self {
            point: Point::new(x, y),
            time: Some(time),
        }
    }
}

impl From<[f64; 2]> for TimedPoint {
    fn from(value: [f64; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl From<[f64; 3]> for TimedPoint {
    fn from(value: [f64; 3]) -> Self {
        Self::with_time(value[0], value[1], value[2])
    }
}

impl From<(f64, f64)> for TimedPoint {
    fn from(value: (f64, f64)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<(f64, f64, f64)> for TimedPoint {
    fn from(value: (f64, f64, f64)) -> Self {
        Self::with_time(value.0, value.1, value.2)
    }
}

impl From<Point> for TimedPoint {
    fn from(point: Point) -> Self {
        Self { point, time: None }
    }
}

/// Label for a trajectory point: non-negative stop id, or `-1` for movement / rejected stay.
pub type StopLabel = i32;

/// Non-stationary / rejected label used throughout Infostop.
pub const NON_STOP: StopLabel = -1;
