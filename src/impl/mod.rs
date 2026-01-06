//! Mining Plugin implementations.
//!
//! This module contains all implementations for the Mining plugin:
//! - `MiningConfig` - Configuration
//! - `MiningHardwareProfile` - Hardware detection
//! - `MiningCoordinator` - Mining thread management
//! - `StratumClient` - Pool protocol client
//! - `MiningPlugin` - Main plugin interface

mod config;
mod coordinator;
mod crypto;
mod hardware;
mod plugin;
mod stratum;

pub use config::MiningConfig;
pub use coordinator::MiningCoordinator;
pub use crypto::{Sha256, double_sha256, sha256, sha256_hex};
pub use hardware::MiningHardwareProfile;
pub use plugin::MiningPlugin;
pub use stratum::{StratumClient, parse_stratum_url};
