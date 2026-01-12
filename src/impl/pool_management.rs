//! GAP-220-F-001: Pool Management
//!
//! Implements mining pool connection management, failover support,
//! and multi-pool coordination.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::errors::{MiningError, MiningResult};

/// Pool priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolPriority {
    /// Primary pool - highest priority.
    Primary = 0,
    /// Backup pool - used when primary fails.
    Backup = 1,
    /// Emergency pool - last resort.
    Emergency = 2,
}

/// Pool configuration.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Pool identifier.
    pub id: String,
    /// Pool URL.
    pub url: String,
    /// Worker name.
    pub worker: String,
    /// Worker password (optional).
    pub password: Option<String>,
    /// Pool priority.
    pub priority: PoolPriority,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Keep-alive interval.
    pub keepalive_interval: Duration,
    /// Maximum retries before failover.
    pub max_retries: u32,
    /// Retry delay.
    pub retry_delay: Duration,
    /// Pool fee percentage.
    pub fee_percent: f64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            url: "stratum+tcp://localhost:3333".to_string(),
            worker: "worker1".to_string(),
            password: None,
            priority: PoolPriority::Primary,
            connect_timeout: Duration::from_secs(30),
            keepalive_interval: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            fee_percent: 1.0,
        }
    }
}

/// Pool connection state.
#[derive(Debug, Clone)]
pub struct PoolState {
    /// Pool configuration.
    pub config: PoolConfig,
    /// Current status.
    pub status: PoolStatus,
    /// Connection attempts.
    pub connection_attempts: u32,
    /// Successful connections.
    pub successful_connections: u32,
    /// Shares submitted.
    pub shares_submitted: u64,
    /// Shares accepted.
    pub shares_accepted: u64,
    /// Shares rejected.
    pub shares_rejected: u64,
    /// Last connection time.
    pub last_connected: Option<Instant>,
    /// Last share time.
    pub last_share: Option<Instant>,
    /// Connection latency.
    pub latency_ms: Option<u64>,
}

impl PoolState {
    /// Creates a new pool state.
    #[must_use]
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            status: PoolStatus::Disconnected,
            connection_attempts: 0,
            successful_connections: 0,
            shares_submitted: 0,
            shares_accepted: 0,
            shares_rejected: 0,
            last_connected: None,
            last_share: None,
            latency_ms: None,
        }
    }

    /// Returns acceptance rate.
    #[must_use]
    pub fn acceptance_rate(&self) -> f64 {
        if self.shares_submitted == 0 {
            return 1.0;
        }
        self.shares_accepted as f64 / self.shares_submitted as f64
    }

    /// Returns rejection rate.
    #[must_use]
    pub fn rejection_rate(&self) -> f64 {
        if self.shares_submitted == 0 {
            return 0.0;
        }
        self.shares_rejected as f64 / self.shares_submitted as f64
    }

    /// Returns stale rate (not accepted or rejected).
    #[must_use]
    pub fn stale_rate(&self) -> f64 {
        if self.shares_submitted == 0 {
            return 0.0;
        }
        let accounted = self.shares_accepted + self.shares_rejected;
        (self.shares_submitted - accounted) as f64 / self.shares_submitted as f64
    }
}

/// Pool status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolStatus {
    /// Disconnected.
    Disconnected,
    /// Connecting.
    Connecting,
    /// Connected and ready.
    Connected,
    /// Subscribed to pool.
    Subscribed,
    /// Authorized and mining.
    Authorized,
    /// Connection failed.
    Failed { reason: String },
    /// Temporarily disabled.
    Disabled { reason: String },
}

/// Pool manager configuration.
#[derive(Debug, Clone)]
pub struct PoolManagerConfig {
    /// Enable automatic failover.
    pub auto_failover: bool,
    /// Failover timeout.
    pub failover_timeout: Duration,
    /// Health check interval.
    pub health_check_interval: Duration,
    /// Maximum pools.
    pub max_pools: usize,
    /// Minimum acceptance rate before failover.
    pub min_acceptance_rate: f64,
}

impl Default for PoolManagerConfig {
    fn default() -> Self {
        Self {
            auto_failover: true,
            failover_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(60),
            max_pools: 10,
            min_acceptance_rate: 0.95,
        }
    }
}

/// Pool manager for multi-pool support.
#[derive(Debug)]
pub struct PoolManager {
    /// Configuration.
    config: PoolManagerConfig,
    /// Pool states.
    pools: Arc<Mutex<HashMap<String, PoolState>>>,
    /// Currently active pool.
    active_pool: Arc<Mutex<Option<String>>>,
    /// Failover history.
    failover_history: Arc<Mutex<Vec<FailoverEvent>>>,
}

impl PoolManager {
    /// Creates a new pool manager.
    #[must_use]
    pub fn new(config: PoolManagerConfig) -> Self {
        Self {
            config,
            pools: Arc::new(Mutex::new(HashMap::new())),
            active_pool: Arc::new(Mutex::new(None)),
            failover_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Adds a pool.
    pub fn add_pool(&self, config: PoolConfig) -> MiningResult<()> {
        let mut pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        if pools.len() >= self.config.max_pools {
            return Err(MiningError::Configuration(format!(
                "Maximum number of pools ({}) exceeded",
                self.config.max_pools
            )));
        }

        let id = config.id.clone();
        pools.insert(id, PoolState::new(config));
        Ok(())
    }

    /// Removes a pool.
    pub fn remove_pool(&self, pool_id: &str) -> MiningResult<()> {
        let mut pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        pools.remove(pool_id).ok_or_else(|| {
            MiningError::PoolConnection(format!("Pool not found: {}", pool_id))
        })?;

        // Check if this was the active pool
        let mut active = self.active_pool.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on active_pool".to_string())
        })?;

        if active.as_ref() == Some(&pool_id.to_string()) {
            *active = None;
        }

        Ok(())
    }

    /// Gets pool state.
    pub fn get_pool(&self, pool_id: &str) -> MiningResult<Option<PoolState>> {
        let pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        Ok(pools.get(pool_id).cloned())
    }

    /// Gets all pools.
    pub fn all_pools(&self) -> MiningResult<Vec<PoolState>> {
        let pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        Ok(pools.values().cloned().collect())
    }

    /// Gets active pool ID.
    pub fn active_pool_id(&self) -> MiningResult<Option<String>> {
        let active = self.active_pool.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on active_pool".to_string())
        })?;

        Ok(active.clone())
    }

    /// Sets active pool.
    pub fn set_active_pool(&self, pool_id: &str) -> MiningResult<()> {
        // Verify pool exists
        {
            let pools = self.pools.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on pools".to_string())
            })?;

            if !pools.contains_key(pool_id) {
                return Err(MiningError::PoolConnection(format!("Pool not found: {}", pool_id)));
            }
        }

        let mut active = self.active_pool.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on active_pool".to_string())
        })?;

        *active = Some(pool_id.to_string());
        Ok(())
    }

    /// Updates pool status.
    pub fn update_status(&self, pool_id: &str, status: PoolStatus) -> MiningResult<()> {
        let mut pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        let pool = pools.get_mut(pool_id).ok_or_else(|| {
            MiningError::PoolConnection(format!("Pool not found: {}", pool_id))
        })?;

        pool.status = status;
        Ok(())
    }

    /// Records a share submission.
    pub fn record_share(&self, pool_id: &str, accepted: bool) -> MiningResult<()> {
        let mut pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        let pool = pools.get_mut(pool_id).ok_or_else(|| {
            MiningError::PoolConnection(format!("Pool not found: {}", pool_id))
        })?;

        pool.shares_submitted += 1;
        if accepted {
            pool.shares_accepted += 1;
        } else {
            pool.shares_rejected += 1;
        }
        pool.last_share = Some(Instant::now());

        Ok(())
    }

    /// Selects best pool based on priority and health.
    pub fn select_best_pool(&self) -> MiningResult<Option<String>> {
        let pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        let mut candidates: Vec<_> = pools
            .values()
            .filter(|p| !matches!(p.status, PoolStatus::Failed { .. } | PoolStatus::Disabled { .. }))
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Sort by priority, then by acceptance rate
        candidates.sort_by(|a, b| {
            match a.config.priority.cmp(&b.config.priority) {
                std::cmp::Ordering::Equal => {
                    b.acceptance_rate().partial_cmp(&a.acceptance_rate()).unwrap_or(std::cmp::Ordering::Equal)
                },
                other => other,
            }
        });

        Ok(candidates.first().map(|p| p.config.id.clone()))
    }

    /// Triggers failover to next best pool.
    pub fn failover(&self, reason: &str) -> MiningResult<Option<String>> {
        let current = self.active_pool_id()?;

        // Mark current as failed
        if let Some(ref pool_id) = current {
            self.update_status(pool_id, PoolStatus::Failed {
                reason: reason.to_string(),
            })?;
        }

        // Select new pool
        let new_pool = self.select_best_pool()?;

        if let Some(ref pool_id) = new_pool {
            self.set_active_pool(pool_id)?;

            // Record failover event
            let mut history = self.failover_history.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on failover_history".to_string())
            })?;

            history.push(FailoverEvent {
                timestamp: Instant::now(),
                from_pool: current,
                to_pool: pool_id.clone(),
                reason: reason.to_string(),
            });
        }

        Ok(new_pool)
    }

    /// Gets failover history.
    pub fn failover_history(&self) -> MiningResult<Vec<FailoverEvent>> {
        let history = self.failover_history.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on failover_history".to_string())
        })?;

        Ok(history.clone())
    }

    /// Performs health check on all pools.
    pub fn health_check(&self) -> MiningResult<HealthCheckResult> {
        let pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        let mut healthy = Vec::new();
        let mut unhealthy = Vec::new();

        for pool in pools.values() {
            if pool.acceptance_rate() >= self.config.min_acceptance_rate
                && !matches!(pool.status, PoolStatus::Failed { .. } | PoolStatus::Disabled { .. })
            {
                healthy.push(pool.config.id.clone());
            } else {
                unhealthy.push(pool.config.id.clone());
            }
        }

        Ok(HealthCheckResult {
            healthy_pools: healthy,
            unhealthy_pools: unhealthy,
            checked_at: Instant::now(),
        })
    }

    /// Gets aggregate statistics across all pools.
    pub fn aggregate_stats(&self) -> MiningResult<AggregatePoolStats> {
        let pools = self.pools.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on pools".to_string())
        })?;

        let mut stats = AggregatePoolStats::default();

        for pool in pools.values() {
            stats.total_shares_submitted += pool.shares_submitted;
            stats.total_shares_accepted += pool.shares_accepted;
            stats.total_shares_rejected += pool.shares_rejected;
            stats.pools_count += 1;

            match pool.status {
                PoolStatus::Authorized | PoolStatus::Subscribed | PoolStatus::Connected => {
                    stats.connected_pools += 1;
                },
                _ => {},
            }
        }

        if stats.total_shares_submitted > 0 {
            stats.overall_acceptance_rate =
                stats.total_shares_accepted as f64 / stats.total_shares_submitted as f64;
        }

        Ok(stats)
    }
}

/// Failover event record.
#[derive(Debug, Clone)]
pub struct FailoverEvent {
    /// When failover occurred.
    pub timestamp: Instant,
    /// Previous pool.
    pub from_pool: Option<String>,
    /// New pool.
    pub to_pool: String,
    /// Reason for failover.
    pub reason: String,
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Healthy pool IDs.
    pub healthy_pools: Vec<String>,
    /// Unhealthy pool IDs.
    pub unhealthy_pools: Vec<String>,
    /// When check was performed.
    pub checked_at: Instant,
}

/// Aggregate statistics across pools.
#[derive(Debug, Clone, Default)]
pub struct AggregatePoolStats {
    /// Total pools.
    pub pools_count: usize,
    /// Connected pools.
    pub connected_pools: usize,
    /// Total shares submitted.
    pub total_shares_submitted: u64,
    /// Total shares accepted.
    pub total_shares_accepted: u64,
    /// Total shares rejected.
    pub total_shares_rejected: u64,
    /// Overall acceptance rate.
    pub overall_acceptance_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_state() {
        let config = PoolConfig::default();
        let state = PoolState::new(config);
        assert_eq!(state.acceptance_rate(), 1.0);
        assert_eq!(state.status, PoolStatus::Disconnected);
    }

    #[test]
    fn test_pool_manager() {
        let manager = PoolManager::new(PoolManagerConfig::default());
        let config = PoolConfig {
            id: "pool1".to_string(),
            ..Default::default()
        };
        assert!(manager.add_pool(config).is_ok());
        assert!(manager.get_pool("pool1").unwrap().is_some());
    }

    #[test]
    fn test_select_best_pool() {
        let manager = PoolManager::new(PoolManagerConfig::default());

        manager.add_pool(PoolConfig {
            id: "primary".to_string(),
            priority: PoolPriority::Primary,
            ..Default::default()
        }).unwrap();

        manager.add_pool(PoolConfig {
            id: "backup".to_string(),
            priority: PoolPriority::Backup,
            ..Default::default()
        }).unwrap();

        let best = manager.select_best_pool().unwrap();
        assert_eq!(best, Some("primary".to_string()));
    }

    #[test]
    fn test_failover() {
        let manager = PoolManager::new(PoolManagerConfig::default());

        manager.add_pool(PoolConfig {
            id: "primary".to_string(),
            priority: PoolPriority::Primary,
            ..Default::default()
        }).unwrap();

        manager.add_pool(PoolConfig {
            id: "backup".to_string(),
            priority: PoolPriority::Backup,
            ..Default::default()
        }).unwrap();

        manager.set_active_pool("primary").unwrap();

        let new_pool = manager.failover("Connection lost").unwrap();
        assert_eq!(new_pool, Some("backup".to_string()));
    }
}
