//! Mining Plugin type definitions.
//!
//! This module contains all type definitions for the Mining plugin:
//! - Mining statistics and job types
//! - Block header and hash target structures
//! - Pool connection state

mod core;

pub use core::{
    BlockHeader, HashTarget, MiningJob, MiningStats, Nonce, PoolConnection,
};
