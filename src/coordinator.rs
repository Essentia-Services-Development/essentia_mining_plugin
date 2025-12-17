//! Mining coordinator for managing mining threads.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    config::MiningConfig,
    errors::{MiningError, MiningResult},
    hardware::MiningHardwareProfile,
    sha256::double_sha256,
    types::{BlockHeader, HashTarget, MiningJob, MiningStats},
};

/// Mining coordinator that manages background mining threads.
pub struct MiningCoordinator {
    config:       MiningConfig,
    hardware:     MiningHardwareProfile,
    running:      Arc<AtomicBool>,
    total_hashes: Arc<AtomicU64>,
    shares_found: Arc<AtomicU64>,
}

impl MiningCoordinator {
    /// Create a new mining coordinator.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::HardwareDetection` if hardware detection fails.
    pub fn new(config: MiningConfig) -> MiningResult<Self> {
        let hardware = MiningHardwareProfile::detect()?;

        if !hardware.is_suitable_for_mining() {
            return Err(MiningError::HardwareDetection(
                "Hardware does not meet minimum requirements for mining".into(),
            ));
        }

        Ok(Self {
            config,
            hardware,
            running: Arc::new(AtomicBool::new(false)),
            total_hashes: Arc::new(AtomicU64::new(0)),
            shares_found: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Start mining with the given job.
    ///
    /// # Errors
    ///
    /// Returns `MiningError::Coordinator` if mining is already running.
    pub fn start(&self, job: MiningJob) -> MiningResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(MiningError::Coordinator("Mining already running".into()));
        }

        self.running.store(true, Ordering::SeqCst);

        let thread_count = self.config.effective_thread_count(self.hardware.physical_cores);
        let nonce_range = u32::MAX / thread_count as u32;

        for thread_id in 0..thread_count {
            let start_nonce = thread_id as u32 * nonce_range;
            let end_nonce = if thread_id == thread_count - 1 {
                u32::MAX
            } else {
                start_nonce + nonce_range - 1
            };

            let running = Arc::clone(&self.running);
            let total_hashes = Arc::clone(&self.total_hashes);
            let shares_found = Arc::clone(&self.shares_found);
            let header = job.header.clone();
            let target = job.target.clone();

            std::thread::spawn(move || {
                Self::mining_thread(
                    running,
                    total_hashes,
                    shares_found,
                    header,
                    target,
                    start_nonce,
                    end_nonce,
                );
            });
        }

        Ok(())
    }

    /// Stop all mining threads.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if mining is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get current mining statistics.
    pub fn stats(&self) -> MiningStats {
        MiningStats {
            total_hashes: self.total_hashes.load(Ordering::Relaxed),
            shares_found: self.shares_found.load(Ordering::Relaxed),
            ..Default::default()
        }
    }

    /// Mining thread function.
    fn mining_thread(
        running: Arc<AtomicBool>, total_hashes: Arc<AtomicU64>, shares_found: Arc<AtomicU64>,
        mut header: BlockHeader, target: HashTarget, start_nonce: u32, end_nonce: u32,
    ) {
        let mut nonce = start_nonce;
        let mut batch_count = 0u64;

        while running.load(Ordering::Relaxed) && nonce < end_nonce {
            header.nonce = nonce;
            let serialized = header.serialize();
            let hash = double_sha256(&serialized);

            // Check if hash meets target
            if target.is_valid_hash(&hash) {
                shares_found.fetch_add(1, Ordering::Relaxed);
            }

            batch_count += 1;

            // Update total hashes periodically
            if batch_count >= 1000 {
                total_hashes.fetch_add(batch_count, Ordering::Relaxed);
                batch_count = 0;
            }

            nonce = nonce.saturating_add(1);
        }

        // Final update
        if batch_count > 0 {
            total_hashes.fetch_add(batch_count, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let config = MiningConfig::default();
        let coordinator = MiningCoordinator::new(config);
        assert!(coordinator.is_ok());
    }

    #[test]
    fn test_coordinator_not_running_initially() {
        let config = MiningConfig::default();
        let coordinator = MiningCoordinator::new(config).unwrap();
        assert!(!coordinator.is_running());
    }

    #[test]
    fn test_stats_initial() {
        let config = MiningConfig::default();
        let coordinator = MiningCoordinator::new(config).unwrap();
        let stats = coordinator.stats();
        assert_eq!(stats.total_hashes, 0);
        assert_eq!(stats.shares_found, 0);
    }
}
