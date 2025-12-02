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
//! - **Hardware Detection**: Leverages `essentia_hwdetect` for CPU/GPU
//!   capability detection
//! - **Background Processing**: Uses `essentia_async_runtime` for non-blocking
//!   mining
//! - **Resource Management**: Integrates with `essentia_resource_management`
//!   for CPU throttling
//! - **Pool Support**: Stratum protocol implementation for mining pool
//!   integration
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

// Mining plugin pedantic lint allowances (MINING-LINT-STAGING-01)
#![allow(
    clippy::unreadable_literal,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::needless_pass_by_value,
    clippy::doc_markdown,
    clippy::unnecessary_literal_bound,
    clippy::unnecessary_wraps,
    clippy::manual_midpoint,
    clippy::map_unwrap_or,
    clippy::redundant_closure_for_method_calls
)]

mod config;
mod coordinator;
mod errors;
pub mod flexforge;
mod hardware;
mod plugin;
mod sha256;
mod stratum;
mod types;

pub use config::MiningConfig;
pub use coordinator::MiningCoordinator;
pub use errors::{MiningError, MiningResult};
pub use flexforge::{MiningDisplayStats, MiningPluginFlexForge, MiningUiConfig};
pub use hardware::MiningHardwareProfile;
pub use plugin::MiningPlugin;
pub use stratum::{StratumClient, parse_stratum_url};
pub use types::{BlockHeader, HashTarget, MiningJob, MiningStats, Nonce, PoolConnection};

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
