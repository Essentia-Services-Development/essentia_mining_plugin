//! Mining Plugin implementations.
//!
//! This module contains all implementations for the Mining plugin:
//! - `MiningConfig` - Configuration
//! - `MiningHardwareProfile` - Hardware detection
//! - `MiningCoordinator` - Mining thread management
//! - `StratumClient` - Pool protocol client
//! - `MiningPlugin` - Main plugin interface
//! - `PoolManager` - Multi-pool management
//! - `HashRateMonitor` - Hash rate tracking
//! - `RewardDistributor` - Reward calculation and distribution

mod config;
mod coordinator;
mod crypto;
mod hardware;
mod hash_rate_monitor;
mod plugin;
mod pool_management;
mod reward_distribution;
mod stratum;

pub use config::MiningConfig;
pub use coordinator::MiningCoordinator;
pub use crypto::{Sha256, double_sha256, sha256, sha256_hex};
pub use hardware::MiningHardwareProfile;
pub use hash_rate_monitor::*;
pub use plugin::MiningPlugin;
pub use pool_management::*;
pub use reward_distribution::*;
pub use stratum::{StratumClient, parse_stratum_url};
