//! Shared access to live chain snapshots cached in Redis.

mod redis_cache;

pub use redis_cache::{LiveChainCache, LiveChainCacheBuilder};
