//! GAP-220-F-003: Reward Distribution
//!
//! Implements mining reward calculation, distribution tracking,
//! and payout management.

#![allow(clippy::double_ended_iterator_last)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::errors::{MiningError, MiningResult};

/// Reward calculation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardMethod {
    /// Pay Per Share - fixed payment per valid share.
    Pps,
    /// Pay Per Last N Shares - proportional to recent shares.
    Pplns,
    /// Proportional - shares since last block.
    Prop,
    /// Score-based - time-weighted shares.
    Score,
    /// Solo mining - full block reward.
    Solo,
}

impl RewardMethod {
    /// Gets human-readable name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pps => "Pay Per Share",
            Self::Pplns => "Pay Per Last N Shares",
            Self::Prop => "Proportional",
            Self::Score => "Score-Based",
            Self::Solo => "Solo Mining",
        }
    }
}

/// Mining share record.
#[derive(Debug, Clone)]
pub struct Share {
    /// Share ID.
    pub id: u64,
    /// Worker ID.
    pub worker_id: String,
    /// Share difficulty.
    pub difficulty: f64,
    /// Timestamp.
    pub timestamp: Instant,
    /// Whether share was accepted.
    pub accepted: bool,
    /// Block height when submitted.
    pub block_height: u64,
}

/// Block reward information.
#[derive(Debug, Clone)]
pub struct BlockReward {
    /// Block height.
    pub height: u64,
    /// Block hash.
    pub hash: String,
    /// Block reward in satoshis.
    pub reward_sats: u64,
    /// Transaction fees in satoshis.
    pub fees_sats: u64,
    /// When block was found.
    pub found_at: Instant,
    /// Whether reward is mature (spendable).
    pub is_mature: bool,
    /// Confirmations.
    pub confirmations: u32,
}

impl BlockReward {
    /// Returns total reward (block + fees).
    #[must_use]
    pub fn total_sats(&self) -> u64 {
        self.reward_sats + self.fees_sats
    }

    /// Returns total reward in BTC.
    #[must_use]
    pub fn total_btc(&self) -> f64 {
        self.total_sats() as f64 / 100_000_000.0
    }
}

/// Worker statistics.
#[derive(Debug, Clone, Default)]
pub struct WorkerStats {
    /// Worker ID.
    pub worker_id: String,
    /// Total shares submitted.
    pub shares_submitted: u64,
    /// Accepted shares.
    pub shares_accepted: u64,
    /// Rejected shares.
    pub shares_rejected: u64,
    /// Total difficulty contributed.
    pub total_difficulty: f64,
    /// Pending reward in satoshis.
    pub pending_reward_sats: u64,
    /// Paid reward in satoshis.
    pub paid_reward_sats: u64,
    /// Last share time.
    pub last_share: Option<Instant>,
    /// Active since.
    pub active_since: Option<Instant>,
}

impl WorkerStats {
    /// Creates new worker stats.
    #[must_use]
    pub fn new(worker_id: impl Into<String>) -> Self {
        Self {
            worker_id: worker_id.into(),
            ..Default::default()
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

    /// Returns total earnings in BTC.
    #[must_use]
    pub fn total_earnings_btc(&self) -> f64 {
        (self.pending_reward_sats + self.paid_reward_sats) as f64 / 100_000_000.0
    }
}

/// Payout record.
#[derive(Debug, Clone)]
pub struct Payout {
    /// Payout ID.
    pub id: u64,
    /// Worker ID.
    pub worker_id: String,
    /// Amount in satoshis.
    pub amount_sats: u64,
    /// Destination address.
    pub address: String,
    /// Transaction ID (if sent).
    pub txid: Option<String>,
    /// Payout status.
    pub status: PayoutStatus,
    /// Created time.
    pub created_at: Instant,
    /// Completed time.
    pub completed_at: Option<Instant>,
}

/// Payout status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayoutStatus {
    /// Pending - not yet processed.
    Pending,
    /// Processing - being sent.
    Processing,
    /// Completed - transaction confirmed.
    Completed,
    /// Failed - transaction failed.
    Failed { reason: String },
}

/// Reward distribution configuration.
#[derive(Debug, Clone)]
pub struct RewardConfig {
    /// Reward calculation method.
    pub method: RewardMethod,
    /// Pool fee percentage.
    pub pool_fee_percent: f64,
    /// Minimum payout threshold in satoshis.
    pub min_payout_sats: u64,
    /// PPLNS window size (for PPLNS method).
    pub pplns_window: usize,
    /// Block maturity confirmations.
    pub maturity_confirmations: u32,
    /// Score decay factor (for Score method).
    pub score_decay: f64,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            method: RewardMethod::Pplns,
            pool_fee_percent: 1.0,
            min_payout_sats: 10_000, // 0.0001 BTC
            pplns_window: 100_000,
            maturity_confirmations: 100,
            score_decay: 0.9999,
        }
    }
}

/// Reward distribution manager.
#[derive(Debug)]
pub struct RewardDistributor {
    /// Configuration.
    config: RewardConfig,
    /// Worker statistics.
    workers: Arc<Mutex<HashMap<String, WorkerStats>>>,
    /// Share history (for PPLNS).
    shares: Arc<Mutex<Vec<Share>>>,
    /// Block rewards.
    blocks: Arc<Mutex<Vec<BlockReward>>>,
    /// Pending payouts.
    payouts: Arc<Mutex<Vec<Payout>>>,
    /// Share counter.
    share_counter: Arc<Mutex<u64>>,
    /// Payout counter.
    payout_counter: Arc<Mutex<u64>>,
    /// Current block height.
    current_height: Arc<Mutex<u64>>,
}

impl RewardDistributor {
    /// Creates a new reward distributor.
    #[must_use]
    pub fn new(config: RewardConfig) -> Self {
        Self {
            config,
            workers: Arc::new(Mutex::new(HashMap::new())),
            shares: Arc::new(Mutex::new(Vec::new())),
            blocks: Arc::new(Mutex::new(Vec::new())),
            payouts: Arc::new(Mutex::new(Vec::new())),
            share_counter: Arc::new(Mutex::new(0)),
            payout_counter: Arc::new(Mutex::new(0)),
            current_height: Arc::new(Mutex::new(0)),
        }
    }

    /// Registers a worker.
    pub fn register_worker(&self, worker_id: impl Into<String>) -> MiningResult<()> {
        let worker_id = worker_id.into();
        let mut workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        workers.entry(worker_id.clone()).or_insert_with(|| {
            let mut stats = WorkerStats::new(worker_id);
            stats.active_since = Some(Instant::now());
            stats
        });

        Ok(())
    }

    /// Records a share.
    pub fn record_share(
        &self,
        worker_id: impl Into<String>,
        difficulty: f64,
        accepted: bool,
    ) -> MiningResult<u64> {
        let worker_id = worker_id.into();
        let now = Instant::now();

        // Get share ID
        let share_id = {
            let mut counter = self.share_counter.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on share_counter".to_string())
            })?;
            *counter += 1;
            *counter
        };

        // Get current height
        let block_height = self.current_height.lock().map(|h| *h).unwrap_or(0);

        // Create share
        let share = Share {
            id: share_id,
            worker_id: worker_id.clone(),
            difficulty,
            timestamp: now,
            accepted,
            block_height,
        };

        // Add to history
        {
            let mut shares = self.shares.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on shares".to_string())
            })?;

            shares.push(share);

            // Trim old shares for PPLNS
            if self.config.method == RewardMethod::Pplns && shares.len() > self.config.pplns_window {
                let excess = shares.len() - self.config.pplns_window;
                shares.drain(0..excess);
            }
        }

        // Update worker stats
        {
            let mut workers = self.workers.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on workers".to_string())
            })?;

            let stats = workers.entry(worker_id).or_insert_with(|| {
                let mut s = WorkerStats::new("");
                s.active_since = Some(now);
                s
            });

            stats.shares_submitted += 1;
            if accepted {
                stats.shares_accepted += 1;
                stats.total_difficulty += difficulty;
            } else {
                stats.shares_rejected += 1;
            }
            stats.last_share = Some(now);
        }

        Ok(share_id)
    }

    /// Records a found block.
    pub fn record_block(&self, reward: BlockReward) -> MiningResult<()> {
        let mut blocks = self.blocks.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on blocks".to_string())
        })?;

        blocks.push(reward);
        Ok(())
    }

    /// Updates block confirmations.
    pub fn update_confirmations(&self, height: u64, confirmations: u32) -> MiningResult<()> {
        let mut blocks = self.blocks.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on blocks".to_string())
        })?;

        for block in blocks.iter_mut() {
            if block.height == height {
                block.confirmations = confirmations;
                block.is_mature = confirmations >= self.config.maturity_confirmations;
            }
        }

        Ok(())
    }

    /// Calculates rewards for a block using configured method.
    pub fn calculate_rewards(&self, block_height: u64) -> MiningResult<HashMap<String, u64>> {
        let blocks = self.blocks.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on blocks".to_string())
        })?;

        let block = blocks
            .iter()
            .find(|b| b.height == block_height)
            .ok_or_else(|| MiningError::Configuration(format!("Block {} not found", block_height)))?;

        // Calculate pool fee
        let total_reward = block.total_sats();
        let pool_fee = (total_reward as f64 * self.config.pool_fee_percent / 100.0) as u64;
        let distributable = total_reward - pool_fee;

        match self.config.method {
            RewardMethod::Pps => self.calculate_pps(distributable),
            RewardMethod::Pplns => self.calculate_pplns(distributable, block_height),
            RewardMethod::Prop => self.calculate_proportional(distributable, block_height),
            RewardMethod::Score => self.calculate_score(distributable),
            RewardMethod::Solo => self.calculate_solo(distributable),
        }
    }

    fn calculate_pps(&self, _distributable: u64) -> MiningResult<HashMap<String, u64>> {
        // PPS pays per share regardless of blocks
        // This is typically handled differently - reward per share is fixed
        Ok(HashMap::new())
    }

    fn calculate_pplns(&self, distributable: u64, block_height: u64) -> MiningResult<HashMap<String, u64>> {
        let shares = self.shares.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on shares".to_string())
        })?;

        // Get shares up to this block
        let relevant_shares: Vec<_> = shares
            .iter()
            .filter(|s| s.accepted && s.block_height <= block_height)
            .take(self.config.pplns_window)
            .collect();

        if relevant_shares.is_empty() {
            return Ok(HashMap::new());
        }

        // Calculate total difficulty
        let total_difficulty: f64 = relevant_shares.iter().map(|s| s.difficulty).sum();

        // Calculate rewards
        let mut rewards = HashMap::new();

        for share in &relevant_shares {
            let proportion = share.difficulty / total_difficulty;
            let reward = (distributable as f64 * proportion) as u64;

            *rewards.entry(share.worker_id.clone()).or_insert(0) += reward;
        }

        Ok(rewards)
    }

    fn calculate_proportional(&self, distributable: u64, block_height: u64) -> MiningResult<HashMap<String, u64>> {
        let shares = self.shares.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on shares".to_string())
        })?;

        // Get shares for this block round
        let relevant_shares: Vec<_> = shares
            .iter()
            .filter(|s| s.accepted && s.block_height == block_height)
            .collect();

        if relevant_shares.is_empty() {
            return Ok(HashMap::new());
        }

        let total_difficulty: f64 = relevant_shares.iter().map(|s| s.difficulty).sum();
        let mut rewards = HashMap::new();

        for share in &relevant_shares {
            let proportion = share.difficulty / total_difficulty;
            let reward = (distributable as f64 * proportion) as u64;
            *rewards.entry(share.worker_id.clone()).or_insert(0) += reward;
        }

        Ok(rewards)
    }

    fn calculate_score(&self, distributable: u64) -> MiningResult<HashMap<String, u64>> {
        let shares = self.shares.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on shares".to_string())
        })?;

        let now = Instant::now();
        let mut scores: HashMap<String, f64> = HashMap::new();

        // Calculate time-weighted scores
        for share in shares.iter().filter(|s| s.accepted) {
            let age_secs = now.duration_since(share.timestamp).as_secs_f64();
            let score = share.difficulty * self.config.score_decay.powf(age_secs);
            *scores.entry(share.worker_id.clone()).or_insert(0.0) += score;
        }

        let total_score: f64 = scores.values().sum();
        if total_score == 0.0 {
            return Ok(HashMap::new());
        }

        let rewards: HashMap<String, u64> = scores
            .into_iter()
            .map(|(worker, score)| {
                let reward = (distributable as f64 * score / total_score) as u64;
                (worker, reward)
            })
            .collect();

        Ok(rewards)
    }

    fn calculate_solo(&self, distributable: u64) -> MiningResult<HashMap<String, u64>> {
        // Solo mining - all reward goes to finder
        let shares = self.shares.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on shares".to_string())
        })?;

        // Get the most recent share (block finder)
        let finder = shares
            .iter()
            .filter(|s| s.accepted)
            .last()
            .map(|s| s.worker_id.clone());

        let mut rewards = HashMap::new();
        if let Some(worker_id) = finder {
            rewards.insert(worker_id, distributable);
        }

        Ok(rewards)
    }

    /// Distributes rewards to worker balances.
    pub fn distribute_rewards(&self, rewards: &HashMap<String, u64>) -> MiningResult<()> {
        let mut workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        for (worker_id, amount) in rewards {
            if let Some(stats) = workers.get_mut(worker_id) {
                stats.pending_reward_sats += amount;
            }
        }

        Ok(())
    }

    /// Creates a payout for a worker.
    pub fn create_payout(&self, worker_id: &str, address: &str) -> MiningResult<Option<Payout>> {
        let mut workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        let stats = workers.get_mut(worker_id).ok_or_else(|| {
            MiningError::Configuration(format!("Worker not found: {}", worker_id))
        })?;

        // Check minimum threshold
        if stats.pending_reward_sats < self.config.min_payout_sats {
            return Ok(None);
        }

        // Get payout ID
        let payout_id = {
            let mut counter = self.payout_counter.lock().map_err(|_| {
                MiningError::Coordinator("Failed to acquire lock on payout_counter".to_string())
            })?;
            *counter += 1;
            *counter
        };

        let amount = stats.pending_reward_sats;
        stats.pending_reward_sats = 0;

        let payout = Payout {
            id: payout_id,
            worker_id: worker_id.to_string(),
            amount_sats: amount,
            address: address.to_string(),
            txid: None,
            status: PayoutStatus::Pending,
            created_at: Instant::now(),
            completed_at: None,
        };

        // Drop lock before adding payout
        drop(workers);

        // Add to pending payouts
        let mut payouts = self.payouts.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on payouts".to_string())
        })?;

        payouts.push(payout.clone());

        Ok(Some(payout))
    }

    /// Completes a payout.
    pub fn complete_payout(&self, payout_id: u64, txid: &str) -> MiningResult<()> {
        let mut payouts = self.payouts.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on payouts".to_string())
        })?;

        let payout = payouts
            .iter_mut()
            .find(|p| p.id == payout_id)
            .ok_or_else(|| MiningError::Configuration(format!("Payout {} not found", payout_id)))?;

        payout.txid = Some(txid.to_string());
        payout.status = PayoutStatus::Completed;
        payout.completed_at = Some(Instant::now());

        // Update worker stats
        let mut workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        if let Some(stats) = workers.get_mut(&payout.worker_id) {
            stats.paid_reward_sats += payout.amount_sats;
        }

        Ok(())
    }

    /// Gets worker statistics.
    pub fn get_worker_stats(&self, worker_id: &str) -> MiningResult<Option<WorkerStats>> {
        let workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        Ok(workers.get(worker_id).cloned())
    }

    /// Gets all worker statistics.
    pub fn all_worker_stats(&self) -> MiningResult<Vec<WorkerStats>> {
        let workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        Ok(workers.values().cloned().collect())
    }

    /// Gets pending payouts.
    pub fn pending_payouts(&self) -> MiningResult<Vec<Payout>> {
        let payouts = self.payouts.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on payouts".to_string())
        })?;

        Ok(payouts
            .iter()
            .filter(|p| p.status == PayoutStatus::Pending)
            .cloned()
            .collect())
    }

    /// Gets pool statistics.
    pub fn pool_stats(&self) -> MiningResult<PoolStats> {
        let workers = self.workers.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on workers".to_string())
        })?;

        let blocks = self.blocks.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on blocks".to_string())
        })?;

        let total_workers = workers.len();
        let total_shares: u64 = workers.values().map(|w| w.shares_accepted).sum();
        let total_difficulty: f64 = workers.values().map(|w| w.total_difficulty).sum();
        let total_pending: u64 = workers.values().map(|w| w.pending_reward_sats).sum();
        let total_paid: u64 = workers.values().map(|w| w.paid_reward_sats).sum();
        let blocks_found = blocks.len();
        let mature_blocks = blocks.iter().filter(|b| b.is_mature).count();

        Ok(PoolStats {
            total_workers,
            total_shares,
            total_difficulty,
            blocks_found,
            mature_blocks,
            total_pending_sats: total_pending,
            total_paid_sats: total_paid,
        })
    }

    /// Sets current block height.
    pub fn set_block_height(&self, height: u64) -> MiningResult<()> {
        let mut current = self.current_height.lock().map_err(|_| {
            MiningError::Coordinator("Failed to acquire lock on current_height".to_string())
        })?;
        *current = height;
        Ok(())
    }
}

/// Pool-wide statistics.
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total registered workers.
    pub total_workers: usize,
    /// Total accepted shares.
    pub total_shares: u64,
    /// Total difficulty contributed.
    pub total_difficulty: f64,
    /// Blocks found.
    pub blocks_found: usize,
    /// Mature (spendable) blocks.
    pub mature_blocks: usize,
    /// Total pending rewards in satoshis.
    pub total_pending_sats: u64,
    /// Total paid rewards in satoshis.
    pub total_paid_sats: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_stats() {
        let mut stats = WorkerStats::new("worker1");
        stats.shares_submitted = 100;
        stats.shares_accepted = 95;
        assert!((stats.acceptance_rate() - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_block_reward() {
        let reward = BlockReward {
            height: 100,
            hash: "abc".to_string(),
            reward_sats: 625_000_000,
            fees_sats: 1_000_000,
            found_at: Instant::now(),
            is_mature: true,
            confirmations: 100,
        };
        assert_eq!(reward.total_sats(), 626_000_000);
        assert!((reward.total_btc() - 6.26).abs() < 0.01);
    }

    #[test]
    fn test_reward_distributor() {
        let config = RewardConfig::default();
        let distributor = RewardDistributor::new(config);

        distributor.register_worker("worker1").unwrap();
        distributor.record_share("worker1", 1.0, true).unwrap();

        let stats = distributor.get_worker_stats("worker1").unwrap().unwrap();
        assert_eq!(stats.shares_accepted, 1);
    }

    #[test]
    fn test_pplns_calculation() {
        let config = RewardConfig {
            method: RewardMethod::Pplns,
            pool_fee_percent: 0.0,
            ..Default::default()
        };
        let distributor = RewardDistributor::new(config);

        distributor.set_block_height(100).unwrap();
        distributor.register_worker("worker1").unwrap();
        distributor.register_worker("worker2").unwrap();

        // Worker 1 contributes 2/3 of difficulty
        distributor.record_share("worker1", 2.0, true).unwrap();
        distributor.record_share("worker2", 1.0, true).unwrap();

        // Record block
        distributor.record_block(BlockReward {
            height: 100,
            hash: "abc".to_string(),
            reward_sats: 300,
            fees_sats: 0,
            found_at: Instant::now(),
            is_mature: true,
            confirmations: 100,
        }).unwrap();

        let rewards = distributor.calculate_rewards(100).unwrap();
        assert_eq!(rewards.get("worker1"), Some(&200));
        assert_eq!(rewards.get("worker2"), Some(&100));
    }
}
