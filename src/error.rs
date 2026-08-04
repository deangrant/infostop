use std::error::Error as StdError;
use std::fmt;

/// Errors returned by Infostop fitting and helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Input data failed validation.
    InvalidInput(String),
    /// No stationary events were found for the given parameters.
    NoStopsFound,
    /// A method that requires a fitted model was called too early.
    NotFitted,
    /// I/O failure (e.g. writing a map file).
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Error::NoStopsFound => write!(
                f,
                "no stop events found; check that `r1`, `min_staying_time`, and `min_size` are chosen correctly"
            ),
            Error::NotFitted => write!(f, "model must be fitted before this method can be used"),
            Error::Io(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl StdError for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value.to_string())
    }
}

/// Crate-wide result type.
pub type Result<T> = std::result::Result<T, Error>;
