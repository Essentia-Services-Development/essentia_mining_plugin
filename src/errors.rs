//! Mining plugin error types.

use core::fmt;

/// Mining operation errors.
#[derive(Debug)]
pub enum MiningError {
    /// Hardware detection failed.
    HardwareDetection(String),
    /// Pool connection error.
    PoolConnection(String),
    /// Stratum protocol error.
    StratumProtocol(String),
    /// Resource allocation failed.
    ResourceAllocation(String),
    /// Mining coordinator error.
    Coordinator(String),
    /// Configuration error.
    Configuration(String),
    /// SHA-256 computation error.
    HashComputation(String),
}

impl fmt::Display for MiningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HardwareDetection(msg) => write!(f, "Hardware detection error: {msg}"),
            Self::PoolConnection(msg) => write!(f, "Pool connection error: {msg}"),
            Self::StratumProtocol(msg) => write!(f, "Stratum protocol error: {msg}"),
            Self::ResourceAllocation(msg) => write!(f, "Resource allocation error: {msg}"),
            Self::Coordinator(msg) => write!(f, "Mining coordinator error: {msg}"),
            Self::Configuration(msg) => write!(f, "Configuration error: {msg}"),
            Self::HashComputation(msg) => write!(f, "Hash computation error: {msg}"),
        }
    }
}

/// Result type for mining operations.
pub type MiningResult<T> = Result<T, MiningError>;
