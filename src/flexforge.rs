//! FlexForge Integration for Essentia Mining Plugin
//!
//! Provides mining dashboard and configuration through the
//! FlexForge Universal Editor.
//!
//! ## Features
//!
//! - Real-time hashrate and statistics streaming
//! - Mining pool configuration
//! - Hardware utilization monitoring
//! - Earnings dashboard

use essentia_traits::plugin_contracts::{
    ConfigField, ConfigSchema, FlexForgeCapability, FlexForgeIntegration, FlexForgePanelCategory,
    FlexForgePanelInfo, StreamingCapable, UiConfigurable,
};

/// Mining Plugin FlexForge integration.
#[derive(Debug)]
pub struct MiningPluginFlexForge {
    config:        MiningUiConfig,
    stream_active: bool,
    stream_id:     Option<u64>,
    next_id:       u64,
    /// Current mining statistics
    stats:         MiningDisplayStats,
}

/// Configuration exposed through FlexForge UI.
#[derive(Debug, Clone)]
pub struct MiningUiConfig {
    /// Mining enabled
    pub mining_enabled: bool,
    /// Pool address
    pub pool_address:   String,
    /// Wallet address
    pub wallet_address: String,
    /// Worker name
    pub worker_name:    String,
    /// CPU mining enabled
    pub cpu_mining:     bool,
    /// GPU mining enabled
    pub gpu_mining:     bool,
    /// Max CPU threads
    pub cpu_threads:    u8,
    /// GPU intensity (1-100)
    pub gpu_intensity:  u8,
    /// Temperature limit (Celsius)
    pub temp_limit:     u8,
}

/// Statistics for dashboard display.
#[derive(Debug, Clone, Default)]
pub struct MiningDisplayStats {
    /// Current hashrate (H/s)
    pub hashrate:         f64,
    /// Accepted shares
    pub shares_accepted:  u64,
    /// Rejected shares
    pub shares_rejected:  u64,
    /// Current difficulty
    pub difficulty:       f64,
    /// Estimated earnings (satoshi/day)
    pub earnings_per_day: u64,
    /// GPU temperature
    pub gpu_temp:         u8,
    /// CPU usage percentage
    pub cpu_usage:        u8,
}

impl Default for MiningUiConfig {
    fn default() -> Self {
        Self {
            mining_enabled: false, // Opt-in by default
            pool_address:   String::new(),
            wallet_address: String::new(),
            worker_name:    String::from("essentia-worker"),
            cpu_mining:     false,
            gpu_mining:     true,
            cpu_threads:    2,
            gpu_intensity:  70,
            temp_limit:     80,
        }
    }
}

impl MiningPluginFlexForge {
    /// Creates a new FlexForge integration wrapper.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config:        MiningUiConfig::default(),
            stream_active: false,
            stream_id:     None,
            next_id:       1,
            stats:         MiningDisplayStats::default(),
        }
    }

    /// Returns panel info with capabilities.
    #[must_use]
    pub fn panel_info(&self) -> FlexForgePanelInfo {
        FlexForgePanelInfo {
            id:           self.panel_id().to_string(),
            name:         self.display_name().to_string(),
            category:     self.category(),
            icon:         self.icon_glyph().map(String::from),
            priority:     self.priority(),
            capabilities: vec![
                FlexForgeCapability::Configuration,
                FlexForgeCapability::Streaming,
                FlexForgeCapability::Dashboard,
            ],
        }
    }

    /// Updates statistics from mining coordinator.
    pub fn update_stats(&mut self, stats: MiningDisplayStats) {
        self.stats = stats;
    }

    fn next_stream_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

impl Default for MiningPluginFlexForge {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FlexForge Integration
// ============================================================================

impl FlexForgeIntegration for MiningPluginFlexForge {
    fn panel_id(&self) -> &str {
        "essentia_mining_plugin"
    }

    fn category(&self) -> FlexForgePanelCategory {
        FlexForgePanelCategory::System
    }

    fn display_name(&self) -> &str {
        "Crypto Mining"
    }

    fn icon_glyph(&self) -> Option<&str> {
        Some("\u{E8A7}") // Coin/currency icon
    }

    fn priority(&self) -> u32 {
        25 // Lower priority in System category
    }

    fn on_panel_activate(&mut self) {
        // Start stats streaming when panel is viewed
        if self.config.mining_enabled && !self.stream_active {
            let _ = self.start_stream();
        }
    }

    fn on_panel_deactivate(&mut self) {
        if self.stream_active
            && let Some(id) = self.stream_id
        {
            let _ = self.stop_stream(id);
        }
    }

    fn on_refresh(&mut self) -> bool {
        self.stream_active && self.config.mining_enabled
    }
}

// ============================================================================
// UI Configurable
// ============================================================================

impl UiConfigurable for MiningPluginFlexForge {
    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema::new()
            .with_field(
                ConfigField::toggle("mining_enabled", "Enable Mining", false)
                    .with_description("Enable cryptocurrency mining")
                    .with_group("General"),
            )
            .with_field(
                ConfigField::text("pool_address", "Pool Address")
                    .with_description("Mining pool stratum address")
                    .with_group("Pool"),
            )
            .with_field(
                ConfigField::text("wallet_address", "Wallet Address")
                    .with_description("Your wallet address for payouts")
                    .with_group("Pool"),
            )
            .with_field(
                ConfigField::text("worker_name", "Worker Name")
                    .with_description("Identifier for this mining worker")
                    .with_group("Pool"),
            )
            .with_field(
                ConfigField::toggle("cpu_mining", "CPU Mining", false)
                    .with_description("Use CPU for mining")
                    .with_group("Hardware"),
            )
            .with_field(
                ConfigField::toggle("gpu_mining", "GPU Mining", true)
                    .with_description("Use GPU for mining")
                    .with_group("Hardware"),
            )
            .with_field(
                ConfigField::number("cpu_threads", "CPU Threads", 2.0, 1.0, 32.0)
                    .with_description("Number of CPU threads for mining")
                    .with_group("Hardware"),
            )
            .with_field(
                ConfigField::number("gpu_intensity", "GPU Intensity (%)", 70.0, 10.0, 100.0)
                    .with_description("GPU mining intensity")
                    .with_group("Hardware"),
            )
            .with_field(
                ConfigField::number("temp_limit", "Temperature Limit (°C)", 80.0, 50.0, 95.0)
                    .with_description("Maximum allowed GPU temperature")
                    .with_group("Safety"),
            )
    }

    fn on_config_changed(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "mining_enabled" => {
                self.config.mining_enabled = value == "true";
                Ok(())
            },
            "pool_address" => {
                self.config.pool_address = value.to_string();
                Ok(())
            },
            "wallet_address" => {
                self.config.wallet_address = value.to_string();
                Ok(())
            },
            "worker_name" => {
                self.config.worker_name = value.to_string();
                Ok(())
            },
            "cpu_mining" => {
                self.config.cpu_mining = value == "true";
                Ok(())
            },
            "gpu_mining" => {
                self.config.gpu_mining = value == "true";
                Ok(())
            },
            "cpu_threads" => {
                let threads: f64 = value.parse().map_err(|_| "Invalid thread count")?;
                self.config.cpu_threads = threads as u8;
                Ok(())
            },
            "gpu_intensity" => {
                let intensity: f64 = value.parse().map_err(|_| "Invalid intensity")?;
                self.config.gpu_intensity = intensity as u8;
                Ok(())
            },
            "temp_limit" => {
                let limit: f64 = value.parse().map_err(|_| "Invalid temperature")?;
                self.config.temp_limit = limit as u8;
                Ok(())
            },
            _ => Err(format!("Unknown configuration key: {key}")),
        }
    }

    fn apply_config(&mut self, config: &[(String, String)]) -> Result<(), String> {
        for (key, value) in config {
            self.on_config_changed(key, value)?;
        }
        Ok(())
    }

    fn get_current_config(&self) -> Vec<(String, String)> {
        vec![
            (
                String::from("mining_enabled"),
                self.config.mining_enabled.to_string(),
            ),
            (
                String::from("pool_address"),
                self.config.pool_address.clone(),
            ),
            (
                String::from("wallet_address"),
                self.config.wallet_address.clone(),
            ),
            (String::from("worker_name"), self.config.worker_name.clone()),
            (
                String::from("cpu_mining"),
                self.config.cpu_mining.to_string(),
            ),
            (
                String::from("gpu_mining"),
                self.config.gpu_mining.to_string(),
            ),
            (
                String::from("cpu_threads"),
                self.config.cpu_threads.to_string(),
            ),
            (
                String::from("gpu_intensity"),
                self.config.gpu_intensity.to_string(),
            ),
            (
                String::from("temp_limit"),
                self.config.temp_limit.to_string(),
            ),
        ]
    }

    fn reset_to_defaults(&mut self) {
        self.config = MiningUiConfig::default();
    }
}

// ============================================================================
// Streaming Capable - Real-time mining stats
// ============================================================================

impl StreamingCapable for MiningPluginFlexForge {
    fn is_streaming(&self) -> bool {
        self.stream_active
    }

    fn start_stream(&mut self) -> Result<u64, String> {
        if self.stream_active {
            return Err("Stream already active".to_string());
        }
        let id = self.next_stream_id();
        self.stream_id = Some(id);
        self.stream_active = true;
        Ok(id)
    }

    fn stop_stream(&mut self, stream_id: u64) -> Result<(), String> {
        if !self.stream_active {
            return Err("No active stream".to_string());
        }
        if self.stream_id != Some(stream_id) {
            return Err("Invalid stream ID".to_string());
        }
        self.stream_active = false;
        self.stream_id = None;
        Ok(())
    }

    fn target_fps(&self) -> u32 {
        // Mining stats update at 2 FPS for efficiency
        2
    }

    fn render_frame(&mut self, stream_id: u64, _delta_ms: f64) -> bool {
        if !self.stream_active || self.stream_id != Some(stream_id) {
            return false;
        }
        true
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "full-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_panel_id() {
        let panel = MiningPluginFlexForge::new();
        assert_eq!(panel.panel_id(), "essentia_mining_plugin");
    }

    #[test]
    fn test_default_config() {
        let panel = MiningPluginFlexForge::new();
        assert!(!panel.config.mining_enabled); // Disabled by default
        assert!(panel.config.gpu_mining);
        assert!(!panel.config.cpu_mining);
        assert_eq!(panel.config.gpu_intensity, 70);
        assert_eq!(panel.config.temp_limit, 80);
    }

    #[test]
    fn test_config_validation() {
        let mut panel = MiningPluginFlexForge::new();
        assert!(panel.on_config_changed("mining_enabled", "true").is_ok());
        assert!(panel.config.mining_enabled);
        assert!(panel.on_config_changed("gpu_intensity", "85").is_ok());
        assert_eq!(panel.config.gpu_intensity, 85);
        assert!(
            panel
                .on_config_changed("pool_address", "stratum+tcp://pool.example.com:3333")
                .is_ok()
        );
        assert!(panel.on_config_changed("invalid_key", "value").is_err());
    }

    #[test]
    fn test_streaming() {
        let mut panel = MiningPluginFlexForge::new();
        panel.config.mining_enabled = true;
        assert!(!panel.is_streaming());

        let id = panel.start_stream().expect("test assertion");
        assert!(panel.is_streaming());
        assert!(panel.render_frame(id, 500.0));

        panel.stop_stream(id).expect("test assertion");
        assert!(!panel.is_streaming());
        assert!(!panel.render_frame(id, 500.0));
    }

    #[test]
    fn test_stats_update() {
        let mut panel = MiningPluginFlexForge::new();
        let stats = MiningDisplayStats {
            hashrate:         1_500_000.0,
            shares_accepted:  100,
            shares_rejected:  2,
            difficulty:       65536.0,
            earnings_per_day: 50000,
            gpu_temp:         68,
            cpu_usage:        25,
        };
        panel.update_stats(stats);
        assert_eq!(panel.stats.shares_accepted, 100);
        assert_eq!(panel.stats.gpu_temp, 68);
    }
}
