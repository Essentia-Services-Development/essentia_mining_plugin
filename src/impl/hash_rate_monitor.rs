//! GAP-220-F-002: Hash Rate Monitor
//!
//! Implements real-time hash rate monitoring, statistics tracking,
//! and performance analysis for mining operations.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::errors::{MiningError, MiningResult};

/// Hash rate sample.
#[derive(Debug, Clone, Copy)]
pub struct HashRateSample {
    /// Timestamp.
    pub timestamp: Instant,
    /// Hashes since last sample.
    pub hashes: u64,
    /// Duration since last sample.
    pub duration: Duration,
}

impl HashRateSample {
    /// Calculates hash rate in H/s.
    #[must_use]
    pub fn hash_rate(&self) -> f64 {
        if self.duration.is_zero() {
            return 0.0;
        }
        self.hashes as f64 / self.duration.as_secs_f64()
    }
}

/// Hash rate statistics.
#[derive(Debug, Clone, Default)]
pub struct HashRateStats {
    /// Current hash rate in H/s.
    pub current: f64,
    /// Average hash rate in H/s.
    pub average: f64,
    /// Peak hash rate in H/s.
    pub peak: f64,
    /// Minimum hash rate in H/s.
    pub minimum: f64,
    /// Standard deviation.
    pub std_dev: f64,
    /// Total hashes computed.
    pub total_hashes: u64,
    /// Monitoring duration.
    pub monitoring_duration: Duration,
    /// Sample count.
    pub sample_count: usize,
}

/// Hash rate display units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashRateUnit {
    /// Hashes per second.
    HashPerSec,
    /// Kilohashes per second.
    KiloHashPerSec,
    /// Megahashes per second.
    MegaHashPerSec,
    /// Gigahashes per second.
    GigaHashPerSec,
    /// Terahashes per second.
    TeraHashPerSec,
}

impl HashRateUnit {
    /// Converts hash rate to this unit.
    #[must_use]
    pub fn convert(&self, hash_rate: f64) -> f64 {
        match self {
            Self::HashPerSec => hash_rate,
            Self::KiloHashPerSec => hash_rate / 1_000.0,
            Self::MegaHashPerSec => hash_rate / 1_000_000.0,
            Self::GigaHashPerSec => hash_rate / 1_000_000_000.0,
            Self::TeraHashPerSec => hash_rate / 1_000_000_000_000.0,
        }
    }

    /// Gets the unit suffix.
    #[must_use]
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::HashPerSec => "H/s",
            Self::KiloHashPerSec => "KH/s",
            Self::MegaHashPerSec => "MH/s",
            Self::GigaHashPerSec => "GH/s",
            Self::TeraHashPerSec => "TH/s",
        }
    }

    /// Auto-selects best unit for display.
    #[must_use]
    pub fn auto_select(hash_rate: f64) -> Self {
        if hash_rate >= 1_000_000_000_000.0 {
            Self::TeraHashPerSec
        } else if hash_rate >= 1_000_000_000.0 {
            Self::GigaHashPerSec
        } else if hash_rate >= 1_000_000.0 {
            Self::MegaHashPerSec
        } else if hash_rate >= 1_000.0 {
            Self::KiloHashPerSec
        } else {
            Self::HashPerSec
        }
    }
}

/// Monitor configuration.
#[derive(Debug, Clone)]
pub struct HashRateMonitorConfig {
    /// Sample interval.
    pub sample_interval: Duration,
    /// Maximum samples to keep.
    pub max_samples: usize,
    /// Moving average window.
    pub moving_average_window: usize,
    /// Alert threshold (percentage drop).
    pub alert_threshold: f64,
    /// Minimum samples for statistics.
    pub min_samples_for_stats: usize,
}

impl Default for HashRateMonitorConfig {
    fn default() -> Self {
        Self {
            sample_interval: Duration::from_secs(1),
            max_samples: 3600, // 1 hour at 1 sample/sec
            moving_average_window: 60,
            alert_threshold: 0.20, // 20% drop
            min_samples_for_stats: 10,
        }
    }
}

/// Hash rate monitor.
#[derive(Debug)]
pub struct HashRateMonitor {
    /// Configuration.
    config: HashRateMonitorConfig,
    /// Sample history.
    samples: Arc<Mutex<VecDeque<HashRateSample>>>,
    /// Last sample time.
    last_sample_time: Arc<Mutex<Option<Instant>>>,
    /// Last hash count.
    last_hash_count: Arc<Mutex<u64>>,
    /// Total hashes.
    total_hashes: Arc<Mutex<u64>>,
    /// Start time.
    start_time: Arc<Mutex<Option<Instant>>>,
    /// Peak hash rate.
    peak_hash_rate: Arc<Mutex<f64>>,
    /// Minimum hash rate (excluding zero).
    min_hash_rate: Arc<Mutex<f64>>,
    /// Alert callbacks.
    alerts: Arc<Mutex<Vec<Alert>>>,
}

impl HashRateMonitor {
    /// Creates a new hash rate monitor.
    #[must_use]
    pub fn new(config: HashRateMonitorConfig) -> Self {
        Self {
            config,
            samples: Arc::new(Mutex::new(VecDeque::new())),
            last_sample_time: Arc::new(Mutex::new(None)),
            last_hash_count: Arc::new(Mutex::new(0)),
            total_hashes: Arc::new(Mutex::new(0)),
            start_time: Arc::new(Mutex::new(None)),
            peak_hash_rate: Arc::new(Mutex::new(0.0)),
            min_hash_rate: Arc::new(Mutex::new(f64::MAX)),
            alerts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Starts monitoring.
    pub fn start(&self) -> MiningResult<()> {
        let mut start_time = self.start_time.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on start_time".to_string())
        })?;
        *start_time = Some(Instant::now());

        let mut last_sample = self.last_sample_time.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on last_sample_time".to_string())
        })?;
        *last_sample = Some(Instant::now());

        Ok(())
    }

    /// Records hash count update.
    pub fn record(&self, current_hash_count: u64) -> MiningResult<Option<HashRateSample>> {
        let now = Instant::now();

        let mut last_sample_time = self.last_sample_time.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on last_sample_time".to_string())
        })?;

        let mut last_hash_count = self.last_hash_count.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on last_hash_count".to_string())
        })?;

        // Update total hashes
        {
            let mut total = self.total_hashes.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on total_hashes".to_string())
            })?;
            *total = current_hash_count;
        }

        // Check if we should record a sample
        let should_sample = last_sample_time
            .map(|t| now.duration_since(t) >= self.config.sample_interval)
            .unwrap_or(true);

        if !should_sample {
            return Ok(None);
        }

        // Calculate delta
        let hashes_delta = current_hash_count.saturating_sub(*last_hash_count);
        let time_delta = last_sample_time
            .map(|t| now.duration_since(t))
            .unwrap_or(Duration::from_secs(1));

        let sample = HashRateSample {
            timestamp: now,
            hashes: hashes_delta,
            duration: time_delta,
        };

        // Update tracking
        *last_sample_time = Some(now);
        *last_hash_count = current_hash_count;

        // Drop locks before adding sample
        drop(last_sample_time);
        drop(last_hash_count);

        // Add sample
        self.add_sample(sample)?;

        // Update peak/min
        self.update_extremes(sample.hash_rate())?;

        // Check for alerts
        self.check_alerts(&sample)?;

        Ok(Some(sample))
    }

    fn add_sample(&self, sample: HashRateSample) -> MiningResult<()> {
        let mut samples = self.samples.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on samples".to_string())
        })?;

        samples.push_back(sample);

        // Trim old samples
        while samples.len() > self.config.max_samples {
            samples.pop_front();
        }

        Ok(())
    }

    fn update_extremes(&self, hash_rate: f64) -> MiningResult<()> {
        let mut peak = self.peak_hash_rate.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on peak_hash_rate".to_string())
        })?;

        if hash_rate > *peak {
            *peak = hash_rate;
        }

        drop(peak);

        let mut min = self.min_hash_rate.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on min_hash_rate".to_string())
        })?;

        if hash_rate > 0.0 && hash_rate < *min {
            *min = hash_rate;
        }

        Ok(())
    }

    fn check_alerts(&self, sample: &HashRateSample) -> MiningResult<()> {
        let stats = self.statistics()?;

        // Check for significant drop
        if stats.sample_count >= self.config.min_samples_for_stats {
            let current = sample.hash_rate();
            let threshold = stats.average * (1.0 - self.config.alert_threshold);

            if current < threshold && current > 0.0 {
                let mut alerts = self.alerts.lock().map_err(|_| {
                    MiningError::Coordinator("Failed to acquire lock on alerts".to_string())
                })?;

                alerts.push(Alert {
                    timestamp: Instant::now(),
                    alert_type: AlertType::HashRateDrop,
                    message: format!(
                        "Hash rate dropped to {:.2} H/s (average: {:.2} H/s)",
                        current, stats.average
                    ),
                    value: current,
                    threshold,
                });
            }
        }

        Ok(())
    }

    /// Gets current statistics.
    pub fn statistics(&self) -> MiningResult<HashRateStats> {
        let samples = self.samples.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on samples".to_string())
        })?;

        if samples.is_empty() {
            return Ok(HashRateStats::default());
        }

        let hash_rates: Vec<f64> = samples.iter().map(|s| s.hash_rate()).collect();
        let total_hashes: u64 = samples.iter().map(|s| s.hashes).sum();

        let current = hash_rates.last().copied().unwrap_or(0.0);
        let average = hash_rates.iter().sum::<f64>() / hash_rates.len() as f64;

        let peak = self.peak_hash_rate.lock().map(|p| *p).unwrap_or(0.0);
        let minimum = self.min_hash_rate.lock().map(|m| if *m == f64::MAX { 0.0 } else { *m }).unwrap_or(0.0);

        // Calculate standard deviation
        let variance = hash_rates
            .iter()
            .map(|x| {
                let diff = x - average;
                diff * diff
            })
            .sum::<f64>()
            / hash_rates.len() as f64;
        let std_dev = variance.sqrt();

        let monitoring_duration = samples
            .front()
            .and_then(|first| samples.back().map(|last| last.timestamp.duration_since(first.timestamp)))
            .unwrap_or(Duration::ZERO);

        Ok(HashRateStats {
            current,
            average,
            peak,
            minimum,
            std_dev,
            total_hashes,
            monitoring_duration,
            sample_count: samples.len(),
        })
    }

    /// Gets moving average over configured window.
    pub fn moving_average(&self) -> MiningResult<f64> {
        let samples = self.samples.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on samples".to_string())
        })?;

        let window_size = self.config.moving_average_window.min(samples.len());
        if window_size == 0 {
            return Ok(0.0);
        }

        let sum: f64 = samples
            .iter()
            .rev()
            .take(window_size)
            .map(|s| s.hash_rate())
            .sum();

        Ok(sum / window_size as f64)
    }

    /// Gets effective hash rate (considering actual accepted shares).
    pub fn effective_hash_rate(&self, accepted_shares: u64, share_difficulty: f64) -> f64 {
        let stats = self.statistics().unwrap_or_default();
        if stats.monitoring_duration.is_zero() {
            return 0.0;
        }

        // Effective = (accepted_shares * share_difficulty * 2^32) / time_seconds
        let time_secs = stats.monitoring_duration.as_secs_f64();
        (accepted_shares as f64 * share_difficulty * 4_294_967_296.0) / time_secs
    }

    /// Gets recent samples.
    pub fn recent_samples(&self, count: usize) -> MiningResult<Vec<HashRateSample>> {
        let samples = self.samples.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on samples".to_string())
        })?;

        Ok(samples.iter().rev().take(count).copied().collect())
    }

    /// Gets alerts.
    pub fn alerts(&self) -> MiningResult<Vec<Alert>> {
        let alerts = self.alerts.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on alerts".to_string())
        })?;

        Ok(alerts.clone())
    }

    /// Clears alerts.
    pub fn clear_alerts(&self) -> MiningResult<()> {
        let mut alerts = self.alerts.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on alerts".to_string())
        })?;

        alerts.clear();
        Ok(())
    }

    /// Formats hash rate for display.
    #[must_use]
    pub fn format_hash_rate(hash_rate: f64) -> String {
        let unit = HashRateUnit::auto_select(hash_rate);
        format!("{:.2} {}", unit.convert(hash_rate), unit.suffix())
    }

    /// Resets the monitor.
    pub fn reset(&self) -> MiningResult<()> {
        {
            let mut samples = self.samples.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on samples".to_string())
            })?;
            samples.clear();
        }

        {
            let mut total = self.total_hashes.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on total_hashes".to_string())
            })?;
            *total = 0;
        }

        {
            let mut peak = self.peak_hash_rate.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on peak_hash_rate".to_string())
            })?;
            *peak = 0.0;
        }

        {
            let mut min = self.min_hash_rate.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on min_hash_rate".to_string())
            })?;
            *min = f64::MAX;
        }

        self.clear_alerts()?;

        Ok(())
    }
}

/// Alert information.
#[derive(Debug, Clone)]
pub struct Alert {
    /// When alert occurred.
    pub timestamp: Instant,
    /// Alert type.
    pub alert_type: AlertType,
    /// Alert message.
    pub message: String,
    /// Value that triggered alert.
    pub value: f64,
    /// Threshold that was exceeded.
    pub threshold: f64,
}

/// Alert types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    /// Hash rate dropped significantly.
    HashRateDrop,
    /// Hash rate spike.
    HashRateSpike,
    /// Hardware temperature warning.
    TemperatureWarning,
    /// Hardware error.
    HardwareError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_rate_sample() {
        let sample = HashRateSample {
            timestamp: Instant::now(),
            hashes: 1000,
            duration: Duration::from_secs(1),
        };
        assert!((sample.hash_rate() - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_hash_rate_unit() {
        assert_eq!(HashRateUnit::auto_select(500.0), HashRateUnit::HashPerSec);
        assert_eq!(HashRateUnit::auto_select(5_000.0), HashRateUnit::KiloHashPerSec);
        assert_eq!(HashRateUnit::auto_select(5_000_000.0), HashRateUnit::MegaHashPerSec);
    }

    #[test]
    fn test_hash_rate_monitor() {
        let config = HashRateMonitorConfig {
            sample_interval: Duration::from_millis(1),
            ..Default::default()
        };
        let monitor = HashRateMonitor::new(config);

        monitor.start().unwrap();
        std::thread::sleep(Duration::from_millis(5));

        let sample = monitor.record(1000).unwrap();
        assert!(sample.is_some());

        let stats = monitor.statistics().unwrap();
        assert_eq!(stats.sample_count, 1);
    }

    #[test]
    fn test_format_hash_rate() {
        assert_eq!(HashRateMonitor::format_hash_rate(1_500_000.0), "1.50 MH/s");
        assert_eq!(HashRateMonitor::format_hash_rate(2_500_000_000.0), "2.50 GH/s");
    }
}
