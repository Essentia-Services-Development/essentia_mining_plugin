//! Mining plugin configuration.

/// Configuration for the mining plugin.
#[derive(Debug, Clone)]
pub struct MiningConfig {
    /// Maximum CPU usage percentage (1-100).
    pub max_cpu_percentage:    u8,
    /// Run mining at background priority.
    pub background_priority:   bool,
    /// Number of mining threads (0 = auto-detect).
    pub thread_count:          usize,
    /// Pool URL for stratum connection.
    pub pool_url:              Option<String>,
    /// Worker name for pool.
    pub worker_name:           String,
    /// Enable GPU mining if available.
    pub gpu_enabled:           bool,
    /// Minimum hashrate before pausing (0 = no minimum).
    pub min_hashrate:          f64,
    /// Auto-pause when system is busy.
    pub auto_pause_on_load:    bool,
    /// CPU temperature threshold for throttling (Celsius).
    pub thermal_throttle_temp: Option<u8>,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            max_cpu_percentage:    25, // Conservative default
            background_priority:   true,
            thread_count:          0, // Auto-detect
            pool_url:              None,
            worker_name:           String::from("essentia_worker"),
            gpu_enabled:           false,
            min_hashrate:          0.0,
            auto_pause_on_load:    true,
            thermal_throttle_temp: Some(80),
        }
    }
}

impl MiningConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum CPU usage percentage.
    pub fn with_max_cpu_usage(mut self, percentage: u8) -> Self {
        self.max_cpu_percentage = percentage.min(100);
        self
    }

    /// Set background priority mode.
    pub fn with_background_priority(mut self, enabled: bool) -> Self {
        self.background_priority = enabled;
        self
    }

    /// Set explicit thread count.
    pub fn with_thread_count(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }

    /// Set pool URL for stratum connection.
    pub fn with_pool_url(mut self, url: impl Into<String>) -> Self {
        self.pool_url = Some(url;
        self
    }

    /// Set worker name.
    pub fn with_worker_name(mut self, name: impl Into<String>) -> Self {
        self.worker_name = name;
        self
    }

    /// Enable/disable GPU mining.
    pub fn with_gpu_enabled(mut self, enabled: bool) -> Self {
        self.gpu_enabled = enabled;
        self
    }

    /// Calculate effective thread count based on config and hardware.
    pub fn effective_thread_count(&self, available_cores: usize) -> usize {
        if self.thread_count > 0 {
            self.thread_count.min(available_cores)
        } else {
            // Auto-detect: use percentage of available cores
            let target =
                (available_cores as f64 * (self.max_cpu_percentage as f64 / 100.0)) as usize;
            target.max(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_thread_count_auto() {
        let config = MiningConfig::default().with_max_cpu_usage(50);
        assert_eq!(config.effective_thread_count(8), 4);
    }

    #[test]
    fn test_effective_thread_count_explicit() {
        let config = MiningConfig::default().with_thread_count(2);
        assert_eq!(config.effective_thread_count(8), 2);
    }

    #[test]
    fn test_builder_pattern() {
        let config = MiningConfig::new()
            .with_max_cpu_usage(75)
            .with_pool_url("stratum+tcp://pool.example.com:3333")
            .with_worker_name("my_worker");

        assert_eq!(config.max_cpu_percentage, 75);
        assert!(config.pool_url.is_some());
        assert_eq!(config.worker_name, "my_worker");
    }
}


