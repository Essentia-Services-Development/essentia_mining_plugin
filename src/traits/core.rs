//! Core mining traits.

use crate::{
    errors::MiningResult,
    types::{MiningJob, MiningStats, PoolConnection},
};

/// Trait for hardware detection and profiling.
pub trait MiningHardwareTrait: Send + Sync {
    /// Get number of physical CPU cores.
    fn physical_cores(&self) -> usize;

    /// Get number of logical CPU cores.
    fn logical_cores(&self) -> usize;

    /// Check if hardware is suitable for mining.
    fn is_suitable_for_mining(&self) -> bool;

    /// Get recommended thread count based on max CPU percentage.
    fn recommended_threads(&self, max_percentage: u8) -> usize;
}

/// Trait for mining coordination.
pub trait MiningCoordinatorTrait: Send + Sync {
    /// Start mining with the given job.
    fn start(&self, job: MiningJob) -> MiningResult<()>;

    /// Stop all mining threads.
    fn stop(&self);

    /// Check if mining is currently running.
    fn is_running(&self) -> bool;

    /// Get current mining statistics.
    fn stats(&self) -> MiningStats;
}

/// Trait for pool client implementations.
pub trait PoolClientTrait: Send + Sync {
    /// Connect to the mining pool.
    fn connect(&mut self) -> MiningResult<()>;

    /// Disconnect from the pool.
    fn disconnect(&mut self);

    /// Get current connection state.
    fn state(&self) -> &PoolConnection;

    /// Check if connected.
    fn is_connected(&self) -> bool;

    /// Get current mining job from pool.
    fn get_job(&self) -> MiningResult<Option<MiningJob>>;

    /// Submit a share to the pool.
    fn submit_share(
        &self, job_id: &str, extranonce2: &[u8], ntime: u32, nonce: u32,
    ) -> MiningResult<bool>;
}
