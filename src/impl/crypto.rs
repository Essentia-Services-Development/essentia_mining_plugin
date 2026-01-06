//! Crypto re-exports from canonical `essentia_core_utils::crypto` module.
//!
//! CR-164: Hash Function Canonicalization
//! This module re-exports SHA-256 primitives from the canonical source,
//! eliminating code duplication while maintaining API compatibility.
//!
//! ## SSOP Compliance
//! Uses `essentia_core_utils::crypto` - zero external dependencies.
//!
//! ## Usage
//! ```rust,ignore
//! use essentia_mining_plugin::r#impl::{sha256, double_sha256, Sha256};
//!
//! let hash = sha256(b"data");
//! let bitcoin_hash = double_sha256(b"block_header");
//! ```

// Re-export canonical SHA-256 implementation
pub use essentia_core_utils::crypto::{Sha256, double_sha256, sha256, sha256_hex};
