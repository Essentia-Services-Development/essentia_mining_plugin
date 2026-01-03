//! Hardware detection integration for mining optimization.
//!
//! This module integrates with `essentia_hwdetect` to determine optimal
//! mining parameters based on available hardware capabilities.

use crate::errors::MiningResult;
use crate::traits::MiningHardwareTrait;

/// Hardware profile for mining optimization.
#[derive(Debug, Clone)]
pub struct MiningHardwareProfile {
    /// Number of physical CPU cores.
    pub physical_cores:     usize,
    /// Number of logical CPU cores (with hyperthreading).
    pub logical_cores:      usize,
    /// Available system memory in bytes.
    pub available_memory:   u64,
    /// CPU supports SHA extensions.
    pub has_sha_extensions: bool,
    /// CPU supports AVX2.
    pub has_avx2:           bool,
    /// GPU available for mining.
    pub gpu_available:      bool,
    /// GPU compute capability (if available).
    pub gpu_compute_units:  Option<u32>,
    /// Performance tier (1-5, 5 being highest).
    pub performance_tier:   u8,
}

impl MiningHardwareProfile {
    /// Detect hardware capabilities for mining.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::HardwareDetection` if hardware detection fails.
    pub fn detect() -> MiningResult<Self> {
        // In production, this would call essentia_hwdetect
        // For now, use basic detection
        Self::detect_basic()
    }

    /// Basic hardware detection without external dependencies.
    fn detect_basic() -> MiningResult<Self> {
        // Detect CPU cores
        let logical_cores = detect_cpu_cores();
        let physical_cores = logical_cores / 2; // Assume hyperthreading

        // Detect memory (rough estimate)
        let available_memory = detect_available_memory();

        // CPU feature detection would go here
        // For now, assume conservative values
        let has_sha_extensions = false;
        let has_avx2 = false;

        // Calculate performance tier
        let performance_tier = Self::calculate_tier(logical_cores, available_memory);

        Ok(Self {
            physical_cores: physical_cores.max(1),
            logical_cores: logical_cores.max(1),
            available_memory,
            has_sha_extensions,
            has_avx2,
            gpu_available: false,
            gpu_compute_units: None,
            performance_tier,
        })
    }

    /// Calculate performance tier based on hardware.
    fn calculate_tier(cores: usize, memory: u64) -> u8 {
        let core_score = match cores {
            0..=2 => 1,
            3..=4 => 2,
            5..=8 => 3,
            9..=16 => 4,
            _ => 5,
        };

        let mem_gb = memory / (1024 * 1024 * 1024);
        let mem_score = match mem_gb {
            0..=4 => 1,
            5..=8 => 2,
            9..=16 => 3,
            17..=32 => 4,
            _ => 5,
        };

        ((core_score + mem_score) / 2) as u8
    }
}

impl MiningHardwareTrait for MiningHardwareProfile {
    fn physical_cores(&self) -> usize {
        self.physical_cores
    }

    fn logical_cores(&self) -> usize {
        self.logical_cores
    }

    fn is_suitable_for_mining(&self) -> bool {
        self.physical_cores >= 2 && self.available_memory >= 2 * 1024 * 1024 * 1024
    }

    fn recommended_threads(&self, max_percentage: u8) -> usize {
        let available = self.physical_cores;
        let target = (available as f64 * (max_percentage as f64 / 100.0)) as usize;
        target.max(1).min(available)
    }
}

/// Detect number of CPU cores.
fn detect_cpu_cores() -> usize {
    // Simple heuristic: check environment or use default
    // In production, use essentia_hwdetect

    #[cfg(target_os = "windows")]
    {
        // Try to get from environment
        if let Ok(val) = std::env::var("NUMBER_OF_PROCESSORS")
            && let Ok(n) = val.parse::<usize>()
        {
            return n;
        }
    }

    // Default fallback
    4
}

/// Detect available system memory.
fn detect_available_memory() -> u64 {
    // In production, use essentia_hwdetect
    // Default to 8GB assumption
    8 * 1024 * 1024 * 1024
}

impl Default for MiningHardwareProfile {
    fn default() -> Self {
        Self::detect().unwrap_or(Self {
            physical_cores:     2,
            logical_cores:      4,
            available_memory:   4 * 1024 * 1024 * 1024,
            has_sha_extensions: false,
            has_avx2:           false,
            gpu_available:      false,
            gpu_compute_units:  None,
            performance_tier:   2,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let profile = MiningHardwareProfile::detect();
        assert!(profile.is_ok());
    }

    #[test]
    fn test_recommended_threads() {
        let profile =
            MiningHardwareProfile { physical_cores: 8, logical_cores: 16, ..Default::default() };

        assert_eq!(profile.recommended_threads(50), 4);
        assert_eq!(profile.recommended_threads(25), 2);
        assert_eq!(profile.recommended_threads(100), 8);
    }

    #[test]
    fn test_performance_tier() {
        let tier = MiningHardwareProfile::calculate_tier(16, 32 * 1024 * 1024 * 1024);
        assert!(tier >= 4);
    }
}
