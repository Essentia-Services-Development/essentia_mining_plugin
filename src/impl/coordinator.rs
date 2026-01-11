//! Mining coordinator for managing mining threads.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    errors::{MiningError, MiningResult},
    r#impl::{MiningConfig, MiningHardwareProfile, double_sha256},
    traits::MiningCoordinatorTrait,
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
        use crate::traits::MiningHardwareTrait;

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

impl MiningCoordinatorTrait for MiningCoordinator {
    fn start(&self, job: MiningJob) -> MiningResult<()> {
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

            // SSOP-EXEMPT(std::thread): Mining uses CPU-intensive threads for SHA256 hashing;
            // async runtime not suitable for compute-bound work
            #[allow(clippy::let_underscore_future)]
            let _ = essentia_async_runtime::spawn(async move {
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

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn stats(&self) -> MiningStats {
        MiningStats {
            total_hashes: self.total_hashes.load(Ordering::Relaxed),
            shares_found: self.shares_found.load(Ordering::Relaxed),
            ..Default::default()
        }
    }
}

#[cfg(all(test, feature = "full-tests"))]
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
        let coordinator = MiningCoordinator::new(config).expect("test assertion");
        assert!(!coordinator.is_running());
    }

    #[test]
    fn test_stats_initial() {
        let config = MiningConfig::default();
        let coordinator = MiningCoordinator::new(config).expect("test assertion");
        let stats = coordinator.stats();
        assert_eq!(stats.total_hashes, 0);
        assert_eq!(stats.shares_found, 0);
    }
}
