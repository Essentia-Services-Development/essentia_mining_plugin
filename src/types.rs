//! Mining plugin type definitions.

/// Statistics for mining operations.
#[derive(Debug, Clone, Default)]
pub struct MiningStats {
    /// Total hashes computed.
    pub total_hashes: u64,
    /// Hashes per second.
    pub hashrate: f64,
    /// Number of valid shares found.
    pub shares_found: u64,
    /// Number of shares accepted by pool.
    pub shares_accepted: u64,
    /// Number of shares rejected by pool.
    pub shares_rejected: u64,
    /// Current difficulty.
    pub difficulty: f64,
    /// Estimated time to find block (seconds).
    pub estimated_time_to_block: Option<f64>,
}

/// Bitcoin block header for mining.
#[derive(Debug, Clone)]
pub struct BlockHeader {
    /// Block version.
    pub version: u32,
    /// Previous block hash (32 bytes).
    pub prev_block_hash: [u8; 32],
    /// Merkle root (32 bytes).
    pub merkle_root: [u8; 32],
    /// Block timestamp.
    pub timestamp: u32,
    /// Target difficulty bits.
    pub bits: u32,
    /// Nonce being mined.
    pub nonce: u32,
}

impl BlockHeader {
    /// Serialize header for hashing (80 bytes).
    pub fn serialize(&self) -> [u8; 80] {
        let mut result = [0u8; 80];
        result[0..4].copy_from_slice(&self.version.to_le_bytes());
        result[4..36].copy_from_slice(&self.prev_block_hash);
        result[36..68].copy_from_slice(&self.merkle_root);
        result[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
        result[72..76].copy_from_slice(&self.bits.to_le_bytes());
        result[76..80].copy_from_slice(&self.nonce.to_le_bytes());
        result
    }
}

/// Target hash threshold for valid blocks.
#[derive(Debug, Clone)]
pub struct HashTarget {
    /// Target as 32-byte big-endian value.
    pub target: [u8; 32],
}

impl HashTarget {
    /// Create target from difficulty bits.
    pub fn from_bits(bits: u32) -> Self {
        let mut target = [0u8; 32];
        let exponent = ((bits >> 24) & 0xFF) as usize;
        let mantissa = bits & 0x00FFFFFF;

        if exponent >= 3 && exponent <= 32 {
            let start = 32 - exponent;
            target[start] = ((mantissa >> 16) & 0xFF) as u8;
            if start + 1 < 32 {
                target[start + 1] = ((mantissa >> 8) & 0xFF) as u8;
            }
            if start + 2 < 32 {
                target[start + 2] = (mantissa & 0xFF) as u8;
            }
        }

        Self { target }
    }

    /// Check if hash meets target (hash <= target).
    pub fn is_valid_hash(&self, hash: &[u8; 32]) -> bool {
        for i in 0..32 {
            if hash[i] < self.target[i] {
                return true;
            }
            if hash[i] > self.target[i] {
                return false;
            }
        }
        true // Equal is valid
    }
}

/// Nonce value for mining.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nonce(pub u32);

impl Nonce {
    /// Increment nonce, returning None on overflow.
    pub fn increment(&mut self) -> Option<u32> {
        self.0 = self.0.checked_add(1)?;
        Some(self.0)
    }
}

/// Mining job from pool.
#[derive(Debug, Clone)]
pub struct MiningJob {
    /// Job identifier.
    pub job_id: String,
    /// Block header template.
    pub header: BlockHeader,
    /// Target for this job.
    pub target: HashTarget,
    /// Extra nonce 1 (from pool).
    pub extranonce1: Vec<u8>,
    /// Extra nonce 2 size.
    pub extranonce2_size: usize,
}

/// Pool connection state.
#[derive(Debug, Clone)]
pub enum PoolConnection {
    /// Not connected.
    Disconnected,
    /// Connecting to pool.
    Connecting { url: String },
    /// Connected and authenticated.
    Connected { url: String, worker: String },
    /// Connection error.
    Error { url: String, reason: String },
}

impl Default for PoolConnection {
    fn default() -> Self {
        Self::Disconnected
    }
}
