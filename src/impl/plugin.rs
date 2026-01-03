//! Mining plugin implementation.

use crate::{
    errors::{MiningError, MiningResult},
    r#impl::{MiningConfig, MiningCoordinator, MiningHardwareProfile, StratumClient},
    traits::{MiningCoordinatorTrait, PoolClientTrait},
    types::{MiningStats, PoolConnection},
};

/// Main mining plugin interface.
pub struct MiningPlugin {
    config:           MiningConfig,
    coordinator:      Option<MiningCoordinator>,
    stratum_client:   Option<StratumClient>,
    hardware_profile: MiningHardwareProfile,
}

impl MiningPlugin {
    /// Create a new mining plugin with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::HardwareDetection` if hardware detection fails.
    /// Returns `MiningError::Configuration` if configuration is invalid.
    pub fn new(config: MiningConfig) -> MiningResult<Self> {
        let hardware_profile = MiningHardwareProfile::detect()?;

        // Validate configuration
        if config.max_cpu_percentage == 0 || config.max_cpu_percentage > 100 {
            return Err(MiningError::Configuration(
                "CPU percentage must be between 1 and 100".into(),
            ));
        }

        Ok(Self { config, coordinator: None, stratum_client: None, hardware_profile })
    }

    /// Get hardware profile.
    #[must_use]
    pub fn hardware_profile(&self) -> &MiningHardwareProfile {
        &self.hardware_profile
    }

    /// Get current configuration.
    #[must_use]
    pub fn config(&self) -> &MiningConfig {
        &self.config
    }

    /// Update configuration.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::Configuration` if new config is invalid.
    pub fn update_config(&mut self, config: MiningConfig) -> MiningResult<()> {
        if config.max_cpu_percentage == 0 || config.max_cpu_percentage > 100 {
            return Err(MiningError::Configuration(
                "CPU percentage must be between 1 and 100".into(),
            ));
        }
        self.config = config;
        Ok(())
    }

    /// Connect to mining pool.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::PoolConnection` if connection fails.
    /// Returns `MiningError::Configuration` if no pool URL is configured.
    pub fn connect_to_pool(&mut self) -> MiningResult<()> {
        let pool_url = self
            .config
            .pool_url
            .clone()
            .ok_or_else(|| MiningError::Configuration("No pool URL configured".into()))?;

        let mut client = StratumClient::new(pool_url, &self.config.worker_name);
        client.connect()?;
        self.stratum_client = Some(client);

        Ok(())
    }

    /// Disconnect from mining pool.
    pub fn disconnect_from_pool(&mut self) {
        if let Some(ref mut client) = self.stratum_client {
            client.disconnect();
        }
        self.stratum_client = None;
    }

    /// Get pool connection state.
    #[must_use]
    pub fn pool_connection_state(&self) -> PoolConnection {
        self.stratum_client
            .as_ref()
            .map(|c| c.state().clone())
            .unwrap_or(PoolConnection::Disconnected)
    }

    /// Start background mining.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::Coordinator` if mining fails to start.
    pub fn start_background_mining(&mut self) -> MiningResult<()> {
        if self.coordinator.is_some() {
            return Err(MiningError::Coordinator("Mining already active".into()));
        }

        let coordinator = MiningCoordinator::new(self.config.clone())?;

        // Get job from pool or create test job
        if let Some(ref client) = self.stratum_client
            && let Some(job) = client.get_job()?
        {
            coordinator.start(job)?;
        }

        self.coordinator = Some(coordinator);
        Ok(())
    }

    /// Stop background mining.
    pub fn stop_background_mining(&mut self) {
        if let Some(ref coordinator) = self.coordinator {
            coordinator.stop();
        }
        self.coordinator = None;
    }

    /// Check if mining is active.
    #[must_use]
    pub fn is_mining(&self) -> bool {
        self.coordinator.as_ref().map(|c| c.is_running()).unwrap_or(false)
    }

    /// Get current mining statistics.
    #[must_use]
    pub fn stats(&self) -> MiningStats {
        self.coordinator.as_ref().map(|c| c.stats()).unwrap_or_default()
    }
}

impl Drop for MiningPlugin {
    fn drop(&mut self) {
        self.stop_background_mining();
        self.disconnect_from_pool();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let config = MiningConfig::default();
        let plugin = MiningPlugin::new(config);
        assert!(plugin.is_ok());
    }

    #[test]
    fn test_plugin_not_mining_initially() {
        let plugin = MiningPlugin::new(MiningConfig::default()).expect("test assertion");
        assert!(!plugin.is_mining());
    }

    #[test]
    fn test_invalid_cpu_percentage() {
        let config = MiningConfig::default().with_max_cpu_usage(0);
        let result = MiningPlugin::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_hardware_profile_access() {
        let plugin = MiningPlugin::new(MiningConfig::default()).expect("test assertion");
        let profile = plugin.hardware_profile();
        assert!(profile.logical_cores > 0);
    }

    #[test]
    fn test_pool_not_connected_initially() {
        let plugin = MiningPlugin::new(MiningConfig::default()).expect("test assertion");
        assert!(matches!(
            plugin.pool_connection_state(),
            PoolConnection::Disconnected
        ));
    }
}
