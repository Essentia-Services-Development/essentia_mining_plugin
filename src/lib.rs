//! # Essentia Mining Plugin
//!
//! Bitcoin and cryptocurrency mining plugin that leverages `essentia_hwdetect`
//! for hardware detection and runs as a background service using available
//! system resources.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Mining Plugin                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │  Hardware   │  │   Mining    │  │   Pool/Protocol     │  │
//! │  │  Detector   │  │ Coordinator │  │   Integration       │  │
//! │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
//! │         │                │                     │             │
//! │         ▼                ▼                     ▼             │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │              Background Mining Service               │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │  essentia_hwdetect  │  essentia_async_runtime  │  essentia_resource_management
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Hardware Detection**: Leverages `essentia_hwdetect` for CPU/GPU capability detection
//! - **Background Processing**: Uses `essentia_async_runtime` for non-blocking mining
//! - **Resource Management**: Integrates with `essentia_resource_management` for CPU throttling
//! - **Pool Support**: Stratum protocol implementation for mining pool integration
//! - **SHA-256 Implementation**: Pure Rust SHA-256 for Proof-of-Work validation
//!
//! ## Usage
//!
//! ```rust,ignore
//! use essentia_mining_plugin::{MiningPlugin, MiningConfig};
//!
//! let config = MiningConfig::default()
//!     .with_max_cpu_usage(50) // Use max 50% CPU
//!     .with_background_priority(true);
//!
//! let plugin = MiningPlugin::new(config)?;
//! plugin.start_background_mining()?;
//! ```

mod types;
mod errors;
mod config;
mod hardware;
mod sha256;
mod coordinator;
mod stratum;
mod plugin;

pub use types::{
    MiningStats, BlockHeader, HashTarget, Nonce, MiningJob, PoolConnection,
};
pub use errors::{MiningError, MiningResult};
pub use config::MiningConfig;
pub use hardware::MiningHardwareProfile;
pub use coordinator::MiningCoordinator;
pub use stratum::StratumClient;
pub use plugin::MiningPlugin;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let config = MiningConfig::default();
        assert!(config.max_cpu_percentage <= 100);
    }

    #[test]
    fn test_default_config() {
        let config = MiningConfig::default();
        assert_eq!(config.max_cpu_percentage, 25); // Default: use 25% CPU
        assert!(config.background_priority);
    }
}
